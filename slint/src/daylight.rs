//! Port of `src/daylight.ts`: every lighting parameter as a function of `t`
//! (0 day → 0.5 dusk → 1 night).

use crate::color::{Rgb, hex, mix, smoothstep};

#[derive(Clone, Copy, Debug)]
pub struct Daylight {
    pub sky_top: Rgb,
    pub sky_horizon: Rgb,
    pub light: Rgb,
    pub dusk: f64,
    pub night: f64,
}

#[derive(Clone, Copy)]
struct Key {
    sky_top: Rgb,
    sky_horizon: Rgb,
    light: Rgb,
}

const DAY: Key = Key {
    sky_top: hex(0x79a6da),
    sky_horizon: hex(0xe6eef0),
    light: [1.0, 1.0, 1.0],
};
const DUSK: Key = Key {
    sky_top: hex(0x4b5486),
    sky_horizon: hex(0xf3a447),
    light: [1.0, 0.74, 0.6],
};
const NIGHT: Key = Key {
    sky_top: hex(0x090d27),
    sky_horizon: hex(0x1d2350),
    light: [0.27, 0.31, 0.55],
};

fn between(a: Key, b: Key, u: f64) -> Key {
    Key {
        sky_top: mix(a.sky_top, b.sky_top, u),
        sky_horizon: mix(a.sky_horizon, b.sky_horizon, u),
        light: mix(a.light, b.light, u),
    }
}

pub fn daylight(t: f64) -> Daylight {
    let key = if t < 0.5 {
        between(DAY, DUSK, smoothstep(0.0, 0.5, t))
    } else {
        between(DUSK, NIGHT, smoothstep(0.5, 1.0, t))
    };
    Daylight {
        sky_top: key.sky_top,
        sky_horizon: key.sky_horizon,
        light: key.light,
        dusk: 1.0 - (2.0 * t - 1.0).abs(),
        night: smoothstep(0.55, 0.95, t),
    }
}
