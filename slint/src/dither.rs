//! Port of `src/dither.ts`: 4×4 Bayer ordered dithering, quantizing each channel to `levels`
//! steps in gamma space (finer steps in the shadows, so the night sky is not all noise).

use crate::raster::{Pixels, Raster};

const BAYER4: [u8; 16] =
    [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

const GAMMA: f64 = 1.8;

/// `Uint8ClampedArray` store: clamp, then round half to even.
fn clamped_u8(v: f64) -> u8 {
    let v = v.clamp(0.0, 255.0);
    let f = v.floor();
    let rounded = if f + 0.5 < v
        || (f + 0.5 == v && f % 2.0 == 1.0)
    {
        f + 1.0
    } else {
        f
    };
    rounded as u8
}

/// The quantizer for one level count, as thresholds in linear space: `q = v^(1/γ)·steps` crosses
/// level `k` exactly where `v` crosses `(k/steps)^γ`, so comparing `v` against precomputed powers
/// gives the TS result without two `pow` calls per channel per pixel.
struct Quantizer {
    steps: usize,
    /// `bounds[k]`: smallest `v` whose `floor(q)` is `k`.
    bounds: Vec<f64>,
    /// `cut[base * 16 + bayer]`: `v` above which `q - base` exceeds that Bayer threshold.
    cut: Vec<f64>,
    /// Output byte of each level.
    level: Vec<u8>,
}

impl Quantizer {
    fn new(levels: u32) -> Self {
        let steps = (levels - 1) as usize;
        let s = steps as f64;
        let bounds = (0..=steps)
            .map(|k| (k as f64 / s).powf(GAMMA))
            .collect();
        let cut = (0..=steps)
            .flat_map(|base| {
                (0..16).map(move |b| {
                    ((base as f64
                        + (b as f64 + 0.5) / 16.0)
                        / s)
                        .powf(GAMMA)
                })
            })
            .collect();
        let level = (0..=steps)
            .map(|k| {
                clamped_u8(
                    (k as f64 / s).powf(GAMMA) * 255.0,
                )
            })
            .collect();
        Self {
            steps,
            bounds,
            cut,
            level,
        }
    }

    fn quantize(&self, v: f64, bayer: usize) -> u8 {
        let v = v.clamp(0.0, 1.0);
        let base = (1..=self.steps)
            .take_while(|&k| self.bounds[k] <= v)
            .count();
        let up =
            usize::from(v > self.cut[base * 16 + bayer]);
        self.level[(base + up).min(self.steps)]
    }
}

pub fn posterize(
    src: &Raster,
    out: &mut Pixels,
    levels: u32,
) {
    let q = Quantizer::new(levels);
    for y in 0..src.h {
        for x in 0..src.w {
            let bayer =
                usize::from(BAYER4[(y & 3) * 4 + (x & 3)]);
            let i = y * src.w + x;
            for k in 0..3 {
                out.data[i * 4 + k] = q.quantize(
                    f64::from(src.rgb[i * 3 + k]),
                    bayer,
                );
            }
            out.data[i * 4 + 3] = 255;
        }
    }
}

/// The TS loop verbatim (two `pow` per channel); the reference the tables are tested against.
#[cfg(test)]
fn posterize_pow(
    src: &Raster,
    out: &mut Pixels,
    levels: u32,
) {
    let steps = f64::from(levels - 1);
    for y in 0..src.h {
        for x in 0..src.w {
            let threshold =
                (f64::from(BAYER4[(y & 3) * 4 + (x & 3)])
                    + 0.5)
                    / 16.0;
            let i = y * src.w + x;
            for k in 0..3 {
                let q = f64::from(src.rgb[i * 3 + k])
                    .clamp(0.0, 1.0)
                    .powf(1.0 / GAMMA)
                    * steps;
                let base = q.floor();
                let up = if q - base > threshold {
                    1.0
                } else {
                    0.0
                };
                out.data[i * 4 + k] = clamped_u8(
                    ((base + up) / steps).powf(GAMMA)
                        * 255.0,
                );
            }
            out.data[i * 4 + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient() -> Raster {
        let mut r = Raster::new(32, 8);
        for (i, v) in r.rgb.iter_mut().enumerate() {
            *v = (i % 97) as f32 / 96.0;
        }
        r
    }

    #[test]
    fn dither_is_deterministic() {
        let src = gradient();
        let mut a = Pixels::new(32, 8);
        let mut b = Pixels::new(32, 8);
        posterize(&src, &mut a, 14);
        posterize(&src, &mut b, 14);
        assert_eq!(a, b);
    }

    #[test]
    fn tables_match_the_pow_formula() {
        // Every f32 step of a fine ramp, at every Bayer phase.
        let mut src = Raster::new(4096, 4);
        for (i, v) in src.rgb.iter_mut().enumerate() {
            *v = (i / 3 % 4096) as f32 / 4095.0 * 1.02
                - 0.01;
        }
        let (mut a, mut b) =
            (Pixels::new(4096, 4), Pixels::new(4096, 4));
        posterize(&src, &mut a, 14);
        posterize_pow(&src, &mut b, 14);
        assert_eq!(a, b);
    }

    #[test]
    fn dither_output_stays_on_the_level_grid() {
        let levels: Vec<u8> = (0..14)
            .map(|n| {
                clamped_u8(
                    (f64::from(n) / 13.0).powf(GAMMA)
                        * 255.0,
                )
            })
            .collect();
        let mut out = Pixels::new(32, 8);
        posterize(&gradient(), &mut out, 14);
        for px in out.data.chunks(4) {
            assert!(
                px[..3].iter().all(|c| levels.contains(c)),
                "{px:?} off the 14-level grid"
            );
            assert_eq!(px[3], 255);
        }
    }

    #[test]
    fn flat_mid_gray_mixes_two_neighbouring_levels() {
        let mut src = Raster::new(4, 4);
        src.rgb.fill(0.3);
        let mut out = Pixels::new(4, 4);
        posterize(&src, &mut out, 14);
        let mut seen: Vec<u8> =
            out.data.chunks(4).map(|p| p[0]).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.len(),
            2,
            "a flat 0.3 should dither between two levels, got {seen:?}"
        );
    }

    #[test]
    fn clamped_store_rounds_half_to_even() {
        assert_eq!(clamped_u8(2.5), 2);
        assert_eq!(clamped_u8(3.5), 4);
        assert_eq!(clamped_u8(-3.0), 0);
        assert_eq!(clamped_u8(300.0), 255);
    }
}
