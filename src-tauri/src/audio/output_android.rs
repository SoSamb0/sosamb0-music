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

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError};
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

/// Minimal safe wrapper over the AAudio C API (only what this backend needs).
mod aaudio {
    use std::ffi::{c_void, CStr};
    use std::ptr;

    use ndk_sys as ffi;

    /// An AAudio error code.
    #[derive(Debug, Clone, Copy)]
    pub struct Error(pub i32);

    impl Error {
        pub const TIMEOUT: Error = Error(ffi::AAUDIO_ERROR_TIMEOUT);

        pub fn text(self) -> String {
            let text = unsafe { ffi::AAudio_convertResultToText(self.0) };
            if text.is_null() {
                return format!("AAudio error {}", self.0);
            }
            unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
        }
    }

    fn check(result: i32) -> Result<i32, Error> {
        if result < 0 { Err(Error(result)) } else { Ok(result) }
    }

    pub struct Stream(*mut ffi::AAudioStream);

    // The stream is only ever used from the output thread that opened it.
    unsafe impl Send for Stream {}

    impl Stream {
        /// Shared-mode float output at the device's own rate, on the power-saving path.
        pub fn open_output(channels: i32) -> Result<Stream, Error> {
            unsafe {
                let mut builder = ptr::null_mut();
                check(ffi::AAudio_createStreamBuilder(&mut builder))?;
                ffi::AAudioStreamBuilder_setDirection(builder, ffi::AAUDIO_DIRECTION_OUTPUT as i32);
                ffi::AAudioStreamBuilder_setFormat(builder, ffi::AAUDIO_FORMAT_PCM_FLOAT as i32);
                ffi::AAudioStreamBuilder_setChannelCount(builder, channels);
                ffi::AAudioStreamBuilder_setSharingMode(builder, ffi::AAUDIO_SHARING_MODE_SHARED as i32);
                ffi::AAudioStreamBuilder_setPerformanceMode(builder, ffi::AAUDIO_PERFORMANCE_MODE_POWER_SAVING as i32);
                let mut stream = ptr::null_mut();
                let result = ffi::AAudioStreamBuilder_openStream(builder, &mut stream);
                ffi::AAudioStreamBuilder_delete(builder);
                check(result)?;
                Ok(Stream(stream))
            }
        }

        pub fn is_float(&self) -> bool {
            unsafe { ffi::AAudioStream_getFormat(self.0) == ffi::AAUDIO_FORMAT_PCM_FLOAT as i32 }
        }

        pub fn channel_count(&self) -> i32 {
            unsafe { ffi::AAudioStream_getChannelCount(self.0) }
        }

        pub fn sample_rate(&self) -> i32 {
            unsafe { ffi::AAudioStream_getSampleRate(self.0) }
        }

        pub fn frames_per_burst(&self) -> i32 {
            unsafe { ffi::AAudioStream_getFramesPerBurst(self.0) }
        }

        pub fn frames_written(&self) -> i64 {
            unsafe { ffi::AAudioStream_getFramesWritten(self.0) }
        }

        pub fn frames_read(&self) -> i64 {
            unsafe { ffi::AAudioStream_getFramesRead(self.0) }
        }

        pub fn start(&self) -> Result<(), Error> {
            check(unsafe { ffi::AAudioStream_requestStart(self.0) }).map(drop)
        }

        pub fn pause(&self) -> Result<(), Error> {
            check(unsafe { ffi::AAudioStream_requestPause(self.0) }).map(drop)
        }

        pub fn flush(&self) -> Result<(), Error> {
            check(unsafe { ffi::AAudioStream_requestFlush(self.0) }).map(drop)
        }

        /// Write interleaved f32 frames, blocking up to `timeout_ns`. Returns the frames written.
        pub fn write(&self, samples: &[f32], channels: usize, timeout_ns: i64) -> Result<usize, Error> {
            let frames = (samples.len() / channels) as i32;
            let n = check(unsafe {
                ffi::AAudioStream_write(self.0, samples.as_ptr() as *const c_void, frames, timeout_ns)
            })?;
            Ok(n as usize)
        }
    }

    impl Drop for Stream {
        fn drop(&mut self) {
            unsafe { ffi::AAudioStream_close(self.0) };
        }
    }
}

use aaudio::Error as AudioError;

struct Stream {
    stream: aaudio::Stream,
    rate: u32,
    /// Frames handed to `write` per call: a couple of bursts.
    chunk: usize,
    samples: Vec<f64>,
    bytes: Vec<u8>,
    /// `bytes` as the f32 samples the device takes.
    out: Vec<f32>,
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
                    eprintln!("lumen: output device failed: {}", e.text());
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
                    if s.stream.start().is_ok() {
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
            let _ = s.stream.pause();
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
            let _ = s.stream.flush();
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

            s.out.clear();
            s.out.extend(s.bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])));
            let mut done = 0;
            while done < frames {
                // Blocks until there is room; the timeout only guards against a stuck device.
                let n = s.stream.write(&s.out[done * CHANNELS..], CHANNELS, 1_000_000_000)?;
                if n == 0 {
                    return Err(AudioError::TIMEOUT);
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
    let stream = aaudio::Stream::open_output(CHANNELS as i32).map_err(|e| format!("AAudio: {}", e.text()))?;
    if !stream.is_float() || stream.channel_count() != CHANNELS as i32 {
        return Err(format!("AAudio opened {} channels, float: {}", stream.channel_count(), stream.is_float()));
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
    let stream = Stream { stream, rate, chunk: burst * 2, samples: Vec::new(), bytes: Vec::new(), out: Vec::new() };
    Ok((stream, format))
}
