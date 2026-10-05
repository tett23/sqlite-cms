use std::fs;
use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};

pub fn read_media(src: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut files = Vec::new();
    if src.is_dir() {
        read_dir(src, "", &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn read_dir(dir: &Path, prefix: &str, files: &mut Vec<(String, Vec<u8>)>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_str().with_context(|| format!("{} のファイル名が UTF-8 ではありません", dir.display()))?;
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let relative = format!("{prefix}{name}");
        if path.is_dir() {
            read_dir(&path, &format!("{relative}/"), files)?;
        } else {
            let bytes = fs::read(&path).with_context(|| format!("{} を読めません", path.display()))?;
            files.push((relative, bytes));
        }
    }
    Ok(())
}

/// 本文の画像の大きさ（ADR 0051）。`path` は配信するパス（`/media/…`）。
#[derive(Debug, PartialEq)]
pub struct MediaSize {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

/// 読んだ画像（`read_media` の結果）のうち、大きさの分かるものの大きさ。画像でないファイルや読めない画像は除く。
pub fn sizes(files: &[(String, Vec<u8>)]) -> Vec<MediaSize> {
    files
        .iter()
        .filter_map(|(path, bytes)| {
            let (width, height) = dimensions(path, bytes)?;
            Some(MediaSize { path: format!("/media/{path}"), width, height })
        })
        .collect()
}

/// 画像の縦横の大きさ（px）。PNG、JPEG、GIF、WebP はヘッダを読み、SVG はルートの width と height か viewBox を読む。
fn dimensions(path: &str, bytes: &[u8]) -> Option<(u32, u32)> {
    let (width, height) = if path.to_ascii_lowercase().ends_with(".svg") {
        svg_dimensions(std::str::from_utf8(bytes).ok()?)?
    } else {
        image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok()?.into_dimensions().ok()?
    };
    (width > 0 && height > 0).then_some((width, height))
}

/// SVG のルートの要素の width と height（単位なしか px）。なければ viewBox の幅と高さ。
fn svg_dimensions(svg: &str) -> Option<(u32, u32)> {
    let start = svg.find("<svg")?;
    let tag = &svg[start..start + svg[start..].find('>')?];
    let px = |value: &str| value.trim().trim_end_matches("px").parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0);
    if let (Some(width), Some(height)) = (attribute(tag, "width").and_then(px), attribute(tag, "height").and_then(px)) {
        return Some((width.round() as u32, height.round() as u32));
    }
    let numbers: Vec<f64> = attribute(tag, "viewBox")?
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().ok())
        .collect::<Option<_>>()?;
    match numbers[..] {
        [_, _, width, height] if width > 0.0 && height > 0.0 => Some((width.round() as u32, height.round() as u32)),
        _ => None,
    }
}

/// 開始タグ（`<svg …`）の属性の値。
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(index) = rest.find(name) {
        let before = rest[..index].chars().next_back();
        let after = rest[index + name.len()..].trim_start();
        rest = &rest[index + name.len()..];
        if !before.is_some_and(char::is_whitespace) {
            continue;
        }
        let Some(after) = after.strip_prefix('=') else { continue };
        let after = after.trim_start();
        let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let value = &after[1..];
        return Some(&value[..value.find(quote)?]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_files_and_skips_dotfiles() {
        let tmp = crate::testutil::tempdir();
        let src = tmp.path().join("media");
        fs::create_dir_all(src.join("2026")).unwrap();
        fs::write(src.join("a.png"), b"a").unwrap();
        fs::write(src.join("2026/b.jpg"), b"b").unwrap();
        fs::write(src.join(".DS_Store"), b"x").unwrap();
        fs::create_dir_all(src.join(".hidden")).unwrap();
        fs::write(src.join(".hidden/c.png"), b"c").unwrap();

        assert_eq!(
            read_media(&src).unwrap(),
            [("2026/b.jpg".to_string(), b"b".to_vec()), ("a.png".to_string(), b"a".to_vec())]
        );
    }

    #[test]
    fn missing_source_is_empty() {
        let tmp = crate::testutil::tempdir();
        assert!(read_media(&tmp.path().join("media")).unwrap().is_empty());
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::RgbaImage::new(width, height).write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png).unwrap();
        bytes
    }

    #[test]
    fn sizes_of_raster_and_svg_images() {
        let files = [
            ("a.png".to_string(), png(3, 2)),
            ("拡張子なし".to_string(), png(5, 4)),
            ("b.svg".to_string(), br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" width="640px" height="150" viewBox="0 0 1 1">"#.to_vec()),
            ("c.SVG".to_string(), b"<svg viewBox='0,0, 480.4 300.6'><rect width=\"1\" height=\"1\"/></svg>".to_vec()),
            ("note.txt".to_string(), b"text".to_vec()),
            ("broken.png".to_string(), b"\x89PNG broken".to_vec()),
        ];
        assert_eq!(
            sizes(&files),
            [
                MediaSize { path: "/media/a.png".into(), width: 3, height: 2 },
                MediaSize { path: "/media/拡張子なし".into(), width: 5, height: 4 },
                MediaSize { path: "/media/b.svg".into(), width: 640, height: 150 },
                MediaSize { path: "/media/c.SVG".into(), width: 480, height: 301 },
            ]
        );
    }

    #[test]
    fn svg_without_usable_size_is_skipped() {
        for svg in [
            r#"<svg width="100%" height="50%">"#,
            r#"<svg width="10em" height="5em">"#,
            r#"<svg data-width="10" data-height="5">"#,
            r#"<svg viewBox="0 0 0 10">"#,
            r#"<svg viewBox="0 0 10">"#,
            r#"<svg>"#,
            r#"<html>"#,
            r#"<svg width="10" height="5""#,
        ] {
            assert_eq!(svg_dimensions(svg), None, "{svg}");
        }
        // width と height の片方だけなら viewBox を使う。
        assert_eq!(svg_dimensions(r#"<svg width="10" viewBox="0 0 30 20">"#), Some((30, 20)));
        // stroke-width などの別の属性は読まない。
        assert_eq!(svg_dimensions(r#"<svg stroke-width="3" width="8" height="6">"#), Some((8, 6)));
    }
}
