//! AAudio output backend for Android.
//!
//! Quality choices:
//! - 32-bit float samples. Android's mixer works in float, so a 24-bit source reaches it without
//!   truncation and without dither.
//! - The stream runs at the device's native rate. Asking for the track's rate would make Android
//!   resample inside its mixer with whatever quality the phone ships; leaving the rate to the
//!   engine means our own resampler is used when the rates differ, and nothing at all when they
//!   match.
//! - Usage "media" / content "music" (AAudio's defaults), power-saving mode (large buffer, the CPU sleeps between
//!   writes). Latency does not matter for music; the position is corrected for it below.
//!
//! Everything still passes through the Android mixer, so this path is never bit-perfect; the
//! engine reports it as shared (`exclusive: false`).
//!
//! The thread mirrors `output.rs`: blocking writes from the ring, the same fades for pause and
//! flush, and the same `OutShared` reporting.

use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use ndk::audio::{
    AudioDirection, AudioError, AudioFormat, AudioPerformanceMode, AudioSharingMode,
    AudioStream, AudioStreamBuilder,
};
use rtrb::Consumer;

use super::backend::{OutCmd, OutShared, CHANNELS};
use super::convert::{convert, GainRamp, SampleFmt};
use super::{DeviceFormat, DeviceInfo};

/// Length of the fade used for pause, seek and volume changes.
const FADE_SECS: f64 = 0.012;
/// How long a pause may wait for the device to play out the fade already in its buffer.
const DRAIN_LIMIT: Duration = Duration::from_millis(600);
/// Device name shown in the signal path window.
const DEVICE_NAME: &str = "Android";

struct Stream {
    stream: AudioStream,
    rate: u32,
    /// Frames handed to `write` per call: a couple of bursts.
    chunk: usize,
    samples: Vec<f64>,
    bytes: Vec<u8>,
}

impl Stream {
    /// Frames written to the device but not yet played.
    fn in_device(&self) -> u64 {
        (self.stream.frames_written() - self.stream.frames_read()).max(0) as u64
    }
}

enum Pending {
    Pause { reply: Sender<()>, since: Option<Instant> },
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
    /// Real (non-silence) frames written since the last flush.
    written: u64,
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
            written: 0,
        }
    }

    fn run(mut self) {
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
                    eprintln!("lumen: output device failed: {}", e.to_text());
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
            OutCmd::Open { reply, .. } => {
                self.settle_pending();
                self.stop_stream();
                self.stream = None;
                let answer = open().map(|(stream, format)| {
                    self.gain.step = 1.0 / (FADE_SECS * stream.rate as f64);
                    self.gain.current = self.shared.volume();
                    self.stream = Some(stream);
                    format
                });
                let _ = reply.send(answer);
            }
            OutCmd::Start => {
                if let (Some(s), false) = (&self.stream, self.running) {
                    if s.stream.request_start().is_ok() {
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
                    self.pending = Some(Pending::Pause { reply, since: None });
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
                // Android routes output itself (speaker, wired, Bluetooth, USB) and the route
                // follows what is plugged in, so there is only the system default.
                let _ = reply.send(vec![DeviceInfo {
                    id: String::new(),
                    name: "Системный вывод".into(),
                    is_default: true,
                }]);
            }
        }
    }

    fn stop_stream(&mut self) {
        if let (Some(s), true) = (&self.stream, self.running) {
            let _ = s.stream.request_pause();
        }
        self.running = false;
    }

    /// Drop everything queued in the ring and restart the play counter.
    fn discard(&mut self, fade_in: bool) {
        let n = self.consumer.slots();
        if let Ok(chunk) = self.consumer.read_chunk(n) {
            chunk.commit_all();
        }
        // A paused stream can drop its buffer too (only silence from the fade is left in it),
        // so the next sound starts immediately and the play counter stays exact.
        if let (Some(s), false) = (&self.stream, self.running) {
            let _ = s.stream.request_flush();
        }
        self.written = 0;
        self.shared.frames_played.store(0, Ordering::Release);
        self.shared.tap.lock().clear();
        self.fade_out = false;
        self.gain.current = if fade_in { 0.0 } else { self.shared.volume() };
    }

    /// Complete a pause or flush that was waiting for its fade-out.
    fn settle_pending(&mut self) {
        match self.pending.take() {
            Some(Pending::Pause { reply, .. }) => {
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

    /// Report how much of the real audio the device has actually played. The buffer is large
    /// in power-saving mode, so counting written frames would run the clock (and lyrics) ahead.
    fn update_played(&self) {
        if let Some(s) = &self.stream {
            let played = self.written.saturating_sub(s.in_device());
            self.shared.frames_played.fetch_max(played, Ordering::AcqRel);
        }
    }

    /// Move one chunk from the ring to the device, blocking until it fits.
    fn render(&mut self) -> Result<(), AudioError> {
        let Some(s) = &mut self.stream else {
            self.running = false;
            return Ok(());
        };
        let queued = self.consumer.slots() / CHANNELS;
        let frames = queued.min(s.chunk);
        self.gain.target = if self.fade_out { 0.0 } else { self.shared.volume() };

        if frames > 0 {
            s.samples.clear();
            if let Ok(chunk) = self.consumer.read_chunk(frames * CHANNELS) {
                let (a, b) = chunk.as_slices();
                s.samples.extend_from_slice(a);
                s.samples.extend_from_slice(b);
                chunk.commit_all();
            }
            // Never wait for the visualiser: skip this chunk if the engine is reading.
            if let Some(mut tap) = self.shared.tap.try_lock() {
                tap.push(&s.samples);
            }
            s.bytes.resize(frames * CHANNELS * SampleFmt::F32.bytes(), 0);
            // Float output is never dithered; `convert` skips the ramp at unity gain.
            convert(&s.samples, &mut s.bytes, SampleFmt::F32, CHANNELS, &mut self.gain, None);

            let mut done = 0;
            while done < frames {
                let offset = done * CHANNELS * SampleFmt::F32.bytes();
                // Blocks until there is room; the timeout only guards against a stuck device.
                let n = unsafe {
                    s.stream.write(
                        s.bytes[offset..].as_ptr() as *const c_void,
                        (frames - done) as i32,
                        1_000_000_000,
                    )
                }? as usize;
                if n == 0 {
                    return Err(AudioError::Timeout);
                }
                done += n;
            }
            self.written += frames as u64;
        } else {
            if self.fade_out {
                // Nothing is playing, so there is nothing to fade.
                self.gain.current = 0.0;
            }
            // Ring is empty: let the device play what it has instead of spinning.
            std::thread::sleep(Duration::from_millis(5));
        }
        self.update_played();

        if self.fade_out && self.gain.current <= 0.0 {
            let drained = self.stream.as_ref().is_none_or(|s| s.in_device() == 0);
            match &mut self.pending {
                // Let the fade itself be heard before the stream stops, or the pause would cut
                // the music mid-buffer and the rest of it would play after resuming.
                Some(Pending::Pause { since, .. }) if !drained => {
                    let started = *since.get_or_insert_with(Instant::now);
                    if started.elapsed() < DRAIN_LIMIT {
                        std::thread::sleep(Duration::from_millis(5));
                        return Ok(());
                    }
                    self.settle_pending();
                }
                _ => self.settle_pending(),
            }
            self.fade_out = false;
        }
        Ok(())
    }
}

fn open() -> Result<(Stream, DeviceFormat), String> {
    let err = |e: AudioError| format!("AAudio: {}", e.to_text());
    let stream = AudioStreamBuilder::new()
        .map_err(err)?
        .direction(AudioDirection::Output)
        .format(AudioFormat::PCM_Float)
        .channel_count(CHANNELS as i32)
        .sharing_mode(AudioSharingMode::Shared)
        .performance_mode(AudioPerformanceMode::PowerSaving)
        .open_stream()
        .map_err(err)?;
    if stream.format() != AudioFormat::PCM_Float || stream.channel_count() != CHANNELS as i32 {
        return Err(format!(
            "AAudio opened {:?} with {} channels instead of float stereo",
            stream.format(),
            stream.channel_count()
        ));
    }
    let rate = stream.sample_rate() as u32;
    let burst = stream.frames_per_burst().max(64) as usize;
    let format = DeviceFormat {
        device_name: DEVICE_NAME.into(),
        rate,
        channels: CHANNELS as u16,
        bits: 32,
        float: true,
        exclusive: false,
        note: None,
    };
    let stream = Stream { stream, rate, chunk: burst * 2, samples: Vec::new(), bytes: Vec::new() };
    Ok((stream, format))
}
