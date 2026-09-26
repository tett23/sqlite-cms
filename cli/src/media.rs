use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub fn copy_media(src: &Path, out_dir: &Path) -> Result<usize> {
    if out_dir.exists() {
        fs::remove_dir_all(out_dir)?;
    }
    if !src.is_dir() {
        return Ok(0);
    }
    copy_dir(src, out_dir)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<usize> {
    fs::create_dir_all(dst)?;
    let mut count = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        let target = dst.join(&name);
        if path.is_dir() {
            count += copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target).with_context(|| format!("{} をコピーできません", path.display()))?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files_under(dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.extend(files_under(&path));
            } else {
                out.push(path.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
            }
        }
        out
    }

    #[test]
    fn copies_nested_files_and_skips_dotfiles() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("media");
        fs::create_dir_all(src.join("2026")).unwrap();
        fs::write(src.join("a.png"), b"a").unwrap();
        fs::write(src.join("2026/b.jpg"), b"b").unwrap();
        fs::write(src.join(".DS_Store"), b"x").unwrap();

        let out = tmp.path().join("public/media");
        assert_eq!(copy_media(&src, &out).unwrap(), 2);
        assert_eq!(fs::read(out.join("a.png")).unwrap(), b"a");
        assert_eq!(fs::read(out.join("2026/b.jpg")).unwrap(), b"b");
        assert!(!out.join(".DS_Store").exists());
    }

    #[test]
    fn removes_files_left_from_previous_build() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("media");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("new.png"), b"n").unwrap();
        let out = tmp.path().join("public/media");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("old.png"), b"o").unwrap();

        copy_media(&src, &out).unwrap();
        assert_eq!(files_under(&out), ["new.png"]);
    }

    #[test]
    fn missing_source_leaves_no_output() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("public/media");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("old.png"), b"o").unwrap();

        assert_eq!(copy_media(&tmp.path().join("media"), &out).unwrap(), 0);
        assert!(!out.exists());
    }
}
