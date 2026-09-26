use std::fs;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_files_and_skips_dotfiles() {
        let tmp = tempfile::tempdir().unwrap();
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
        let tmp = tempfile::tempdir().unwrap();
        assert!(read_media(&tmp.path().join("media")).unwrap().is_empty());
    }
}
