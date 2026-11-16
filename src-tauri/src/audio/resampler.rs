//! Sample rate conversion, used only when the device cannot run at the source rate
//! (always the case in shared mode, where Windows fixes the mixer rate).

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

const CHUNK: usize = 2048;

/// Streaming wrapper around rubato's synchronous FFT resampler.
///
/// It hides the fixed chunk size and removes the resampler's startup delay, so the output is
/// sample-aligned with the input: `n` input frames produce `ceil(n * ratio)` output frames
/// after `finish`. That alignment is what keeps gapless transitions gapless when resampling.
pub struct RateConverter {
    inner: Fft<f64>,
    channels: usize,
    from: u32,
    to: u32,
    pending: Vec<f64>,
    scratch: Vec<f64>,
    /// Startup-delay frames still to discard.
    skip: usize,
    frames_in: u64,
    frames_out: u64,
    /// Total output frames allowed, set once the input is known to be complete.
    limit: Option<u64>,
}

impl RateConverter {
    pub fn new(from: u32, to: u32, channels: usize) -> Result<RateConverter, String> {
        let inner = Fft::<f64>::new(from as usize, to as usize, CHUNK, channels, FixedSync::Input)
            .map_err(|e| format!("resampler {from} -> {to} Hz: {e}"))?;
        let skip = inner.output_delay();
        let scratch = vec![0.0; inner.output_frames_max() * channels];
        Ok(RateConverter {
            inner,
            channels,
            from,
            to,
            pending: Vec::with_capacity(CHUNK * channels * 2),
            scratch,
            skip,
            frames_in: 0,
            frames_out: 0,
            limit: None,
        })
    }

    /// Feed interleaved input; converted frames are appended to `out`.
    pub fn process(&mut self, input: &[f64], out: &mut Vec<f64>) {
        self.frames_in += (input.len() / self.channels) as u64;
        self.pending.extend_from_slice(input);
        while self.pending.len() / self.channels >= self.inner.input_frames_next() {
            if !self.run_chunk(out) {
                break;
            }
        }
    }

    /// Flush everything still inside the resampler. Call once at end of stream.
    pub fn finish(&mut self, out: &mut Vec<f64>) {
        // Integer maths: a float ratio would round 44100 -> 48000 up to 48001 frames.
        let total = (self.frames_in as u128 * self.to as u128).div_ceil(self.from as u128) as u64;
        self.limit = Some(total);
        // Push silence through until the delayed tail has come out.
        let mut guard = 0;
        while self.frames_out < total && guard < 64 {
            let need = self.inner.input_frames_next() * self.channels;
            if self.pending.len() < need {
                self.pending.resize(need, 0.0);
            }
            if !self.run_chunk(out) {
                break;
            }
            guard += 1;
        }
        self.pending.clear();
    }

    /// Forget all history (after a seek).
    pub fn reset(&mut self) {
        self.inner.reset();
        self.pending.clear();
        self.skip = self.inner.output_delay();
        self.frames_in = 0;
        self.frames_out = 0;
        self.limit = None;
    }

    fn run_chunk(&mut self, out: &mut Vec<f64>) -> bool {
        let ch = self.channels;
        let need = self.inner.input_frames_next();
        let cap = self.scratch.len() / ch;
        let produced = {
            let Ok(input) = InterleavedSlice::new(&self.pending[..need * ch], ch, need) else {
                return false;
            };
            let Ok(mut output) = InterleavedSlice::new_mut(&mut self.scratch[..], ch, cap) else {
                return false;
            };
            match self.inner.process_into_buffer(&input, &mut output, None) {
                Ok((_, n)) => n,
                Err(_) => return false,
            }
        };
        self.pending.drain(..need * ch);

        let start = self.skip.min(produced);
        self.skip -= start;
        let mut take = produced - start;
        if let Some(limit) = self.limit {
            take = take.min(limit.saturating_sub(self.frames_out) as usize);
        }
        out.extend_from_slice(&self.scratch[start * ch..(start + take) * ch]);
        self.frames_out += take as u64;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn output_length_and_alignment() {
        let (from, to) = (44_100u32, 48_000u32);
        let n = 44_100usize;
        let input: Vec<f64> = (0..n)
            .flat_map(|i| {
                let s = 0.5 * (2.0 * PI * 1000.0 * i as f64 / from as f64).sin();
                [s, s]
            })
            .collect();

        let mut rc = RateConverter::new(from, to, 2).unwrap();
        let mut out = Vec::new();
        for block in input.chunks(1234 * 2) {
            rc.process(block, &mut out);
        }
        rc.finish(&mut out);
        assert_eq!(out.len() / 2, 48_000, "one second in must be one second out");

        // The delay is trimmed, so the output must line up with an ideal 1 kHz sine.
        let mut err = 0.0f64;
        for i in 2_000..46_000 {
            let ideal = 0.5 * (2.0 * PI * 1000.0 * i as f64 / to as f64).sin();
            err = err.max((out[i * 2] - ideal).abs());
        }
        assert!(err < 1e-3, "max deviation from ideal sine: {err}");
    }

    #[test]
    fn reset_restores_alignment() {
        let mut rc = RateConverter::new(96_000, 48_000, 2).unwrap();
        let mut out = Vec::new();
        rc.process(&vec![0.1; 96_000], &mut out);
        rc.reset();
        out.clear();
        rc.process(&vec![0.0; 96_000 * 2], &mut out);
        rc.finish(&mut out);
        assert_eq!(out.len() / 2, 48_000);
    }
}
