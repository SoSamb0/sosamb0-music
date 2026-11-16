//! WASAPI output backend (shared and exclusive mode, event driven).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use rtrb::Consumer;
use wasapi::{
    calculate_period_100ns, AudioClient, AudioRenderClient, Device, DeviceEnumerator, Direction,
    Handle, SampleType, StreamMode, WasapiError, WaveFormat,
};

use super::backend::{OutCmd, OutShared, CHANNELS};
use super::convert::{convert, GainRamp, Rng, SampleFmt};
use super::{DeviceFormat, DeviceInfo};

/// Length of the fade used for pause, seek and volume changes.
const FADE_SECS: f64 = 0.012;
/// AUDCLNT_E_BUFFER_SIZE_NOT_ALIGNED
const E_NOT_ALIGNED: i32 = 0x8889_0019u32 as i32;

struct Stream {
    client: AudioClient,
    render: AudioRenderClient,
    event: Handle,
    fmt: SampleFmt,
    exclusive: bool,
    rate: u32,
    samples: Vec<f64>,
    bytes: Vec<u8>,
}

enum Pending {
    Pause(Sender<()>),
    Flush { fade_in: bool, reply: Sender<()> },
}

pub fn spawn(consumer: Consumer<f64>, shared: Arc<OutShared>) -> Sender<OutCmd> {
    let (tx, rx) = crossbeam_channel::unbounded();
    std::thread::Builder::new()
        .name("lumen-output".into())
        .spawn(move || Output::new(consumer, shared, rx).run())
        .expect("spawn output thread");
    tx
}

struct Output {
    consumer: Consumer<f64>,
    shared: Arc<OutShared>,
    rx: Receiver<OutCmd>,
    stream: Option<Stream>,
    running: bool,
    fade_out: bool,
    pending: Option<Pending>,
    gain: GainRamp,
    rng: Rng,
}

impl Output {
    fn new(consumer: Consumer<f64>, shared: Arc<OutShared>, rx: Receiver<OutCmd>) -> Output {
        Output {
            consumer,
            shared,
            rx,
            stream: None,
            running: false,
            fade_out: false,
            pending: None,
            gain: GainRamp { current: 1.0, target: 1.0, step: 1.0 },
            rng: Rng::new(),
        }
    }

    fn run(mut self) {
        let _ = wasapi::initialize_mta();
        raise_thread_priority();
        loop {
            let cmd = if self.running {
                match self.rx.try_recv() {
                    Ok(c) => Some(c),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => return,
                }
            } else {
                match self.rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(c) => Some(c),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            };
            if let Some(cmd) = cmd {
                self.handle(cmd);
                continue;
            }
            if self.running {
                if let Err(e) = self.render() {
                    eprintln!("lumen: output device failed: {e}");
                    self.stop_stream();
                    self.stream = None;
                    self.settle_pending();
                    self.shared.failed.store(true, Ordering::Release);
                }
            }
        }
    }

    fn handle(&mut self, cmd: OutCmd) {
        match cmd {
            OutCmd::Open { device_id, exclusive, rate, reply } => {
                self.settle_pending();
                self.stop_stream();
                // Release the old device before claiming the new one: in exclusive mode the
                // same endpoint cannot be opened twice.
                self.stream = None;
                let result = open(device_id.as_deref(), exclusive, rate);
                let answer = match result {
                    Ok((stream, format)) => {
                        self.gain.step = 1.0 / (FADE_SECS * stream.rate as f64);
                        self.gain.current = self.shared.volume();
                        self.stream = Some(stream);
                        Ok(format)
                    }
                    Err(e) => Err(e),
                };
                let _ = reply.send(answer);
            }
            OutCmd::Start => {
                if let (Some(s), false) = (&self.stream, self.running) {
                    if s.client.start_stream().is_ok() {
                        self.running = true;
                    } else {
                        self.shared.failed.store(true, Ordering::Release);
                    }
                }
                self.fade_out = false;
            }
            OutCmd::Pause(reply) => {
                self.settle_pending();
                if self.running {
                    self.fade_out = true;
                    self.pending = Some(Pending::Pause(reply));
                } else {
                    let _ = reply.send(());
                }
            }
            OutCmd::Flush { fade_in, reply } => {
                self.settle_pending();
                if self.running && self.gain.current > 0.0 {
                    self.fade_out = true;
                    self.pending = Some(Pending::Flush { fade_in, reply });
                } else {
                    self.discard(fade_in);
                    let _ = reply.send(());
                }
            }
            OutCmd::Close => {
                self.settle_pending();
                self.stop_stream();
                self.stream = None;
            }
            OutCmd::ListDevices(reply) => {
                let _ = reply.send(list_devices().unwrap_or_default());
            }
        }
    }

    fn stop_stream(&mut self) {
        if let (Some(s), true) = (&self.stream, self.running) {
            let _ = s.client.stop_stream();
        }
        self.running = false;
    }

    /// Drop everything queued in the ring and restart the play counter.
    fn discard(&mut self, fade_in: bool) {
        let n = self.consumer.slots();
        if let Ok(chunk) = self.consumer.read_chunk(n) {
            chunk.commit_all();
        }
        self.shared.frames_played.store(0, Ordering::Release);
        self.fade_out = false;
        self.gain.current = if fade_in { 0.0 } else { self.shared.volume() };
    }

    /// Complete a pause or flush that was waiting for its fade-out.
    fn settle_pending(&mut self) {
        match self.pending.take() {
            Some(Pending::Pause(reply)) => {
                self.stop_stream();
                self.fade_out = false;
                let _ = reply.send(());
            }
            Some(Pending::Flush { fade_in, reply }) => {
                self.discard(fade_in);
                let _ = reply.send(());
            }
            None => {}
        }
    }

    /// Wait for the device and hand it one period of audio.
    fn render(&mut self) -> Result<(), WasapiError> {
        let Some(s) = &mut self.stream else {
            self.running = false;
            return Ok(());
        };
        s.event.wait_for_event(1000)?;

        let space = s.client.get_available_space_in_frames()? as usize;
        let queued = self.consumer.slots() / CHANNELS;
        // Exclusive mode must be fed a full buffer every period, so an underrun is padded
        // with silence. Shared mode simply writes what is available.
        let real = space.min(queued);
        let frames = if s.exclusive { space } else { real };

        self.gain.target = if self.fade_out { 0.0 } else { self.shared.volume() };
        if frames > 0 {
            s.samples.clear();
            if real > 0 {
                if let Ok(chunk) = self.consumer.read_chunk(real * CHANNELS) {
                    let (a, b) = chunk.as_slices();
                    s.samples.extend_from_slice(a);
                    s.samples.extend_from_slice(b);
                    chunk.commit_all();
                }
            }
            s.samples.resize(frames * CHANNELS, 0.0);
            s.bytes.resize(frames * CHANNELS * s.fmt.bytes(), 0);

            let dither = !s.fmt.is_float()
                && (self.shared.dither.load(Ordering::Relaxed) || !self.gain.is_unity());
            convert(
                &s.samples,
                &mut s.bytes,
                s.fmt,
                CHANNELS,
                &mut self.gain,
                if dither { Some(&mut self.rng) } else { None },
            );
            s.render.write_to_device(&s.bytes, None)?;
            self.shared.frames_played.fetch_add(real as u64, Ordering::AcqRel);
        } else if self.fade_out {
            // Nothing is playing, so there is nothing to fade.
            self.gain.current = 0.0;
        }

        if self.fade_out && self.gain.current <= 0.0 {
            self.settle_pending();
            self.fade_out = false;
        }
        Ok(())
    }
}

fn err(e: WasapiError) -> String {
    e.to_string()
}

fn find_device(enumerator: &DeviceEnumerator, id: Option<&str>) -> Result<Device, String> {
    if let Some(id) = id {
        if let Ok(d) = enumerator.get_device(id) {
            return Ok(d);
        }
        // The chosen device is gone (unplugged): fall back to the system default.
    }
    enumerator.get_default_device(Direction::Render).map_err(err)
}

fn open(id: Option<&str>, exclusive: bool, rate: u32) -> Result<(Stream, DeviceFormat), String> {
    let enumerator = DeviceEnumerator::new().map_err(err)?;
    let device = find_device(&enumerator, id)?;
    let name = device.get_friendlyname().unwrap_or_else(|_| "Audio device".into());

    let mut note = None;
    if exclusive {
        match open_exclusive(&device, rate) {
            Ok((client, wave, fmt, actual_rate)) => {
                let format = DeviceFormat {
                    device_name: name,
                    rate: actual_rate,
                    channels: CHANNELS as u16,
                    bits: fmt.bits(),
                    float: false,
                    exclusive: true,
                    note: (actual_rate != rate)
                        .then(|| format!("device does not support {rate} Hz, using {actual_rate} Hz")),
                };
                return finish(client, wave, fmt, true, format);
            }
            Err(e) => note = Some(format!("exclusive mode unavailable ({e}), using shared mode")),
        }
    }

    let mut client = device.get_iaudioclient().map_err(err)?;
    let mix_rate = client.get_mixformat().map_err(err)?.get_samplespersec();
    let wave = WaveFormat::new(32, 32, SampleType::Float, mix_rate as usize, CHANNELS, None);
    let (period, _) = client.get_device_period().map_err(err)?;
    let mode = StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: period * 3 };
    client.initialize_client(&wave, Direction::Render, mode, None).map_err(err)?;
    let format = DeviceFormat {
        device_name: name,
        rate: mix_rate,
        channels: CHANNELS as u16,
        bits: 32,
        float: true,
        exclusive: false,
        note,
    };
    finish(client, wave, SampleFmt::F32, false, format)
}

fn finish(
    client: AudioClient,
    wave: WaveFormat,
    fmt: SampleFmt,
    exclusive: bool,
    format: DeviceFormat,
) -> Result<(Stream, DeviceFormat), String> {
    let event = client.set_get_eventhandle().map_err(err)?;
    let render = client.get_audiorenderclient().map_err(err)?;
    let stream = Stream {
        client,
        render,
        event,
        fmt,
        exclusive,
        rate: wave.get_samplespersec(),
        samples: Vec::new(),
        bytes: Vec::new(),
    };
    Ok((stream, format))
}

/// Rates to try, best first, when the device rejects the source rate in exclusive mode.
/// Integer multiples come first because they resample most cleanly.
fn fallback_rates(rate: u32) -> Vec<u32> {
    let mut rates = vec![rate, rate * 2, rate * 4];
    if rate % 2 == 0 {
        rates.push(rate / 2);
    }
    let family: [u32; 4] = if rate % 44_100 == 0 {
        [176_400, 88_200, 44_100, 48_000]
    } else {
        [192_000, 96_000, 48_000, 44_100]
    };
    rates.extend(family);
    rates.retain(|&r| (8_000..=768_000).contains(&r));
    let mut seen = Vec::new();
    rates.retain(|r| !seen.contains(r) && { seen.push(*r); true });
    rates
}

fn open_exclusive(
    device: &Device,
    rate: u32,
) -> Result<(AudioClient, WaveFormat, SampleFmt, u32), String> {
    // Widest container first: any integer source then fits without losing bits.
    const FORMATS: [(usize, usize, SampleFmt); 4] = [
        (32, 32, SampleFmt::I32),
        (32, 24, SampleFmt::I24In32),
        (24, 24, SampleFmt::I24P3),
        (16, 16, SampleFmt::I16),
    ];
    let mut last = String::from("no supported format");
    for try_rate in fallback_rates(rate) {
        for (store, valid, fmt) in FORMATS {
            let mut client = device.get_iaudioclient().map_err(err)?;
            let want = WaveFormat::new(store, valid, SampleType::Int, try_rate as usize, CHANNELS, None);
            let Ok(wave) = client.is_supported_exclusive_with_quirks(&want) else { continue };
            let (period, _) = client.get_device_period().map_err(err)?;
            let period = client
                .calculate_aligned_period_near(period, Some(128), &wave)
                .unwrap_or(period);
            let mode = StreamMode::EventsExclusive { period_hns: period };
            match client.initialize_client(&wave, Direction::Render, mode, None) {
                Ok(()) => return Ok((client, wave, fmt, try_rate)),
                Err(WasapiError::Windows(e)) if e.code().0 == E_NOT_ALIGNED => {
                    // The driver wants a specific buffer size; ask for it and retry once.
                    let frames = client.get_buffer_size().map_err(err)?;
                    let period = calculate_period_100ns(frames as i64, try_rate as i64);
                    let mut client = device.get_iaudioclient().map_err(err)?;
                    let mode = StreamMode::EventsExclusive { period_hns: period };
                    match client.initialize_client(&wave, Direction::Render, mode, None) {
                        Ok(()) => return Ok((client, wave, fmt, try_rate)),
                        Err(e) => last = e.to_string(),
                    }
                }
                Err(e) => last = e.to_string(),
            }
        }
    }
    Err(last)
}

fn list_devices() -> Result<Vec<DeviceInfo>, String> {
    let enumerator = DeviceEnumerator::new().map_err(err)?;
    let default_id = enumerator
        .get_default_device(Direction::Render)
        .ok()
        .and_then(|d| d.get_id().ok());
    let collection = enumerator.get_device_collection(Direction::Render).map_err(err)?;
    let mut out = Vec::new();
    for device in &collection {
        let Ok(device) = device else { continue };
        let (Ok(id), Ok(name)) = (device.get_id(), device.get_friendlyname()) else { continue };
        out.push(DeviceInfo { is_default: Some(&id) == default_id.as_ref(), id, name });
    }
    Ok(out)
}

/// Register the thread with the Windows multimedia scheduler so audio keeps flowing
/// while the rest of the system is busy.
fn raise_thread_priority() {
    use windows::core::w;
    use windows::Win32::System::Threading::AvSetMmThreadCharacteristicsW;
    let mut task_index = 0u32;
    // The returned handle is intentionally kept for the life of the thread.
    let _ = unsafe { AvSetMmThreadCharacteristicsW(w!("Pro Audio"), &mut task_index) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_rates_start_with_the_source_rate_and_have_no_duplicates() {
        for rate in [44_100, 48_000, 88_200, 96_000, 192_000, 352_800] {
            let rates = fallback_rates(rate);
            assert_eq!(rates[0], rate);
            let mut sorted = rates.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), rates.len());
        }
        assert_eq!(fallback_rates(44_100)[1], 88_200);
    }
}
