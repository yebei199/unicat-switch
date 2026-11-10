//! Port of `src/raster.ts`: the linear-RGB float canvas every layer is composited into before the
//! single dither pass, plus sprite blitting.

use crate::color::Rgb;

/// An RGBA8 image (`ImageData` in the TS version): decoded art and the dithered scene.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pixels {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl Pixels {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0; width * height * 4],
        }
    }
}

pub struct Raster {
    pub w: usize,
    pub h: usize,
    /// `Float32Array` in the TS version: storage is f32, arithmetic is f64.
    pub rgb: Vec<f32>,
}

impl Raster {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            rgb: vec![0.0; w * h * 3],
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let inside = x >= 0
            && y >= 0
            && (x as usize) < self.w
            && (y as usize) < self.h;
        inside
            .then(|| (y as usize * self.w + x as usize) * 3)
    }

    pub fn set(&mut self, x: usize, y: usize, c: Rgb) {
        let i = (y * self.w + x) * 3;
        for k in 0..3 {
            self.rgb[i + k] = c[k] as f32;
        }
    }

    pub fn blend(
        &mut self,
        x: i32,
        y: i32,
        c: Rgb,
        a: f64,
    ) {
        let Some(i) = self.index(x, y) else { return };
        for k in 0..3 {
            let v = f64::from(self.rgb[i + k]);
            self.rgb[i + k] = (v + (c[k] - v) * a) as f32;
        }
    }

    pub fn add(&mut self, x: i32, y: i32, c: Rgb, k: f64) {
        let Some(i) = self.index(x, y) else { return };
        for n in 0..3 {
            self.rgb[i + n] = (f64::from(self.rgb[i + n])
                + c[n] * k)
                as f32;
        }
    }

    /// Solid disc with no anti-aliasing: pixel art wants hard edges.
    pub fn disc(
        &mut self,
        cx: f64,
        cy: f64,
        r: f64,
        c: Rgb,
    ) {
        let mut y = (cy - r).floor();
        while y <= cy + r {
            let mut x = (cx - r).floor();
            while x <= cx + r {
                if (x + 0.5 - cx).powi(2)
                    + (y + 0.5 - cy).powi(2)
                    <= r * r
                {
                    self.blend(x as i32, y as i32, c, 1.0);
                }
                x += 1.0;
            }
            y += 1.0;
        }
    }

    /// Additive glow with radial falloff; the dither pass later turns it into dots.
    pub fn halo(
        &mut self,
        cx: f64,
        cy: f64,
        r: f64,
        c: Rgb,
        k: f64,
    ) {
        let mut y = (cy - r).floor();
        while y <= cy + r {
            let mut x = (cx - r).floor();
            while x <= cx + r {
                let d =
                    (x + 0.5 - cx).hypot(y + 0.5 - cy) / r;
                if d < 1.0 {
                    self.add(
                        x as i32,
                        y as i32,
                        c,
                        k * (1.0 - d).powi(2),
                    );
                }
                x += 1.0;
            }
            y += 1.0;
        }
    }
}

/// A rectangle of a sprite sheet (one frame).
pub struct Sprite<'a> {
    pub image: &'a Pixels,
    pub sx: usize,
    pub sy: usize,
    pub sw: usize,
    pub sh: usize,
}

/// Pastes the sprite multiplied by `tint`; `flip` mirrors it horizontally.
pub fn blit(
    dst: &mut Raster,
    s: &Sprite,
    dx: i32,
    dy: i32,
    flip: bool,
    tint: Rgb,
) {
    let d = &s.image.data;
    for y in 0..s.sh {
        for x in 0..s.sw {
            let sx =
                s.sx + if flip { s.sw - 1 - x } else { x };
            let i = ((s.sy + y) * s.image.width + sx) * 4;
            if d[i + 3] < 128 {
                continue;
            }
            let c = [
                f64::from(d[i]) / 255.0 * tint[0],
                f64::from(d[i + 1]) / 255.0 * tint[1],
                f64::from(d[i + 2]) / 255.0 * tint[2],
            ];
            dst.blend(dx + x as i32, dy + y as i32, c, 1.0);
        }
    }
}
