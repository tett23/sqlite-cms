//! リンクカードの画像を、カードの大きさに縮めて WebP にする（ADR 0049）。
//!
//! カードの画像は 7.5rem × 3.9rem（120 × 62 px）で表示する。高精細の画面に合わせて、その 2 倍の大きさにする。
//! OGP の画像は 1200 × 630 px ほどの大きさがあり、そのまま配信すると 1 枚で 100 KB を超えることがある。

use std::io::Cursor;

use anyhow::{Context, Result};
use image::imageops::FilterType;
use image::{ImageReader, Limits};

/// 配信する画像の大きさ（px）。OGP の画像に多い 1.91:1 に近づける。
pub const WIDTH: u32 = 240;
pub const HEIGHT: u32 = 126;

/// WebP の品質（0〜100）。小さく縮めた画像では、80 で見た目の違いはほとんどわからない。
const QUALITY: f32 = 80.0;

/// 読む画像の縦と横の上限（px）。ファイルは小さくても、展開すると大きな画像で、メモリを使い果たさないようにする。
const MAX_DIMENSION: u32 = 8192;

/// 画像を読み、中央を切り抜いてカードの大きさに縮め、WebP にする。
pub fn to_webp(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().context("画像の形式を判別できません")?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    reader.limits(limits);
    let image = reader.decode().context("画像を読めません")?;
    // 縦横比が違えば、中央を切り抜く（表示の object-fit: cover と同じ）。
    let card = image.resize_to_fill(WIDTH, HEIGHT, FilterType::Lanczos3).to_rgba8();
    Ok(webp::Encoder::from_rgba(&card, WIDTH, HEIGHT).encode(QUALITY).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    /// 左半分が赤、右半分が青の画像を、指定した形式で書き出す。
    fn encoded(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let image = RgbaImage::from_fn(width, height, |x, _| if x < width / 2 { Rgba([255, 0, 0, 255]) } else { Rgba([0, 0, 255, 255]) });
        let image = match format {
            ImageFormat::Jpeg => DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(image).to_rgb8()),
            _ => DynamicImage::ImageRgba8(image),
        };
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    fn decode(webp: &[u8]) -> RgbaImage {
        assert!(webp.starts_with(b"RIFF") && &webp[8..12] == b"WEBP", "WebP ではありません");
        image::load_from_memory_with_format(webp, ImageFormat::WebP).unwrap().to_rgba8()
    }

    #[test]
    fn converts_png_jpeg_gif_and_webp_to_a_card_sized_webp() {
        for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Gif, ImageFormat::WebP] {
            let card = decode(&to_webp(&encoded(1200, 630, format)).unwrap());
            assert_eq!(card.dimensions(), (WIDTH, HEIGHT), "{format:?}");
            // 左は赤、右は青のまま（非可逆なので、色は近いことだけを確かめる）。
            let left = card.get_pixel(10, HEIGHT / 2);
            let right = card.get_pixel(WIDTH - 10, HEIGHT / 2);
            assert!(left[0] > 200 && left[2] < 60, "{format:?} 左 {left:?}");
            assert!(right[2] > 200 && right[0] < 60, "{format:?} 右 {right:?}");
        }
    }

    #[test]
    fn crops_the_center_when_the_aspect_ratio_differs() {
        // とても横に長い画像は、中央（赤と青の境目のあたり）を切り抜く。
        let card = decode(&to_webp(&encoded(4000, 100, ImageFormat::Png)).unwrap());
        assert_eq!(card.dimensions(), (WIDTH, HEIGHT));
        assert!(card.get_pixel(5, HEIGHT / 2)[0] > 200, "左端は赤");
        assert!(card.get_pixel(WIDTH - 5, HEIGHT / 2)[2] > 200, "右端は青");
        // 縦に長い画像も、カードの大きさになる。
        assert_eq!(decode(&to_webp(&encoded(100, 1000, ImageFormat::Png)).unwrap()).dimensions(), (WIDTH, HEIGHT));
        // 小さな画像は、引き伸ばす。
        assert_eq!(decode(&to_webp(&encoded(30, 20, ImageFormat::Png)).unwrap()).dimensions(), (WIDTH, HEIGHT));
    }

    #[test]
    fn keeps_transparency() {
        let mut out = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(600, 315, Rgba([0, 128, 0, 0]))).write_to(&mut out, ImageFormat::Png).unwrap();
        let card = decode(&to_webp(&out.into_inner()).unwrap());
        assert!(card.get_pixel(WIDTH / 2, HEIGHT / 2)[3] < 10, "透明のまま");
    }

    #[test]
    fn rejects_unreadable_and_huge_images() {
        assert!(to_webp(b"not an image").is_err());
        assert!(to_webp(b"\x89PNG\r\n\x1a\n broken").is_err());
        // 縦横の上限を超える画像は、展開する前に断る。
        let huge = encoded(MAX_DIMENSION + 1, 1, ImageFormat::Png);
        assert!(format!("{:#}", to_webp(&huge).unwrap_err()).contains("画像を読めません"));
    }
}
