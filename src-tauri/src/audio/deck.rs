//! A deck owns one track: its media source, demuxer, decoder and (if needed) resampler.
//! It produces interleaved stereo f64 at the device rate into `fifo`.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{Time, TimeBase};

use super::resampler::RateConverter;
use super::source::{self, Progress};
use super::{SourceInfo, TrackReq};

/// Bytes that must be downloaded ahead of the read position before decoding continues.
/// Keeps the decoder from blocking the engine thread on a slow network.
const MIN_AHEAD: u64 = 192 * 1024;

pub struct Deck {
    pub req: TrackReq,
    pub info: SourceInfo,
    pub progress: Option<Arc<Progress>>,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    time_base: Option<TimeBase>,
    conv: Option<RateConverter>,
    out_rate: u32,
    gain: f64,
    /// Decoded audio ready for the mixer: interleaved stereo at `out_rate`.
    pub fifo: Vec<f64>,
    pub eof: bool,
    pub error: Option<String>,
    scratch: Vec<f64>,
    stereo: Vec<f64>,
    /// Source frames still to drop so an accurate seek lands on the exact sample.
    skip_frames: u64,
    /// Source-time position of the decode head, in frames.
    pos_frames: u64,
}

fn is_lossless(codec: &str) -> bool {
    let c = codec.to_ascii_lowercase();
    c.starts_with("pcm") || matches!(c.as_str(), "flac" | "alac" | "wavpack" | "ape" | "tta")
}

impl Deck {
    /// Open a track. This performs network I/O and is meant to run off the engine thread.
    pub fn open(req: TrackReq, cache_dir: &Path, cache_limit: u64) -> Result<Deck, String> {
        let opened = source::open(&req, cache_dir, cache_limit)?;
        let mss = MediaSourceStream::new(opened.source, Default::default());

        let mut hint = Hint::new();
        if !req.suffix.is_empty() {
            hint.with_extension(&req.suffix);
        }
        let format = symphonia::default::get_probe()
            .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
            .map_err(|e| format!("unrecognised audio format: {e}"))?;

        let track = format.default_track(TrackType::Audio).ok_or("no audio track in file")?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or("audio parameters missing")?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(|e| format!("unsupported codec: {e}"))?;

        let rate = params.sample_rate.ok_or("sample rate missing")?;
        let channels = params.channels.as_ref().map(|c| c.count()).unwrap_or(2).max(1) as u16;
        let codec = decoder.codec_info().short_name.to_string();
        let lossless = is_lossless(&codec);
        let duration = match track.num_frames {
            Some(n) if n > 0 => Some(n as f64 / rate as f64),
            _ if req.duration > 0.0 => Some(req.duration),
            _ => None,
        };
        let info = SourceInfo {
            codec,
            rate,
            channels,
            // Lossy codecs decode to floating point: a bit depth would be meaningless.
            bits: if lossless { params.bits_per_sample } else { None },
            lossless,
            duration,
        };

        Ok(Deck {
            track_id: track.id,
            time_base: track.time_base,
            req,
            info,
            progress: opened.progress,
            format,
            decoder,
            conv: None,
            out_rate: rate,
            gain: 1.0,
            fifo: Vec::new(),
            eof: false,
            error: None,
            scratch: Vec::new(),
            stereo: Vec::new(),
            skip_frames: 0,
            pos_frames: 0,
        })
    }

    /// Set the device rate this deck must produce. Creates or drops the resampler as needed.
    pub fn set_output_rate(&mut self, out_rate: u32) -> Result<(), String> {
        if out_rate == self.out_rate && (self.conv.is_some() == (out_rate != self.info.rate)) {
            return Ok(());
        }
        self.out_rate = out_rate;
        self.fifo.clear();
        self.conv = if out_rate != self.info.rate {
            Some(RateConverter::new(self.info.rate, out_rate, 2)?)
        } else {
            None
        };
        Ok(())
    }

    pub fn set_gain(&mut self, gain: f64) {
        self.gain = gain;
    }

    pub fn gain(&self) -> f64 {
        self.gain
    }

    pub fn resampled(&self) -> bool {
        self.conv.is_some()
    }

    /// Source-time position of the decode head, in seconds.
    pub fn decoded_secs(&self) -> f64 {
        self.pos_frames as f64 / self.info.rate as f64
    }

    /// Seconds of audio not yet handed to the mixer.
    pub fn remaining_secs(&self) -> Option<f64> {
        let buffered = (self.fifo.len() / 2) as f64 / self.out_rate as f64;
        self.info.duration.map(|d| (d - self.decoded_secs()).max(0.0) + buffered)
    }

    /// False while the network has not delivered enough data to decode without blocking.
    pub fn ready(&self) -> bool {
        match &self.progress {
            None => true,
            Some(p) => {
                p.done.load(Ordering::Acquire) || p.ahead() >= MIN_AHEAD || p.error().is_some()
            }
        }
    }

    pub fn buffered_fraction(&self) -> f64 {
        self.progress.as_ref().map_or(1.0, |p| p.fraction())
    }

    /// Whether a seek to `secs` can be served from what is already downloaded.
    pub fn can_seek(&self, secs: f64) -> bool {
        let Some(p) = &self.progress else { return true };
        if p.done.load(Ordering::Acquire) || p.error().is_some() {
            return true;
        }
        let total = p.total.load(Ordering::Relaxed);
        match self.info.duration {
            Some(d) if d > 0.0 && total > 0 => {
                let need = (secs / d).clamp(0.0, 1.0) * total as f64 + 512.0 * 1024.0;
                p.downloaded.load(Ordering::Relaxed) as f64 >= need.min(total as f64)
            }
            _ => false,
        }
    }

    /// Decode until `fifo` holds at least `min_frames`, the stream ends, or the network
    /// runs dry. Returns false when no progress was possible.
    pub fn fill(&mut self, min_frames: usize) -> bool {
        let mut progressed = false;
        while self.fifo.len() / 2 < min_frames && !self.eof && self.ready() {
            self.decode_packet();
            progressed = true;
        }
        progressed
    }

    fn decode_packet(&mut self) {
        let packet = loop {
            match self.format.next_packet() {
                Ok(Some(p)) if p.track_id == self.track_id => break p,
                Ok(Some(_)) => continue,
                Ok(None) => return self.finish(None),
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return self.finish(None);
                }
                Err(SymError::ResetRequired) => return self.finish(None),
                Err(e) => return self.finish(Some(e.to_string())),
            }
        };

        let decoded = match self.decoder.decode(&packet) {
            Ok(buf) => buf,
            // A damaged packet: skip it, keep playing.
            Err(SymError::DecodeError(_)) | Err(SymError::IoError(_)) => return,
            Err(e) => return self.finish(Some(e.to_string())),
        };

        let channels = decoded.spec().channels().count().max(1);
        let frames = decoded.frames();
        if frames == 0 {
            return;
        }
        self.scratch.resize(frames * channels, 0.0);
        decoded.copy_to_slice_interleaved(&mut self.scratch);

        // Drop the frames between the seek point the container gave us and the one asked for.
        let drop = (self.skip_frames as usize).min(frames);
        self.skip_frames -= drop as u64;
        let src = &self.scratch[drop * channels..];
        self.pos_frames += (frames - drop) as u64;
        if src.is_empty() {
            return;
        }

        self.stereo.clear();
        to_stereo(src, channels, &mut self.stereo);
        if self.gain != 1.0 {
            let g = self.gain;
            self.stereo.iter_mut().for_each(|s| *s *= g);
        }
        match &mut self.conv {
            Some(conv) => conv.process(&self.stereo, &mut self.fifo),
            None => self.fifo.extend_from_slice(&self.stereo),
        }
    }

    fn finish(&mut self, error: Option<String>) {
        self.eof = true;
        self.error = error;
        if let Some(conv) = &mut self.conv {
            conv.finish(&mut self.fifo);
        }
    }

    /// Seek to `secs`. Returns the position actually reached, which matches the request to
    /// the sample for formats that support accurate seeking.
    pub fn seek(&mut self, secs: f64) -> Result<f64, String> {
        let secs = secs.max(0.0);
        let time = Time::try_from_secs_f64(secs).ok_or("invalid seek time")?;
        let seeked = self
            .format
            .seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(self.track_id) })
            .map_err(|e| format!("seek failed: {e}"))?;
        self.decoder.reset();
        self.fifo.clear();
        if let Some(conv) = &mut self.conv {
            conv.reset();
        }
        self.eof = false;
        self.error = None;

        let rate = self.info.rate as f64;
        let to_secs = |ts| self.time_base.and_then(|tb| tb.calc_time(ts)).map(|t| t.as_secs_f64());
        let (actual, required) = match (to_secs(seeked.actual_ts), to_secs(seeked.required_ts)) {
            (Some(a), Some(r)) => (a, r.max(a)),
            _ => (secs, secs),
        };
        self.skip_frames = ((required - actual) * rate).round() as u64;
        self.pos_frames = (required * rate).round() as u64;
        Ok(required)
    }
}

/// Fold any channel layout down to stereo. Mono and stereo pass through untouched.
fn to_stereo(src: &[f64], channels: usize, out: &mut Vec<f64>) {
    match channels {
        1 => out.extend(src.iter().flat_map(|&s| [s, s])),
        2 => out.extend_from_slice(src),
        _ => {
            // Standard order: FL FR FC LFE BL BR [SL SR]. LFE is left out, as in ITU downmix.
            const K: f64 = std::f64::consts::FRAC_1_SQRT_2;
            let norm = 1.0 / (1.0 + K + K * if channels > 6 { 2.0 } else { 1.0 });
            for f in src.chunks_exact(channels) {
                let c = f.get(2).copied().unwrap_or(0.0);
                let (bl, br) = (f.get(4).copied().unwrap_or(0.0), f.get(5).copied().unwrap_or(0.0));
                let (sl, sr) = (f.get(6).copied().unwrap_or(0.0), f.get(7).copied().unwrap_or(0.0));
                out.push((f[0] + K * c + K * bl + K * sl) * norm);
                out.push((f[1] + K * c + K * br + K * sr) * norm);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_and_mono_pass_through_exactly() {
        let mut out = Vec::new();
        to_stereo(&[0.25, -0.5, 0.125, 1.0], 2, &mut out);
        assert_eq!(out, vec![0.25, -0.5, 0.125, 1.0]);
        out.clear();
        to_stereo(&[0.25, -0.5], 1, &mut out);
        assert_eq!(out, vec![0.25, 0.25, -0.5, -0.5]);
    }

    use crate::audio::convert::{convert, GainRamp, SampleFmt};

    /// Write a minimal PCM WAV file and return its raw sample bytes.
    fn write_wav(path: &std::path::Path, rate: u32, bits: u16, frames: usize) -> Vec<u8> {
        let bytes = (bits / 8) as usize;
        let mut seed = 0x1234_5678u32;
        let mut data = Vec::with_capacity(frames * 2 * bytes);
        for _ in 0..frames * 2 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            data.extend_from_slice(&seed.to_le_bytes()[..bytes]);
        }
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&rate.to_le_bytes());
        wav.extend_from_slice(&(rate * 2 * bytes as u32).to_le_bytes());
        wav.extend_from_slice(&(2 * bytes as u16).to_le_bytes());
        wav.extend_from_slice(&bits.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        std::fs::write(path, wav).unwrap();
        data
    }

    fn open(path: &std::path::Path) -> Deck {
        let req = TrackReq {
            token: 1,
            url: path.to_string_lossy().into_owned(),
            cache_key: String::new(),
            suffix: "wav".into(),
            duration: 0.0,
            album_id: String::new(),
            replay_gain: Default::default(),
        };
        Deck::open(req, &std::env::temp_dir(), 0).unwrap()
    }

    fn device_bytes(samples: &[f64], fmt: SampleFmt) -> Vec<u8> {
        let mut out = vec![0u8; samples.len() * fmt.bytes()];
        let mut unity = GainRamp { current: 1.0, target: 1.0, step: 1.0 };
        convert(samples, &mut out, fmt, 2, &mut unity, None);
        out
    }

    /// The whole path from file to device bytes, minus the sound card itself:
    /// what goes to the DAC must be the bytes stored in the file.
    #[test]
    fn file_to_device_is_bit_perfect() {
        for (bits, fmt) in [(16u16, SampleFmt::I16), (24, SampleFmt::I24P3)] {
            let path = std::env::temp_dir().join(format!("lumen-bitperfect-{bits}.wav"));
            let original = write_wav(&path, 44_100, bits, 50_000);
            let mut deck = open(&path);
            assert_eq!(deck.info.bits, Some(bits as u32));
            assert!(deck.info.lossless && !deck.resampled());
            deck.fill(usize::MAX);
            assert!(deck.eof && deck.error.is_none());
            assert_eq!(deck.fifo.len(), 100_000);
            assert!(device_bytes(&deck.fifo, fmt) == original, "{bits} bit output differs from the file");
            drop(deck);
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn seek_lands_on_the_exact_sample() {
        let path = std::env::temp_dir().join("lumen-seek.wav");
        let original = write_wav(&path, 48_000, 16, 96_000);
        let mut deck = open(&path);
        let reached = deck.seek(0.75).unwrap();
        assert!((reached - 0.75).abs() < 1e-9);
        deck.fill(1024);
        let frame = 36_000;
        let got = device_bytes(&deck.fifo[..512], SampleFmt::I16);
        assert!(got[..] == original[frame * 4..frame * 4 + got.len()]);
        assert!((deck.remaining_secs().unwrap() - 1.25).abs() < 1e-6);
        drop(deck);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn surround_downmix_cannot_clip() {
        let mut out = Vec::new();
        to_stereo(&[1.0; 6], 6, &mut out);
        assert!(out.iter().all(|s| s.abs() <= 1.0 + 1e-12));
        out.clear();
        to_stereo(&[1.0; 8], 8, &mut out);
        assert!(out.iter().all(|s| s.abs() <= 1.0 + 1e-12));
    }
}
