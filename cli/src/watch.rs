//! serve で記事の変更を見つける（ADR 0034）。
//!
//! 依存を足さず、決まった間隔で site.toml と content/ のファイルの一覧、大きさ、更新時刻を調べ、指紋（ハッシュ）が変わったかで判断する。
//! OS の変更の通知（FSEvents、inotify）は使わない。記事リポジトリのファイルは少なく、調べるのは速い。

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

fn visit(path: &Path, out: &mut Vec<(PathBuf, u64, u128)>) {
    let Ok(metadata) = fs::metadata(path) else { return };
    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(path) else { return };
        for entry in entries.flatten() {
            visit(&entry.path(), out);
        }
    } else {
        let modified = metadata.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
        out.push((path.to_path_buf(), metadata.len(), modified));
    }
}

/// site.toml と content/ の中のファイルの一覧、大きさ、更新時刻から作る指紋。
pub fn fingerprint(site_dir: &Path) -> u64 {
    let mut files = Vec::new();
    visit(&site_dir.join("site.toml"), &mut files);
    visit(&site_dir.join("content"), &mut files);
    files.sort();
    let mut hasher = DefaultHasher::new();
    files.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_changes_when_files_are_added_changed_or_removed() {
        let site = crate::testutil::tempdir();
        fs::write(site.path().join("site.toml"), "title = \"t\"\n").unwrap();
        fs::create_dir_all(site.path().join("content/posts")).unwrap();
        let initial = fingerprint(site.path());
        assert_eq!(fingerprint(site.path()), initial);

        let post = site.path().join("content/posts/a.md");
        fs::write(&post, "a").unwrap();
        let added = fingerprint(site.path());
        assert_ne!(added, initial);

        fs::write(&post, "ab").unwrap();
        let changed = fingerprint(site.path());
        assert_ne!(changed, added);

        fs::remove_file(&post).unwrap();
        assert_eq!(fingerprint(site.path()), initial);

        // content/ の外（キャッシュなど）の変更は数えない。
        fs::create_dir_all(site.path().join(".sqlite-cms-cache")).unwrap();
        fs::write(site.path().join(".sqlite-cms-cache/x"), "x").unwrap();
        assert_eq!(fingerprint(site.path()), initial);
    }
}
