//! Final conversion from the engine's f64 samples to the device's wire format.

/// Sample layout expected by the output device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFmt {
    I16,
    /// 24 bit packed in 3 bytes.
    I24P3,
    /// 24 significant bits, left justified in a 32 bit container.
    I24In32,
    I32,
    F32,
}

impl SampleFmt {
    pub fn bytes(self) -> usize {
        match self {
            SampleFmt::I16 => 2,
            SampleFmt::I24P3 => 3,
            SampleFmt::I24In32 | SampleFmt::I32 | SampleFmt::F32 => 4,
        }
    }

    /// Significant bits per sample.
    pub fn bits(self) -> u16 {
        match self {
            SampleFmt::I16 => 16,
            SampleFmt::I24P3 | SampleFmt::I24In32 => 24,
            SampleFmt::I32 | SampleFmt::F32 => 32,
        }
    }

    pub fn is_float(self) -> bool {
        self == SampleFmt::F32
    }
}

/// Small, fast PRNG for dither noise. Quality requirements here are modest; speed matters.
pub struct Rng(u32);

impl Rng {
    pub fn new() -> Self {
        Rng(0x9E37_79B9)
    }

    #[inline]
    fn next_unit(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x as f64 / 4_294_967_296.0
    }

    /// Triangular PDF noise in (-1, 1), i.e. +-1 LSB peak.
    #[inline]
    pub fn tpdf(&mut self) -> f64 {
        self.next_unit() - self.next_unit()
    }
}

/// Gain applied while converting, with a linear ramp towards `target` to avoid zipper noise
/// and clicks on pause, seek and volume changes.
pub struct GainRamp {
    pub current: f64,
    pub target: f64,
    /// Change per frame.
    pub step: f64,
}

impl GainRamp {
    #[inline]
    fn advance(&mut self) -> f64 {
        if self.current < self.target {
            self.current = (self.current + self.step).min(self.target);
        } else if self.current > self.target {
            self.current = (self.current - self.step).max(self.target);
        }
        self.current
    }

    /// True when the gain stage is a mathematical no-op.
    #[inline]
    pub fn is_unity(&self) -> bool {
        self.current == 1.0 && self.target == 1.0
    }
}

#[inline]
fn quantize(s: f64, bits: u16, dither: Option<&mut Rng>) -> i32 {
    match dither {
        // No dither: scale to 32 bit and truncate to the device width. For integer sources
        // whose width fits the device this is exact, which is what makes bit-perfect possible.
        None => {
            let v = (s * 2_147_483_648.0).round();
            let v = v.clamp(-2_147_483_648.0, 2_147_483_647.0) as i32;
            v >> (32 - bits)
        }
        Some(rng) => {
            let scale = (1u64 << (bits - 1)) as f64;
            let v = (s * scale + rng.tpdf()).round();
            v.clamp(-scale, scale - 1.0) as i32
        }
    }
}

/// Convert interleaved f64 `src` into device bytes in `dst`.
///
/// `dst` must hold exactly `src.len() * fmt.bytes()` bytes. `dither` is ignored for float
/// output. When the gain ramp is at unity and no dither is requested the conversion is exact.
pub fn convert(
    src: &[f64],
    dst: &mut [u8],
    fmt: SampleFmt,
    channels: usize,
    gain: &mut GainRamp,
    mut dither: Option<&mut Rng>,
) {
    debug_assert_eq!(dst.len(), src.len() * fmt.bytes());
    let width = fmt.bytes();
    let unity = gain.is_unity();
    let bits = fmt.bits();

    for (frame, out) in src.chunks_exact(channels).zip(dst.chunks_exact_mut(channels * width)) {
        let g = if unity { 1.0 } else { gain.advance() };
        for (&s, o) in frame.iter().zip(out.chunks_exact_mut(width)) {
            let s = if unity { s } else { s * g };
            match fmt {
                SampleFmt::F32 => o.copy_from_slice(&(s as f32).to_le_bytes()),
                SampleFmt::I16 => {
                    let v = quantize(s, bits, dither.as_deref_mut());
                    o.copy_from_slice(&(v as i16).to_le_bytes());
                }
                SampleFmt::I24P3 => {
                    let v = quantize(s, bits, dither.as_deref_mut());
                    o.copy_from_slice(&v.to_le_bytes()[..3]);
                }
                SampleFmt::I24In32 => {
                    let v = quantize(s, bits, dither.as_deref_mut());
                    o.copy_from_slice(&(v << 8).to_le_bytes());
                }
                SampleFmt::I32 => {
                    let v = quantize(s, bits, dither.as_deref_mut());
                    o.copy_from_slice(&v.to_le_bytes());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unity() -> GainRamp {
        GainRamp { current: 1.0, target: 1.0, step: 0.001 }
    }

    /// Every 16 bit value survives the trip to f64 and out to any integer device format.
    #[test]
    fn i16_is_bit_exact_on_every_integer_format() {
        let ints: Vec<i16> = (i16::MIN..=i16::MAX).collect();
        // Same conversion symphonia uses for i16 -> f64.
        let f: Vec<f64> = ints.iter().map(|&s| s as f64 / 32_768.0).collect();

        let mut out = vec![0u8; f.len() * 2];
        convert(&f, &mut out, SampleFmt::I16, 1, &mut unity(), None);
        for (i, c) in out.chunks_exact(2).enumerate() {
            assert_eq!(i16::from_le_bytes([c[0], c[1]]), ints[i]);
        }

        let mut out = vec![0u8; f.len() * 3];
        convert(&f, &mut out, SampleFmt::I24P3, 1, &mut unity(), None);
        for (i, c) in out.chunks_exact(3).enumerate() {
            let v = i32::from_le_bytes([c[0], c[1], c[2], if c[2] & 0x80 != 0 { 0xFF } else { 0 }]);
            assert_eq!(v, (ints[i] as i32) << 8);
        }

        let mut out = vec![0u8; f.len() * 4];
        convert(&f, &mut out, SampleFmt::I24In32, 1, &mut unity(), None);
        for (i, c) in out.chunks_exact(4).enumerate() {
            assert_eq!(i32::from_le_bytes([c[0], c[1], c[2], c[3]]), (ints[i] as i32) << 16);
        }

        let mut out = vec![0u8; f.len() * 4];
        convert(&f, &mut out, SampleFmt::I32, 1, &mut unity(), None);
        for (i, c) in out.chunks_exact(4).enumerate() {
            assert_eq!(i32::from_le_bytes([c[0], c[1], c[2], c[3]]), (ints[i] as i32) << 16);
        }
    }

    /// 24 bit samples (as symphonia delivers FLAC: left justified in i32) are exact too.
    #[test]
    fn i24_is_bit_exact() {
        let mut rng = Rng::new();
        let mut ints: Vec<i32> = (0..200_000)
            .map(|_| ((rng.next_unit() * 16_777_216.0) as i32) - 8_388_608)
            .collect();
        ints.extend_from_slice(&[-8_388_608, 8_388_607, 0, -1, 1]);
        let f: Vec<f64> = ints.iter().map(|&s| ((s << 8) as f64) / 2_147_483_648.0).collect();

        let mut out = vec![0u8; f.len() * 3];
        convert(&f, &mut out, SampleFmt::I24P3, 1, &mut unity(), None);
        for (i, c) in out.chunks_exact(3).enumerate() {
            let v = i32::from_le_bytes([c[0], c[1], c[2], if c[2] & 0x80 != 0 { 0xFF } else { 0 }]);
            assert_eq!(v, ints[i]);
        }

        let mut out = vec![0u8; f.len() * 4];
        convert(&f, &mut out, SampleFmt::I24In32, 1, &mut unity(), None);
        for (i, c) in out.chunks_exact(4).enumerate() {
            assert_eq!(i32::from_le_bytes([c[0], c[1], c[2], c[3]]), ints[i] << 8);
        }
    }

    /// Full 32 bit integers are exact as well.
    #[test]
    fn i32_is_bit_exact() {
        let mut rng = Rng::new();
        let mut ints: Vec<i32> = (0..200_000).map(|_| (rng.next_unit() * 4_294_967_296.0) as i64 as i32).collect();
        ints.extend_from_slice(&[i32::MIN, i32::MAX, 0, -1, 1]);
        let f: Vec<f64> = ints.iter().map(|&s| s as f64 / 2_147_483_648.0).collect();
        let mut out = vec![0u8; f.len() * 4];
        convert(&f, &mut out, SampleFmt::I32, 1, &mut unity(), None);
        for (i, c) in out.chunks_exact(4).enumerate() {
            assert_eq!(i32::from_le_bytes([c[0], c[1], c[2], c[3]]), ints[i]);
        }
    }

    #[test]
    fn dither_stays_within_one_lsb_and_never_overflows() {
        let mut rng = Rng::new();
        let src = [1.0, -1.0, 0.999_999, -0.999_999, 0.0, 0.5];
        let mut out = vec![0u8; src.len() * 2];
        for _ in 0..1000 {
            convert(&src, &mut out, SampleFmt::I16, 1, &mut unity(), Some(&mut rng));
            let v: Vec<i16> = out.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
            assert!(v[0] >= 32_766 && v[1] <= -32_767);
            assert!((v[4] as i32).abs() <= 1);
            assert!((v[5] as i32 - 16_384).abs() <= 1);
        }
    }

    #[test]
    fn gain_ramp_reaches_target() {
        let mut g = GainRamp { current: 0.0, target: 1.0, step: 0.1 };
        let src = vec![1.0; 40];
        let mut out = vec![0u8; src.len() * 4];
        convert(&src, &mut out, SampleFmt::F32, 2, &mut g, None);
        assert!(g.is_unity());
        let first = f32::from_le_bytes([out[0], out[1], out[2], out[3]]);
        let last = f32::from_le_bytes(out[out.len() - 4..].try_into().unwrap());
        assert!(first < 0.2 && last == 1.0);
    }
}
