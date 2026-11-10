//! Port of `src/sky.ts`: gradient, sun and moon, twinkling stars, drifting clouds.

use std::f64::consts::PI;
use std::sync::LazyLock;

use crate::color::{Rgb, hex, lerp, mix, smoothstep};
use crate::daylight::Daylight;
use crate::layout::{H, HORIZON, W};
use crate::raster::Raster;

const SUN: Rgb = hex(0xfff4c8);
const SUNSET: Rgb = hex(0xff9a4a);
const MOON: Rgb = hex(0xf1eedc);
const CRATER: Rgb = hex(0xc9c4ad);
const STAR: Rgb = hex(0xfffbe6);

struct Cloud {
    x: f64,
    y: i32,
    shape: [&'static str; 3],
}

const CLOUDS: [Cloud; 3] = [
    Cloud {
        x: 10.0,
        y: 9,
        shape: [
            "...####.....",
            ".#########..",
            "############",
        ],
    },
    Cloud {
        x: 120.0,
        y: 5,
        shape: ["....###...", ".########.", "##########"],
    },
    Cloud {
        x: 80.0,
        y: 24,
        shape: [
            "..####.......",
            "############.",
            "#############",
        ],
    },
];

/// mulberry32, bit-for-bit with the JS (`| 0`, `Math.imul`, `>>>` are all mod-2³² arithmetic).
fn mulberry32(seed: u32) -> impl FnMut() -> f64 {
    let mut a = seed;
    move || {
        a = a.wrapping_add(0x6d2b_79f5);
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add(
            (t ^ (t >> 7)).wrapping_mul(61 | t),
        ) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

struct Star {
    x: i32,
    y: i32,
    rate: f64,
    phase: f64,
    big: bool,
}

static STARS: LazyLock<Vec<Star>> = LazyLock::new(|| {
    let mut rand = mulberry32(7);
    (0..48)
        .map(|_| Star {
            x: (rand() * W as f64).floor() as i32,
            y: (rand() * (HORIZON - 8.0)).floor() as i32,
            rate: 1.0 + rand() * 2.5,
            phase: rand() * PI * 2.0,
            big: rand() < 0.12,
        })
        .collect()
});

fn gradient(dst: &mut Raster, day: &Daylight) {
    for y in 0..H {
        let c = mix(
            day.sky_top,
            day.sky_horizon,
            (y as f64 / HORIZON).min(1.0).powf(1.5),
        );
        for x in 0..W {
            dst.set(x, y, c);
        }
    }
}

/// The sun sets from upper middle to lower right, the moon rises from lower left to upper left;
/// for t in 0.45–0.55 both sit near the horizon.
fn celestial(dst: &mut Raster, t: f64, day: &Daylight) {
    let s = smoothstep(0.0, 0.55, t);
    if s < 1.0 {
        let sun = mix(SUN, SUNSET, day.dusk);
        let x = lerp(100.0, 236.0, s);
        let y = 12.0 + 62.0 * s.powf(1.6);
        dst.halo(x, y, 12.0, sun, 0.45);
        dst.disc(x, y, 4.5, sun);
    }
    let m = smoothstep(0.45, 1.0, t);
    if m > 0.0 {
        let x = lerp(-10.0, 86.0, m);
        let y = 14.0 + 60.0 * (1.0 - m).powf(1.6);
        dst.halo(x, y, 10.0, MOON, 0.18 * m);
        dst.disc(x, y, 4.0, MOON);
        dst.blend(
            x.floor() as i32 - 1,
            y.floor() as i32 - 1,
            CRATER,
            1.0,
        );
        dst.blend(
            x.floor() as i32 + 1,
            y.floor() as i32 + 1,
            CRATER,
            1.0,
        );
    }
}

fn stars(dst: &mut Raster, night: f64, time: f64) {
    if night <= 0.0 {
        return;
    }
    for s in STARS.iter() {
        let a = night
            * (0.55
                + 0.45 * (time * s.rate + s.phase).sin());
        dst.blend(s.x, s.y, STAR, a);
        if !s.big {
            continue;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            dst.blend(s.x + dx, s.y + dy, STAR, a * 0.45);
        }
    }
}

fn clouds(dst: &mut Raster, day: &Daylight, time: f64) {
    let lit = mix(day.sky_top, day.light, 0.82);
    let shade =
        [lit[0] * 0.86, lit[1] * 0.88, lit[2] * 0.93];
    for cloud in &CLOUDS {
        let width = cloud.shape[0].len() as f64;
        let x0 = (((cloud.x + time * 1.2)
            % (W as f64 + width))
            - width)
            .floor() as i32;
        for (dy, row) in cloud.shape.iter().enumerate() {
            let c = if dy == cloud.shape.len() - 1 {
                shade
            } else {
                lit
            };
            for (dx, cell) in row.bytes().enumerate() {
                if cell == b'#' {
                    dst.blend(
                        x0 + dx as i32,
                        cloud.y + dy as i32,
                        c,
                        1.0,
                    );
                }
            }
        }
    }
}

pub fn sky(
    dst: &mut Raster,
    t: f64,
    day: &Daylight,
    time: f64,
) {
    gradient(dst, day);
    stars(dst, day.night, time);
    celestial(dst, t, day);
    clouds(dst, day, time);
}

#[cfg(test)]
mod tests {
    use super::mulberry32;

    #[test]
    fn mulberry32_matches_the_js_sequence() {
        // First draws of the JS mulberry32(7), computed with node.
        let mut rand = mulberry32(7);
        let got: Vec<f64> =
            (0..3).map(|_| rand()).collect();
        assert_eq!(
            got,
            [
                0.011_704_753_153_026_104,
                0.061_958_257_574_588_06,
                0.976_907_632_779_33
            ]
        );
    }
}
