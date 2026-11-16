//! Optional DSP chain: preamp, parametric EQ, bass shelf, crossfeed, stereo width, limiter.
//!
//! The chain works on interleaved stereo f64 at the device rate. When `is_active()` is false
//! the engine does not call `process` at all, so an idle chain cannot alter the signal.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BandKind {
    Peak,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EqBand {
    pub kind: BandKind,
    pub freq: f64,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default = "default_q")]
    pub q: f64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_q() -> f64 {
    1.0
}
fn default_true() -> bool {
    true
}

impl EqBand {
    fn is_audible(&self) -> bool {
        self.enabled
            && match self.kind {
                BandKind::Peak | BandKind::LowShelf | BandKind::HighShelf => self.gain_db.abs() > 1e-6,
                _ => true,
            }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DspConfig {
    /// Master switch for the whole chain.
    pub enabled: bool,
    pub preamp_db: f64,
    pub eq_enabled: bool,
    pub bands: Vec<EqBand>,
    /// Low shelf boost at 90 Hz, in dB.
    pub bass_db: f64,
    /// Headphone crossfeed amount, 0..1.
    pub crossfeed: f64,
    /// Stereo width: 0 = mono, 1 = unchanged, 2 = extra wide.
    pub width: f64,
    pub limiter: bool,
}

impl Default for DspConfig {
    fn default() -> Self {
        DspConfig {
            enabled: false,
            preamp_db: 0.0,
            eq_enabled: false,
            bands: Vec::new(),
            bass_db: 0.0,
            crossfeed: 0.0,
            width: 1.0,
            limiter: true,
        }
    }
}

impl DspConfig {
    fn eq_active(&self) -> bool {
        self.eq_enabled && self.bands.iter().any(EqBand::is_audible)
    }

    /// True when the chain would change the signal.
    pub fn is_active(&self) -> bool {
        self.enabled
            && (self.preamp_db.abs() > 1e-6
                || self.eq_active()
                || self.bass_db.abs() > 1e-6
                || self.crossfeed > 1e-6
                || (self.width - 1.0).abs() > 1e-6)
    }
}

/// Biquad coefficients, normalised so that a0 = 1 (RBJ Audio EQ Cookbook).
#[derive(Clone, Copy, Debug)]
struct Coeffs {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Coeffs {
    fn new(kind: BandKind, rate: f64, freq: f64, gain_db: f64, q: f64) -> Coeffs {
        let freq = freq.clamp(10.0, rate * 0.49);
        let q = q.clamp(0.05, 30.0);
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match kind {
            BandKind::Peak => (
                1.0 + alpha * a,
                -2.0 * cos,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos,
                1.0 - alpha / a,
            ),
            BandKind::LowShelf => {
                let k = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos + k),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                    a * ((a + 1.0) - (a - 1.0) * cos - k),
                    (a + 1.0) + (a - 1.0) * cos + k,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                    (a + 1.0) + (a - 1.0) * cos - k,
                )
            }
            BandKind::HighShelf => {
                let k = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos + k),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                    a * ((a + 1.0) + (a - 1.0) * cos - k),
                    (a + 1.0) - (a - 1.0) * cos + k,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos),
                    (a + 1.0) - (a - 1.0) * cos - k,
                )
            }
            BandKind::LowPass => (
                (1.0 - cos) / 2.0,
                1.0 - cos,
                (1.0 - cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            BandKind::HighPass => (
                (1.0 + cos) / 2.0,
                -(1.0 + cos),
                (1.0 + cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            BandKind::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        };
        Coeffs { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }
}

/// Stereo biquad, transposed direct form II.
#[derive(Clone, Copy, Debug)]
struct Biquad {
    c: Coeffs,
    z: [[f64; 2]; 2],
}

impl Biquad {
    fn new(c: Coeffs) -> Biquad {
        Biquad { c, z: [[0.0; 2]; 2] }
    }

    #[inline]
    fn tick(&mut self, ch: usize, x: f64) -> f64 {
        let c = &self.c;
        let z = &mut self.z[ch];
        let y = c.b0 * x + z[0];
        z[0] = c.b1 * x - c.a1 * y + z[1];
        z[1] = c.b2 * x - c.a2 * y;
        y
    }
}

const LOOKAHEAD: usize = 64;
/// Limiter ceiling, about -0.1 dBFS.
const CEILING: f64 = 0.988;

/// Look-ahead peak limiter. Transparent below the ceiling, it only engages when DSP boosts
/// would otherwise clip.
struct Limiter {
    delay: [[f64; 2]; LOOKAHEAD],
    idx: usize,
    gain: f64,
    attack: f64,
    release: f64,
}

impl Limiter {
    fn new(rate: f64) -> Limiter {
        Limiter {
            delay: [[0.0; 2]; LOOKAHEAD],
            idx: 0,
            gain: 1.0,
            // Reach the target comfortably within the look-ahead window.
            attack: (-1.0 / (LOOKAHEAD as f64 / 8.0)).exp(),
            release: (-1.0 / (0.15 * rate)).exp(),
        }
    }

    #[inline]
    fn tick(&mut self, l: f64, r: f64) -> (f64, f64) {
        let [dl, dr] = self.delay[self.idx];
        self.delay[self.idx] = [l, r];
        self.idx = (self.idx + 1) % LOOKAHEAD;

        // Hold the reduction for as long as a peak is anywhere inside the delay line, so the
        // gain cannot start recovering before that peak has actually been played.
        let peak = self.delay.iter().fold(0.0f64, |m, f| m.max(f[0].abs()).max(f[1].abs()));
        let target = if peak > CEILING { CEILING / peak } else { 1.0 };
        let k = if target < self.gain { self.attack } else { self.release };
        self.gain = target + (self.gain - target) * k;

        ((dl * self.gain).clamp(-1.0, 1.0), (dr * self.gain).clamp(-1.0, 1.0))
    }

    fn reset(&mut self) {
        self.delay = [[0.0; 2]; LOOKAHEAD];
        self.gain = 1.0;
    }
}

pub struct DspChain {
    cfg: DspConfig,
    rate: f64,
    preamp: f64,
    eq: Vec<Biquad>,
    bass: Option<Biquad>,
    /// Crossfeed amount and one-pole low-pass state/coefficient.
    feed: f64,
    feed_lp: [f64; 2],
    feed_k: f64,
    limiter: Limiter,
}

impl DspChain {
    pub fn new() -> DspChain {
        let mut d = DspChain {
            cfg: DspConfig::default(),
            rate: 44_100.0,
            preamp: 1.0,
            eq: Vec::new(),
            bass: None,
            feed: 0.0,
            feed_lp: [0.0; 2],
            feed_k: 0.0,
            limiter: Limiter::new(44_100.0),
        };
        d.rebuild(true);
        d
    }

    pub fn is_active(&self) -> bool {
        self.cfg.is_active()
    }

    pub fn set_config(&mut self, cfg: DspConfig) {
        // Keep filter state when only parameters moved, so dragging a band does not click.
        let same_shape = cfg.bands.len() == self.cfg.bands.len()
            && cfg.eq_active() == self.cfg.eq_active()
            && cfg.bands.iter().zip(&self.cfg.bands).all(|(a, b)| a.is_audible() == b.is_audible());
        self.cfg = cfg;
        self.rebuild(!same_shape);
    }

    pub fn set_rate(&mut self, rate: u32) {
        if (self.rate - rate as f64).abs() > 0.5 {
            self.rate = rate as f64;
            self.limiter = Limiter::new(self.rate);
            self.rebuild(true);
        }
    }

    /// Clear all filter memory (after a seek or track jump).
    pub fn reset(&mut self) {
        for b in &mut self.eq {
            b.z = [[0.0; 2]; 2];
        }
        if let Some(b) = &mut self.bass {
            b.z = [[0.0; 2]; 2];
        }
        self.feed_lp = [0.0; 2];
        self.limiter.reset();
    }

    fn rebuild(&mut self, clear_state: bool) {
        let cfg = &self.cfg;
        self.preamp = 10f64.powf(cfg.preamp_db / 20.0);

        let coeffs: Vec<Coeffs> = if cfg.eq_active() {
            cfg.bands
                .iter()
                .filter(|b| b.is_audible())
                .map(|b| Coeffs::new(b.kind, self.rate, b.freq, b.gain_db, b.q))
                .collect()
        } else {
            Vec::new()
        };
        if !clear_state && coeffs.len() == self.eq.len() {
            for (b, c) in self.eq.iter_mut().zip(coeffs) {
                b.c = c;
            }
        } else {
            self.eq = coeffs.into_iter().map(Biquad::new).collect();
        }

        if cfg.bass_db.abs() > 1e-6 {
            let c = Coeffs::new(BandKind::LowShelf, self.rate, 90.0, cfg.bass_db, 0.7);
            match &mut self.bass {
                Some(b) if !clear_state => b.c = c,
                slot => *slot = Some(Biquad::new(c)),
            }
        } else {
            self.bass = None;
        }

        self.feed = cfg.crossfeed.clamp(0.0, 1.0) * 0.5;
        self.feed_k = 1.0 - (-2.0 * PI * 700.0 / self.rate).exp();
    }

    /// Process interleaved stereo samples in place.
    pub fn process(&mut self, buf: &mut [f64]) {
        let width = self.cfg.width.clamp(0.0, 2.0);
        let widen = (width - 1.0).abs() > 1e-6;
        let feed = self.feed;
        let limit = self.cfg.limiter;

        for f in buf.chunks_exact_mut(2) {
            let mut l = f[0] * self.preamp;
            let mut r = f[1] * self.preamp;

            for b in &mut self.eq {
                l = b.tick(0, l);
                r = b.tick(1, r);
            }
            if let Some(b) = &mut self.bass {
                l = b.tick(0, l);
                r = b.tick(1, r);
            }

            if feed > 0.0 {
                // Low-passed signal bleeds into the opposite ear; the direct path gets the
                // matching high boost so centred material keeps a flat response.
                self.feed_lp[0] += self.feed_k * (l - self.feed_lp[0]);
                self.feed_lp[1] += self.feed_k * (r - self.feed_lp[1]);
                let (ll, lr) = (self.feed_lp[0], self.feed_lp[1]);
                let dl = l + feed * (l - ll);
                let dr = r + feed * (r - lr);
                l = (dl + feed * lr) / (1.0 + feed);
                r = (dr + feed * ll) / (1.0 + feed);
            }

            if widen {
                let mid = (l + r) * 0.5;
                let side = (l - r) * 0.5 * width;
                l = mid + side;
                r = mid - side;
            }

            if limit {
                (l, r) = self.limiter.tick(l, r);
            }
            f[0] = l;
            f[1] = r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f64, rate: f64, n: usize) -> Vec<f64> {
        (0..n)
            .flat_map(|i| {
                let s = 0.25 * (2.0 * PI * freq * i as f64 / rate).sin();
                [s, s]
            })
            .collect()
    }

    fn rms(buf: &[f64]) -> f64 {
        (buf.iter().map(|x| x * x).sum::<f64>() / buf.len() as f64).sqrt()
    }

    #[test]
    fn default_config_is_inactive() {
        assert!(!DspConfig::default().is_active());
        let mut on = DspConfig { enabled: true, ..Default::default() };
        assert!(!on.is_active());
        on.eq_enabled = true;
        on.bands.push(EqBand { kind: BandKind::Peak, freq: 1000.0, gain_db: 0.0, q: 1.0, enabled: true });
        assert!(!on.is_active(), "a flat band must not activate the chain");
        on.bands[0].gain_db = 3.0;
        assert!(on.is_active());
    }

    #[test]
    fn peak_band_boosts_its_centre_frequency() {
        let mut d = DspChain::new();
        d.set_rate(48_000);
        d.set_config(DspConfig {
            enabled: true,
            eq_enabled: true,
            limiter: false,
            bands: vec![EqBand { kind: BandKind::Peak, freq: 1000.0, gain_db: 6.0, q: 1.0, enabled: true }],
            ..Default::default()
        });

        let mut at = sine(1000.0, 48_000.0, 48_000);
        let before = rms(&at[48_000..]);
        d.process(&mut at);
        let gain_db = 20.0 * (rms(&at[48_000..]) / before).log10();
        assert!((gain_db - 6.0).abs() < 0.1, "got {gain_db} dB at centre");

        d.reset();
        let mut far = sine(60.0, 48_000.0, 48_000);
        let before = rms(&far[48_000..]);
        d.process(&mut far);
        let gain_db = 20.0 * (rms(&far[48_000..]) / before).log10();
        assert!(gain_db.abs() < 0.3, "got {gain_db} dB far from centre");
    }

    #[test]
    fn crossfeed_keeps_mono_level_and_bleeds_hard_panned_bass() {
        let mut d = DspChain::new();
        d.set_rate(48_000);
        d.set_config(DspConfig { enabled: true, crossfeed: 1.0, limiter: false, ..Default::default() });

        let mut mono = sine(200.0, 48_000.0, 48_000);
        let before = rms(&mono[48_000..]);
        d.process(&mut mono);
        assert!((rms(&mono[48_000..]) / before - 1.0).abs() < 0.01);

        d.reset();
        let mut left: Vec<f64> = sine(100.0, 48_000.0, 48_000);
        left.chunks_exact_mut(2).for_each(|f| f[1] = 0.0);
        d.process(&mut left);
        let right: Vec<f64> = left.chunks_exact(2).skip(24_000).map(|f| f[1]).collect();
        assert!(rms(&right) > 0.03, "bass should reach the opposite channel");
    }

    #[test]
    fn limiter_never_exceeds_full_scale() {
        let mut d = DspChain::new();
        d.set_rate(44_100);
        d.set_config(DspConfig { enabled: true, preamp_db: 12.0, limiter: true, ..Default::default() });
        let mut buf = sine(440.0, 44_100.0, 44_100);
        buf.iter_mut().for_each(|s| *s *= 3.0);
        d.process(&mut buf);
        assert!(buf.iter().all(|s| s.abs() <= 1.0));
        // After the attack settles the output should sit at the ceiling, not be hard clipped.
        let tail_peak = buf[44_100..].iter().fold(0.0f64, |m, s| m.max(s.abs()));
        assert!(tail_peak <= CEILING + 0.01 && tail_peak > 0.9);
    }

    #[test]
    fn width_zero_is_mono() {
        let mut d = DspChain::new();
        d.set_config(DspConfig { enabled: true, width: 0.0, limiter: false, ..Default::default() });
        let mut buf = vec![0.5, -0.25, 0.1, 0.3];
        d.process(&mut buf);
        assert!((buf[0] - buf[1]).abs() < 1e-12 && (buf[2] - buf[3]).abs() < 1e-12);
    }
}
