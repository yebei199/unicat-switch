//! The art in `../art/`, embedded and decoded once (the `pixels()` loader in `src/main.ts`).

use std::sync::LazyLock;

use crate::raster::Pixels;

pub struct Art {
    pub scene: Pixels,
    pub sit: Pixels,
    pub walk: Pixels,
}

pub static ART: LazyLock<Art> = LazyLock::new(|| Art {
    scene: decode(include_bytes!("../../art/scene.png")),
    sit: decode(include_bytes!("../../art/cat-sit.png")),
    walk: decode(include_bytes!("../../art/cat-walk.png")),
});

/// Any PNG (the art is 4/8-bit palette with tRNS) to straight RGBA8.
fn decode(bytes: &[u8]) -> Pixels {
    let mut decoder =
        png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(
        png::Transformations::normalize_to_color8()
            | png::Transformations::ALPHA,
    );
    let mut reader = decoder
        .read_info()
        .expect("embedded art is a valid PNG");
    let mut buf = vec![
        0;
        reader
            .output_buffer_size()
            .expect("art fits in memory")
    ];
    let info = reader
        .next_frame(&mut buf)
        .expect("embedded art decodes");
    let (width, height) =
        (info.width as usize, info.height as usize);
    let data = match info.color_type {
        png::ColorType::Rgba => {
            buf[..width * height * 4].to_vec()
        }
        png::ColorType::GrayscaleAlpha => buf
            [..width * height * 2]
            .chunks(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        other => panic!(
            "unexpected decoded color type {other:?}"
        ),
    };
    Pixels {
        width,
        height,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::ART;
    use crate::layout::{H, W};

    #[test]
    fn art_decodes_to_expected_sizes() {
        assert_eq!(
            (ART.scene.width, ART.scene.height),
            (W, H)
        );
        assert_eq!(
            (ART.sit.width, ART.sit.height),
            (12, 18)
        );
        assert_eq!(
            (ART.walk.width, ART.walk.height),
            (76, 15)
        );
    }
}
