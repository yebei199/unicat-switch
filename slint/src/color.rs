//! Port of `src/color.ts`: linear RGB triples and the scalar helpers everything else shares.

pub type Rgb = [f64; 3];

pub fn lerp(a: f64, b: f64, u: f64) -> f64 {
    a + (b - a) * u
}

pub fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let u = clamp01((x - e0) / (e1 - e0));
    u * u * (3.0 - 2.0 * u)
}

pub fn mix(a: Rgb, b: Rgb, u: f64) -> Rgb {
    [
        lerp(a[0], b[0], u),
        lerp(a[1], b[1], u),
        lerp(a[2], b[2], u),
    ]
}

/// `0xRRGGBB` as three channels in 0..=1.
pub const fn hex(n: u32) -> Rgb {
    [
        ((n >> 16) & 255) as f64 / 255.0,
        ((n >> 8) & 255) as f64 / 255.0,
        (n & 255) as f64 / 255.0,
    ]
}

/// `Math.round`: halves round toward +∞ (Rust's `round` goes away from zero).
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}
