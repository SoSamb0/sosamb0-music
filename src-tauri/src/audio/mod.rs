//! SoSamb0 Music audio engine.
//!
//! Signal flow:
//!
//! ```text
//! HTTP/cache -> symphonia decode -> f64 -> [ReplayGain] -> [resample] -> [crossfade mix]
//!            -> [DSP chain] -> lock-free ring -> output thread -> [volume/dither] -> WASAPI
//! ```
//!
//! Every stage in brackets is skipped entirely when it is not needed. Integer PCM converted to
//! f64 and back is exact (f64 has a 53 bit mantissa), so with all optional stages idle and an
//! exclusive-mode device running at the source rate, the samples reaching the DAC are
//! bit-identical to the decoded file. See `convert::tests` for the proof.

pub mod backend;
pub mod convert;
pub mod deck;
pub mod dsp;
pub mod engine;
#[cfg(windows)]
pub mod output;
pub mod prefetch;
pub mod resampler;
pub mod source;

use serde::{Deserialize, Serialize};

pub use dsp::DspConfig;
pub use engine::{Command, Engine, EngineHandle};

/// A request to play one track. `token` identifies the queue entry on the UI side.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackReq {
    pub token: u64,
    /// `http(s)://` stream URL, or a local file path.
    pub url: String,
    /// Stable identifier used for the on-disk cache. Empty disables caching.
    #[serde(default)]
    pub cache_key: String,
    /// File extension hint ("flac", "mp3", ...).
    #[serde(default)]
    pub suffix: String,
    /// Duration reported by the server, used when the container does not carry one.
    #[serde(default)]
    pub duration: f64,
    #[serde(default)]
    pub album_id: String,
    #[serde(default)]
    pub replay_gain: ReplayGainInfo,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReplayGainInfo {
    pub track_gain: Option<f64>,
    pub album_gain: Option<f64>,
    pub track_peak: Option<f64>,
    pub album_peak: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplayGainMode {
    #[default]
    Off,
    Track,
    Album,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    pub crossfade_secs: f64,
    pub replay_gain: ReplayGainMode,
    pub replay_gain_preamp_db: f64,
    pub prevent_clipping: bool,
    /// Maximum size of the on-disk stream cache, in megabytes.
    pub cache_limit_mb: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            crossfade_secs: 0.0,
            replay_gain: ReplayGainMode::Off,
            replay_gain_preamp_db: 0.0,
            prevent_clipping: true,
            cache_limit_mb: 2048,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OutputConfig {
    /// `None` follows the system default device.
    pub device_id: Option<String>,
    pub exclusive: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// What was actually opened on the output device.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFormat {
    pub device_name: String,
    pub rate: u32,
    pub channels: u16,
    /// Significant bits per sample.
    pub bits: u16,
    pub float: bool,
    pub exclusive: bool,
    /// Set when the request could not be honoured as asked (e.g. exclusive mode refused).
    pub note: Option<String>,
}

/// Properties of the decoded source stream.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub codec: String,
    pub rate: u32,
    pub channels: u16,
    pub bits: Option<u32>,
    pub lossless: bool,
    pub duration: Option<f64>,
}

/// Everything that happens to the signal between the file and the DAC.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalPath {
    pub source: SourceInfo,
    pub device: DeviceFormat,
    pub resampled: bool,
    pub replay_gain_db: f64,
    pub dsp_active: bool,
    pub volume_unity: bool,
    pub dithered: bool,
    /// True when the device receives exactly the samples stored in the file.
    pub bit_perfect: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayState {
    Stopped,
    Loading,
    Buffering,
    Playing,
    Paused,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EngineEvent {
    #[serde(rename_all = "camelCase")]
    Status {
        state: PlayState,
        token: Option<u64>,
        position: f64,
        duration: f64,
        /// Fraction of the current file that is downloaded, 0..1.
        buffered: f64,
    },
    #[serde(rename_all = "camelCase")]
    TrackStarted { token: u64 },
    /// The queue ran out: the last track finished and no next track was set.
    Ended,
    #[serde(rename_all = "camelCase")]
    Error { token: Option<u64>, message: String },
    #[serde(rename_all = "camelCase")]
    Signal { path: Option<SignalPath> },
}
