//! Port of `src/glass.ts` from a WebGL fragment shader to a CPU loop: the low-resolution scene is
//! upscaled nearest-neighbour across the pill track, and the glass ball refracts it spherically.
//! Math is f32 like `highp` GLSL; every pixel is shaded at its centre like `gl_FragCoord`.

use crate::raster::Pixels;

/// Ball centre and the two semi-axes, in canvas device pixels, origin top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub rx: f32,
    pub ry: f32,
}

type Vec3 = [f32; 3];

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: f32, b: f32, u: f32) -> f32 {
    a + (b - a) * u
}

fn mix3(a: Vec3, b: Vec3, u: f32) -> Vec3 {
    [
        mix(a[0], b[0], u),
        mix(a[1], b[1], u),
        mix(a[2], b[2], u),
    ]
}

fn scale3(c: Vec3, k: f32) -> Vec3 {
    [c[0] * k, c[1] * k, c[2] * k]
}

/// `asin(r)` mapping: the centre magnifies about 1.57×, compression grows toward the rim, and at
/// the rim the mapping meets the picture outside (k = 1). Returns the factor that scales the
/// normalized offset `d` (|d| = r) back into pixels. (The shader spells the constants 1.5707963
/// and 0.6366; the first is the same f32, the second only applies where `d` < 1e-4.)
pub fn refraction(r: f32) -> f32 {
    use std::f32::consts::{FRAC_2_PI, FRAC_PI_2};
    if r > 1e-4 {
        r.asin() / (FRAC_PI_2 * r)
    } else {
        FRAC_2_PI
    }
}

fn length(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

/// `texture2D` coordinate with NEAREST filtering and CLAMP_TO_EDGE: canvas px → texel index.
/// (The value is never negative, so the truncating cast is `floor` without a libm call.)
fn texel(v: f32, res: f32, size: usize) -> usize {
    (((v / res).clamp(0.0, 1.0) * size as f32) as usize)
        .min(size - 1)
}

/// The shader's `uniform`s plus the scene texture.
struct Frag<'a> {
    scene: &'a Pixels,
    res: [f32; 2],
    ball: Ball,
    night: f32,
}

impl Frag<'_> {
    fn texel(&self, x: usize, y: usize) -> Vec3 {
        let i = (y * self.scene.width + x) * 4;
        let d = &self.scene.data;
        [
            f32::from(d[i]) / 255.0,
            f32::from(d[i + 1]) / 255.0,
            f32::from(d[i + 2]) / 255.0,
        ]
    }

    fn scene(&self, p: [f32; 2]) -> Vec3 {
        self.texel(
            texel(p[0], self.res[0], self.scene.width),
            texel(p[1], self.res[1], self.scene.height),
        )
    }

    /// Signed distance to the pill outline (negative inside).
    fn pill(&self, p: [f32; 2]) -> f32 {
        let r = self.res[1] * 0.5;
        let qx = (p[0] - self.res[0] * 0.5).abs()
            - (self.res[0] * 0.5 - r);
        let qy = (p[1] - self.res[1] * 0.5).abs();
        length(qx.max(0.0), qy.max(0.0))
            + qx.max(qy).min(0.0)
            - r
    }

    fn glass(&self, d: [f32; 2], r: f32) -> Vec3 {
        let Ball { x, y, rx, ry } = self.ball;
        let k = refraction(r);
        let off = [d[0] * rx * k, d[1] * ry * k];
        let ca = 0.014 * r * r;
        let mut c = [
            self.scene([
                x + off[0] * (1.0 + ca),
                y + off[1] * (1.0 + ca),
            ])[0],
            self.scene([x + off[0], y + off[1]])[1],
            self.scene([
                x + off[0] * (1.0 - ca),
                y + off[1] * (1.0 - ca),
            ])[2],
        ];
        let z = (1.0 - r * r).max(0.0).sqrt();
        c = mix3(
            c,
            [0.96, 0.98, 1.0],
            (1.0 - z).powi(3) * mix(0.55, 0.12, self.night),
        );
        c = scale3(
            c,
            mix(1.0, 0.8, smoothstep(0.86, 1.0, r)),
        );
        let h = [d[0] + 0.36, (d[1] + 0.48) * 1.8];
        let spec =
            smoothstep(0.36, 0.04, length(h[0], h[1]))
                * mix(0.62, 0.2, self.night);
        let dir = [d[0] / r.max(1e-4), d[1] / r.max(1e-4)];
        let n = length(0.55, 0.83);
        let rim = smoothstep(0.76, 0.97, r)
            * smoothstep(
                0.2,
                1.0,
                dir[0] * (0.55 / n) + dir[1] * (0.83 / n),
            );
        let glint = rim * mix(0.4, 0.14, self.night);
        [
            c[0] + spec + glint,
            c[1] + spec + glint,
            c[2] + spec + glint,
        ]
    }

    /// `main()` for the pixel centred at `p` whose base texel is `base`: colour and coverage.
    /// Terms that are provably zero (outside the pill, beyond the shadow, outside the ball) are
    /// skipped; the arithmetic that remains is the shader's.
    fn shade(
        &self,
        p: [f32; 2],
        base: Vec3,
    ) -> (Vec3, f32) {
        let sd = self.pill(p);
        let mask = (0.5 - sd).clamp(0.0, 1.0);
        if mask <= 0.0 {
            return ([0.0; 3], 0.0);
        }
        let Ball { x, y, rx, ry } = self.ball;
        let mut c = base;
        let sp = [
            (p[0] - x) / (rx * 1.04),
            (p[1] - y - ry * 0.12) / (ry * 1.04),
        ];
        if sp[0] * sp[0] + sp[1] * sp[1] < 1.25 * 1.25 {
            c = scale3(
                c,
                1.0 - 0.3
                    * smoothstep(
                        1.25,
                        0.9,
                        length(sp[0], sp[1]),
                    ),
            );
        }
        if sd > -self.res[1] * 0.1 {
            let inner =
                smoothstep(-self.res[1] * 0.1, 0.0, sd);
            c = scale3(
                c,
                1.0 - inner
                    * (0.22
                        + 0.25
                            * smoothstep(
                                self.res[1] * 0.45,
                                0.0,
                                p[1],
                            )),
            );
        }
        let d = [(p[0] - x) / rx, (p[1] - y) / ry];
        if d[0] * d[0] + d[1] * d[1] < 1.0 {
            let r = length(d[0], d[1]);
            let edge = ((1.0 - r) * rx).clamp(0.0, 1.0);
            if edge > 0.0 {
                c = mix3(
                    c,
                    self.glass(d, r.min(0.999)),
                    edge,
                );
            }
        }
        (c, mask)
    }
}

/// Unorm8 store, rounding to nearest (the cast truncates a non-negative value, no libm `round`).
fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Shades a `width`×`height` canvas and composites it over `out` (straight-alpha RGBA8 whose
/// rows are `stride` bytes apart, starting at the canvas origin), like the WebGL canvas over
/// the button behind it.
pub fn draw(
    scene: &Pixels,
    width: usize,
    height: usize,
    ball: Ball,
    night: f32,
    out: &mut [u8],
    stride: usize,
) {
    let frag = Frag {
        scene,
        res: [width as f32, height as f32],
        ball,
        night,
    };
    let columns: Vec<usize> = (0..width)
        .map(|x| {
            texel(x as f32 + 0.5, width as f32, scene.width)
        })
        .collect();
    for y in 0..height {
        let py = y as f32 + 0.5;
        let ty = texel(py, height as f32, scene.height);
        let row =
            &mut out[y * stride..y * stride + width * 4];
        for ((x, px), &tx) in row
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .enumerate()
            .zip(&columns)
        {
            let (c, mask) = frag.shade(
                [x as f32 + 0.5, py],
                frag.texel(tx, ty),
            );
            over(px, c, mask);
        }
    }
}

/// Straight-alpha "over"; a fully covered pixel gets exactly the unorm8 of `c`.
fn over(px: &mut [u8; 4], c: Vec3, a: f32) {
    if a <= 0.0 {
        return;
    }
    if a >= 1.0 {
        *px = [to_u8(c[0]), to_u8(c[1]), to_u8(c[2]), 255];
        return;
    }
    let da = f32::from(px[3]) / 255.0;
    let oa = a + da * (1.0 - a);
    let blend = |s: f32, d: u8| {
        to_u8(
            (s.clamp(0.0, 1.0) * a
                + f32::from(d) / 255.0 * da * (1.0 - a))
                / oa,
        )
    };
    *px = [
        blend(c[0], px[0]),
        blend(c[1], px[1]),
        blend(c[2], px[2]),
        to_u8(oa),
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> Pixels {
        let mut p = Pixels::new(8, 4);
        for (i, px) in p.data.chunks_mut(4).enumerate() {
            px.copy_from_slice(&[
                (i * 37 % 256) as u8,
                (i * 91 % 256) as u8,
                (i * 13 % 256) as u8,
                255,
            ]);
        }
        p
    }

    #[test]
    fn refraction_at_centre_is_identity() {
        // d = 0: the offset vanishes, so the centre samples exactly the pixel under it.
        assert_eq!(0.0 * refraction(0.0), 0.0);
        let scene = checker();
        let ball = Ball {
            x: 40.5,
            y: 20.5,
            rx: 18.0,
            ry: 18.0,
        };
        let frag = Frag {
            scene: &scene,
            res: [80.0, 40.0],
            ball,
            night: 0.0,
        };
        assert_eq!(
            frag.glass([0.0, 0.0], 0.0),
            frag.scene([ball.x, ball.y])
        );
    }

    #[test]
    fn refraction_meets_the_outside_at_the_rim() {
        // asin(1) / (π/2) = 1: at the rim the ball samples the pixel underneath, no seam.
        assert!((refraction(1.0) - 1.0).abs() < 1e-6);
        assert!(
            (refraction(1e-3) - 2.0 / std::f32::consts::PI)
                .abs()
                < 1e-4
        );
        assert!(
            (refraction(5e-5) - 2.0 / std::f32::consts::PI)
                .abs()
                < 1e-4
        );
    }

    #[test]
    fn outside_the_pill_is_transparent_and_inside_opaque() {
        let scene = checker();
        let mut out = vec![0; 80 * 40 * 4];
        draw(
            &scene,
            80,
            40,
            Ball {
                x: 20.0,
                y: 20.0,
                rx: 17.6,
                ry: 17.6,
            },
            0.0,
            &mut out,
            80 * 4,
        );
        let alpha =
            |x: usize, y: usize| out[(y * 80 + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(79, 39), 0);
        assert_eq!(alpha(40, 20), 255);
    }
}
