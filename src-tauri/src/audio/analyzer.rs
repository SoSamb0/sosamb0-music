//! Spectrum, waveform and level analysis for the visualisers.
//!
//! The output thread copies what it hands to the device into a small ring (`Tap`), never
//! blocking. The engine thread reads it about 30 times a second, but only while a visualiser
//! is on screen, so the analysis costs nothing otherwise.

use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use serde::Serialize;

pub const FFT: usize = 2048;
pub const BANDS: usize = 48;
pub const WAVE: usize = 128;
const TAP_FRAMES: usize = 4096;

/// Recent stereo output, written by the output thread.
pub struct Tap {
    left: Vec<f32>,
    right: Vec<f32>,
    pos: usize,
}

impl Tap {
    pub fn new() -> Tap {
        Tap { left: vec![0.0; TAP_FRAMES], right: vec![0.0; TAP_FRAMES], pos: 0 }
    }

    /// Append interleaved stereo frames.
    pub fn push(&mut self, interleaved: &[f64]) {
        for f in interleaved.chunks_exact(2) {
            self.left[self.pos] = f[0] as f32;
            self.right[self.pos] = f[1] as f32;
            self.pos = (self.pos + 1) % TAP_FRAMES;
        }
    }

    /// Copy the newest `n` frames, oldest first.
    fn latest(&self, n: usize, left: &mut Vec<f32>, right: &mut Vec<f32>) {
        left.clear();
        right.clear();
        let start = (self.pos + TAP_FRAMES - n) % TAP_FRAMES;
        for k in 0..n {
            let i = (start + k) % TAP_FRAMES;
            left.push(self.left[i]);
            right.push(self.right[i]);
        }
    }

    pub fn clear(&mut self) {
        self.left.fill(0.0);
        self.right.fill(0.0);
    }
}

/// One frame for the visualisers.
#[derive(Clone, Debug, Serialize)]
pub struct VizFrame {
    /// Log-spaced spectrum, 0..=255.
    pub bands: Vec<u8>,
    /// Mono waveform, -127..=127.
    pub wave: Vec<i8>,
    /// Left and right level (RMS over ~50 ms), 0..1 on a dB scale.
    pub level: [f32; 2],
}

pub struct Analyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    spectrum: Vec<Complex<f32>>,
    left: Vec<f32>,
    right: Vec<f32>,
    /// Bin ranges per band, for the current sample rate.
    edges: Vec<(usize, usize, f32)>,
    rate: u32,
}

impl Analyzer {
    pub fn new() -> Analyzer {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(FFT);
        let window = (0..FFT).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (FFT - 1) as f32).cos()).collect();
        let input = fft.make_input_vec();
        let spectrum = fft.make_output_vec();
        Analyzer { fft, window, input, spectrum, left: Vec::new(), right: Vec::new(), edges: Vec::new(), rate: 0 }
    }

    fn plan_bands(&mut self, rate: u32) {
        self.rate = rate;
        self.edges.clear();
        let bin_hz = rate as f32 / FFT as f32;
        let lo = 30.0f32;
        let hi = (rate as f32 / 2.0).min(16_000.0);
        for b in 0..BANDS {
            let f0 = lo * (hi / lo).powf(b as f32 / BANDS as f32);
            let f1 = lo * (hi / lo).powf((b + 1) as f32 / BANDS as f32);
            let i0 = ((f0 / bin_hz) as usize).max(1);
            let i1 = ((f1 / bin_hz).ceil() as usize).max(i0 + 1).min(FFT / 2);
            // Music has less energy up high; tilt +3 dB per octave around 1 kHz so the
            // display is evenly busy instead of all bass.
            let centre = (f0 * f1).sqrt();
            let tilt = 3.0 * (centre / 1000.0).log2();
            self.edges.push((i0, i1, tilt));
        }
    }

    pub fn analyze(&mut self, tap: &Tap, rate: u32) -> VizFrame {
        if rate != self.rate {
            self.plan_bands(rate);
        }
        tap.latest(FFT, &mut self.left, &mut self.right);

        for i in 0..FFT {
            self.input[i] = (self.left[i] + self.right[i]) * 0.5 * self.window[i];
        }
        let _ = self.fft.process(&mut self.input, &mut self.spectrum);

        // A full-scale sine through a Hann window peaks at FFT/4 in magnitude.
        let norm = 4.0 / FFT as f32;
        let bands = self
            .edges
            .iter()
            .map(|&(i0, i1, tilt)| {
                let peak = self.spectrum[i0..i1].iter().map(|c| c.norm()).fold(0.0f32, f32::max) * norm;
                let db = 20.0 * peak.max(1e-9).log10() + tilt;
                (((db + 66.0) / 60.0).clamp(0.0, 1.0) * 255.0) as u8
            })
            .collect();

        let step = 1024 / WAVE;
        let start = FFT - 1024;
        let wave = (0..WAVE)
            .map(|k| {
                let i = start + k * step;
                (((self.left[i] + self.right[i]) * 0.5).clamp(-1.0, 1.0) * 127.0) as i8
            })
            .collect();

        let n = (rate as usize / 20).min(FFT);
        let level = [&self.left, &self.right].map(|ch| {
            let rms = (ch[FFT - n..].iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
            ((20.0 * rms.max(1e-9).log10() + 48.0) / 48.0).clamp(0.0, 1.0)
        });

        VizFrame { bands, wave, level }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tap_with(f: impl Fn(usize) -> (f64, f64)) -> Tap {
        let mut tap = Tap::new();
        let frames: Vec<f64> = (0..TAP_FRAMES).flat_map(|i| { let (l, r) = f(i); [l, r] }).collect();
        tap.push(&frames);
        tap
    }

    #[test]
    fn silence_is_flat_and_quiet() {
        let frame = Analyzer::new().analyze(&Tap::new(), 48_000);
        assert!(frame.bands.iter().all(|&b| b == 0));
        assert!(frame.wave.iter().all(|&w| w == 0));
        assert_eq!(frame.level, [0.0, 0.0]);
    }

    #[test]
    fn a_tone_lights_up_its_own_band() {
        let rate = 44_100.0;
        let tap = tap_with(|i| {
            let s = 0.5 * (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / rate).sin();
            (s, s)
        });
        let mut a = Analyzer::new();
        let frame = a.analyze(&tap, 44_100);
        let loudest = frame.bands.iter().enumerate().max_by_key(|(_, b)| **b).unwrap().0;
        let (i0, i1, _) = a.edges[loudest];
        let bin_hz = rate as f32 / FFT as f32;
        assert!(i0 as f32 * bin_hz <= 1100.0 && i1 as f32 * bin_hz >= 900.0, "loudest band {loudest} is not around 1 kHz");
        assert!(frame.bands[loudest] > 200);
        assert!(frame.bands[0] < 60, "a 1 kHz tone must not light up the bass");
        assert!(frame.level[0] > 0.7 && (frame.level[0] - frame.level[1]).abs() < 1e-6);
    }

    #[test]
    fn levels_follow_each_channel() {
        let tap = tap_with(|i| (if i % 2 == 0 { 0.8 } else { -0.8 }, 0.0));
        let frame = Analyzer::new().analyze(&tap, 48_000);
        assert!(frame.level[0] > 0.9 && frame.level[1] == 0.0);
    }
}
