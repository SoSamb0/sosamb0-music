//! Platform-neutral contract between the engine and an output backend.
//!
//! A backend owns the consumer end of the sample ring and a device. It is driven by
//! `OutCmd` messages and reports through `OutShared`. Windows implements it with WASAPI
//! (`output.rs`); an Android backend only has to honour the same contract.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crossbeam_channel::Sender;

use super::{DeviceFormat, DeviceInfo};

/// The ring always carries interleaved stereo.
pub const CHANNELS: usize = 2;

pub struct OutShared {
    /// Frames of real audio (not underrun silence) handed to the device since the last flush.
    pub frames_played: AtomicU64,
    /// Linear volume as f64 bits. Exactly 1.0 bypasses the gain stage.
    volume: AtomicU64,
    /// Engine's request to dither integer output (set whenever the signal was processed).
    pub dither: AtomicBool,
    /// Raised by the backend when the device stopped working; the engine reopens it.
    pub failed: AtomicBool,
    /// What was last sent to the device, for the visualisers. Only ever try-locked by the output thread.
    pub tap: parking_lot::Mutex<super::analyzer::Tap>,
}

impl OutShared {
    pub fn new() -> OutShared {
        OutShared {
            frames_played: AtomicU64::new(0),
            volume: AtomicU64::new(1.0f64.to_bits()),
            dither: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            tap: parking_lot::Mutex::new(super::analyzer::Tap::new()),
        }
    }

    pub fn volume(&self) -> f64 {
        f64::from_bits(self.volume.load(Ordering::Relaxed))
    }

    pub fn set_volume(&self, v: f64) {
        self.volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

pub enum OutCmd {
    /// Open a device. `rate` is the wanted rate in exclusive mode; shared mode uses the
    /// system mixer rate. Any previously open device is released first.
    Open {
        device_id: Option<String>,
        exclusive: bool,
        rate: u32,
        reply: Sender<Result<DeviceFormat, String>>,
    },
    Start,
    /// Fade out, stop the stream, then reply.
    Pause(Sender<()>),
    /// Fade out, discard everything queued, reset `frames_played`, then reply.
    Flush { fade_in: bool, reply: Sender<()> },
    /// Release the device so other applications can use it.
    Close,
    ListDevices(Sender<Vec<DeviceInfo>>),
}
