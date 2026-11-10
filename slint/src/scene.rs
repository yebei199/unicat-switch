//! Port of `src/scene.ts`: sky, relit foreground, cat and lamplight composited into the float
//! raster, then dithered into a 224×96 RGBA image.

use crate::assets::Art;
use crate::cat::Cat;
use crate::color::{Rgb, hex, js_round, mix};
use crate::daylight::daylight;
use crate::dither::posterize;
use crate::layout::{GLOW, H, W, WINDOW};
use crate::raster::{Pixels, Raster, Sprite, blit};
use crate::sky::sky;

const LEVELS: u32 = 14;
const LAMP: Rgb = hex(0xffcf73);
const POOL: Rgb = hex(0xff9440);
const GLASS: Rgb = hex(0x46598f);

/// The window panes are the yellow pixels inside the frame: blue glass by day, warm light at night.
fn window_pixels(scene: &Pixels) -> Vec<bool> {
    let d = &scene.data;
    let mut found = vec![false; W * H];
    for y in WINDOW.y0..=WINDOW.y1 {
        for x in WINDOW.x0..=WINDOW.x1 {
            let p = y as usize * W + x as usize;
            let i = p * 4;
            if d[i] > 140
                && d[i + 1] > 140
                && d[i + 2] < 110
            {
                found[p] = true;
            }
        }
    }
    found
}

pub struct Scene {
    pub image: Pixels,
    raster: Raster,
    glass: Vec<bool>,
    art: &'static Art,
}

impl Scene {
    pub fn new(art: &'static Art) -> Self {
        Self {
            image: Pixels::new(W, H),
            raster: Raster::new(W, H),
            glass: window_pixels(&art.scene),
            art,
        }
    }

    pub fn render(
        &mut self,
        t: f64,
        time: f64,
        cat: &Cat,
    ) -> &Pixels {
        let day = daylight(t);
        sky(&mut self.raster, t, &day, time);
        self.foreground(day.light, day.night);
        self.cat(cat, day.light);
        self.lamplight(day.night);
        posterize(&self.raster, &mut self.image, LEVELS);
        &self.image
    }

    fn foreground(&mut self, light: Rgb, night: f64) {
        let d = &self.art.scene.data;
        let lit = mix(GLASS, LAMP, night);
        for y in 0..H {
            for x in 0..W {
                let i = (y * W + x) * 4;
                if d[i + 3] < 128 {
                    continue;
                }
                if self.glass[y * W + x] {
                    let pane = [
                        GLASS[0] * light[0],
                        GLASS[1] * light[1],
                        GLASS[2] * light[2],
                    ];
                    self.raster.set(
                        x,
                        y,
                        mix(pane, lit, night),
                    );
                    continue;
                }
                let c = [
                    f64::from(d[i]) / 255.0 * light[0],
                    f64::from(d[i + 1]) / 255.0 * light[1],
                    f64::from(d[i + 2]) / 255.0 * light[2],
                ];
                self.raster.set(x, y, c);
            }
        }
    }

    /// Window light on the wall and the grass by the door; added after the cat, so the cat sitting
    /// at the door at night is lit too.
    fn lamplight(&mut self, night: f64) {
        if night <= 0.0 {
            return;
        }
        let cx = f64::from(WINDOW.x0 + WINDOW.x1) / 2.0;
        let cy = f64::from(WINDOW.y0 + WINDOW.y1) / 2.0;
        self.raster.halo(cx, cy, 16.0, LAMP, 0.32 * night);
        for y in GLOW.y - GLOW.ry..=GLOW.y + GLOW.ry {
            for x in GLOW.x - GLOW.rx..=GLOW.x + GLOW.rx {
                let fx = f64::from(x - GLOW.x)
                    / f64::from(GLOW.rx);
                let fy = f64::from(y - GLOW.y)
                    / f64::from(GLOW.ry);
                let f = 1.0 - fx * fx - fy * fy;
                if f > 0.0 {
                    self.raster.add(
                        x,
                        y,
                        POOL,
                        0.42 * night * f,
                    );
                }
            }
        }
    }

    fn cat(&mut self, cat: &Cat, light: Rgb) {
        let sheet = if cat.walking {
            &self.art.walk
        } else {
            &self.art.sit
        };
        let sw = if cat.walking {
            sheet.width / 4
        } else {
            sheet.width
        };
        let sprite = Sprite {
            image: sheet,
            sx: if cat.walking {
                cat.frame() * sw
            } else {
                0
            },
            sy: 0,
            sw,
            sh: sheet.height,
        };
        // Walk frames face right, the sitting pose faces left: no flip when sitting, so the cat
        // looks at the house.
        let flip = cat.walking && cat.facing < 0.0;
        let dx = js_round(cat.x - sw as f64 / 2.0) as i32;
        blit(
            &mut self.raster,
            &sprite,
            dx,
            cat.baseline() - sheet.height as i32,
            flip,
            light,
        );
    }
}
