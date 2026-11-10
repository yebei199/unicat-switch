//! Port of the `.switch` button in `src/style.css` and `theme()` in `src/main.ts`: the pill bezel
//! around the canvas, a top-to-bottom gradient whose colours follow the sky phase, with the two
//! inset shadows. Drawn on the CPU because the software renderer cannot fill a rounded rectangle
//! with a gradient. (The outer drop shadow and night glow stay in the .slint as drop shadows.)

use crate::color::{Rgb, hex, mix, smoothstep};

const HI: [Rgb; 2] = [hex(0xfdfaf3), hex(0x3b3b55)];
const LO: [Rgb; 2] = [hex(0xd8cfbc), hex(0x17182b)];
/// The CSS was written for a 540px-wide switch; its pixel lengths scale with the bezel.
const CSS_WIDTH: f64 = 540.0;

/// Signed distance to a `width`×`height` pill (negative inside).
fn pill(x: f64, y: f64, width: f64, height: f64) -> f64 {
    let r = height * 0.5;
    let qx = (x - width * 0.5).abs() - (width * 0.5 - r);
    let qy = (y - height * 0.5).abs();
    let (ox, oy) = (qx.max(0.0), qy.max(0.0));
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

/// `inset 0 <dy> <blur> colour`: strength where the box shifted by `dy` no longer covers.
fn inset(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    dy: f64,
    blur: f64,
) -> f64 {
    smoothstep(-blur, blur, pill(x, y - dy, width, height))
}

fn to_u8(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Fills `out` (`width`×`height` straight-alpha RGBA8) with the bezel at sky phase `t`, except
/// where the canvas inset by `pad` covers it completely (the canvas is drawn over it next).
pub fn draw(
    width: usize,
    height: usize,
    pad: usize,
    t: f64,
    out: &mut [u8],
) {
    let (w, h) = (width as f64, height as f64);
    let (p, cw, ch) = (
        pad as f64,
        w - 2.0 * pad as f64,
        h - 2.0 * pad as f64,
    );
    let unit = w / CSS_WIDTH;
    let (hi, lo) =
        (mix(HI[0], HI[1], t), mix(LO[0], LO[1], t));
    let rows = out
        .as_chunks_mut::<4>()
        .0
        .chunks_exact_mut(width)
        .take(height);
    for (row_index, row) in rows.enumerate() {
        let y = row_index as f64 + 0.5;
        // The pill is mirror-symmetric, so the covered run is centred: find where it starts.
        let covered = (0..width / 2).find(|&x| {
            pill(x as f64 + 0.5 - p, y - p, cw, ch) <= -0.5
        });
        let skip =
            covered.map_or(0..0, |edge| edge..width - edge);
        for (x, px) in row.iter_mut().enumerate() {
            if skip.contains(&x) {
                continue;
            }
            let x = x as f64 + 0.5;
            let mask =
                (0.5 - pill(x, y, w, h)).clamp(0.0, 1.0);
            if mask <= 0.0 {
                *px = [0; 4];
                continue;
            }
            let mut c = mix(hi, lo, y / h);
            c = mix(
                c,
                [1.0; 3],
                0.5 * inset(x, y, w, h, 2.0 * unit, unit),
            );
            c = mix(
                c,
                [0.0; 3],
                0.15 * inset(
                    x,
                    y,
                    w,
                    h,
                    -3.0 * unit,
                    6.0 * unit,
                ),
            );
            *px = [
                to_u8(c[0]),
                to_u8(c[1]),
                to_u8(c[2]),
                to_u8(mask),
            ];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezel_is_a_pill_lit_on_top_and_leaves_the_canvas_alone()
     {
        let (w, h) = (270, 123);
        let mut out = vec![7; w * h * 4];
        draw(w, h, 6, 0.0, &mut out);
        let at = |x: usize, y: usize| {
            &out[(y * w + x) * 4..(y * w + x) * 4 + 4]
        };
        assert_eq!(
            at(0, 0)[3],
            0,
            "corners are outside the pill"
        );
        assert_eq!(
            at(w / 2, 3)[3],
            255,
            "the ring is opaque"
        );
        assert_eq!(
            at(w / 2, h / 2),
            [7; 4],
            "under the canvas is untouched"
        );
        assert!(
            at(w / 2, 1)[0] > at(w / 2, h - 2)[0],
            "the top is lighter than the bottom"
        );
    }
}
