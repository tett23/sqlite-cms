use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::linkcard::{self, Fetcher};
use crate::{db, favicon, media, site};

pub const HEADERS: &str = "\
/db/*.sqlite
  Cache-Control: public, max-age=31536000, immutable

/db/manifest.json
  Cache-Control: no-cache
";

const MARKER: &str = ".sqlite-cms";

/// content/favicon.svg があればそれを、なければサイト名から作った仮の favicon を返す。
fn read_favicon(site_dir: &Path) -> Result<Vec<u8>> {
    let path = site_dir.join("content").join("favicon.svg");
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(favicon::placeholder(&site::read_site_config(site_dir)?.title).into_bytes())
        }
        Err(e) => Err(e).with_context(|| format!("{} を読めません", path.display())),
    }
}

pub struct SiteOutput {
    pub files: BTreeMap<String, Vec<u8>>,
}

impl SiteOutput {
    /// DB、マニフェスト、画像、favicon、リンクカードの画像を組み立てる。リンクカードの画像は fetcher で取得する。
    pub fn data(site_dir: &Path, fetcher: &dyn Fetcher) -> Result<Self> {
        let cards = linkcard::collect(site_dir, &db::markdown_sources(site_dir)?, fetcher);
        let bytes = db::build_db_bytes(site_dir, &cards)?;
        let db_path = format!("/db/{}", db::db_file_name(&bytes));

        let mut files = BTreeMap::new();
        files.insert("/db/manifest.json".to_string(), format!("{{\"db\":\"{db_path}\"}}\n").into_bytes());
        files.insert(db_path, bytes);
        for (path, bytes) in media::read_media(&site_dir.join("content").join("media"))? {
            files.insert(format!("/media/{path}"), bytes);
        }
        files.insert("/favicon.svg".to_string(), read_favicon(site_dir)?);
        for card in cards {
            files.insert(card.path, card.bytes);
        }
        Ok(Self { files })
    }

    pub fn site(site_dir: &Path, spa: &[(&str, &[u8])], fetcher: &dyn Fetcher) -> Result<Self> {
        if !spa.iter().any(|(path, _)| *path == "/index.html") {
            bail!(
                "この sqlite-cms は SPA を含まずにビルドされています。\
                 リリースのバイナリを使うか、開発者向けの手順（DEVELOPMENT.md）でビルドし直してください"
            );
        }
        let mut output = Self::data(site_dir, fetcher)?;
        for (path, bytes) in spa {
            output.files.entry(path.to_string()).or_insert_with(|| bytes.to_vec());
        }
        Ok(output)
    }

    pub fn total_bytes(&self) -> usize {
        self.files.values().map(Vec::len).sum()
    }

    pub fn write_site(&self, out_dir: &Path) -> Result<()> {
        if out_dir.exists() {
            let is_empty = fs::read_dir(out_dir)?.next().is_none();
            if !is_empty && !out_dir.join(MARKER).is_file() {
                bail!(
                    "{} は空ではなく、sqlite-cms の出力先でもありません。\
                     別のディレクトリを指定するか、中身を確かめてから消してください",
                    out_dir.display()
                );
            }
            fs::remove_dir_all(out_dir)?;
        }
        self.write_files(out_dir)?;
        fs::write(out_dir.join("_headers"), HEADERS)?;
        fs::write(out_dir.join(MARKER), "sqlite-cms の出力先。ビルドのたびに作り直される。\n")?;
        Ok(())
    }

    pub fn write_data(&self, out_dir: &Path) -> Result<()> {
        for dir in ["db", "media", linkcard::PATH_PREFIX.trim_matches('/')] {
            let dir = out_dir.join(dir);
            if dir.exists() {
                fs::remove_dir_all(&dir)?;
            }
        }
        self.write_files(out_dir)
    }

    fn write_files(&self, out_dir: &Path) -> Result<()> {
        for (path, bytes) in &self.files {
            let target = out_dir.join(path.trim_start_matches('/'));
            fs::create_dir_all(target.parent().unwrap())?;
            fs::write(&target, bytes).with_context(|| format!("{} に書き込めません", target.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPA: &[(&str, &[u8])] = &[("/index.html", b"<html>"), ("/assets/app.js", b"js")];

    fn site_fixture() -> crate::testutil::TempDir {
        let dir = crate::testutil::tempdir();
        fs::write(dir.path().join("site.toml"), "title = \"t\"\n").unwrap();
        fs::create_dir_all(dir.path().join("content/posts")).unwrap();
        fs::create_dir_all(dir.path().join("content/media")).unwrap();
        fs::write(dir.path().join("content/media/a.png"), b"png").unwrap();
        dir
    }

    fn paths(output: &SiteOutput) -> Vec<&str> {
        output.files.keys().map(String::as_str).collect()
    }

    fn files_under(dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in fs::read_dir(d).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.push(path.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn data_contains_db_manifest_and_media() {
        let site = site_fixture();
        let output = SiteOutput::data(site.path(), &linkcard::Offline).unwrap();
        let paths = paths(&output);
        assert_eq!(paths.len(), 4);
        assert!(paths[0].starts_with("/db/articles-"));
        assert_eq!(&paths[1..], ["/db/manifest.json", "/favicon.svg", "/media/a.png"]);
        let manifest = String::from_utf8(output.files["/db/manifest.json"].clone()).unwrap();
        assert_eq!(manifest, format!("{{\"db\":\"{}\"}}\n", paths[0]));
    }

    #[test]
    fn favicon_comes_from_content_or_falls_back_to_a_placeholder() {
        let site = site_fixture();
        let output = SiteOutput::data(site.path(), &linkcard::Offline).unwrap();
        assert_eq!(output.files["/favicon.svg"], crate::favicon::placeholder("t").into_bytes());

        fs::write(site.path().join("content/favicon.svg"), b"<svg>mine</svg>").unwrap();
        let output = SiteOutput::data(site.path(), &linkcard::Offline).unwrap();
        assert_eq!(output.files["/favicon.svg"], b"<svg>mine</svg>");
    }

    #[test]
    fn site_adds_spa_files() {
        let site = site_fixture();
        let output = SiteOutput::site(site.path(), SPA, &linkcard::Offline).unwrap();
        assert!(output.files.contains_key("/index.html"));
        assert!(output.files.contains_key("/assets/app.js"));
        assert!(output.files.contains_key("/db/manifest.json"));
    }

    #[test]
    fn site_without_spa_is_error() {
        let site = site_fixture();
        let err = SiteOutput::site(site.path(), &[], &linkcard::Offline).err().unwrap();
        assert!(err.to_string().contains("SPA を含まずに"));
    }

    #[test]
    fn write_site_creates_complete_output_with_headers_and_marker() {
        let site = site_fixture();
        let out = crate::testutil::tempdir();
        let out_dir = out.path().join("dist");
        SiteOutput::site(site.path(), SPA, &linkcard::Offline).unwrap().write_site(&out_dir).unwrap();

        let files = files_under(&out_dir);
        for expected in [".sqlite-cms", "_headers", "assets/app.js", "db/manifest.json", "favicon.svg", "index.html", "media/a.png"] {
            assert!(files.iter().any(|f| f == expected), "{expected} がありません: {files:?}");
        }
        assert_eq!(fs::read_to_string(out_dir.join("_headers")).unwrap(), HEADERS);
    }

    #[test]
    fn write_site_replaces_previous_output() {
        let site = site_fixture();
        let out = crate::testutil::tempdir();
        let out_dir = out.path().join("dist");
        let output = SiteOutput::site(site.path(), SPA, &linkcard::Offline).unwrap();
        output.write_site(&out_dir).unwrap();
        fs::write(out_dir.join("stale.js"), b"old").unwrap();

        output.write_site(&out_dir).unwrap();
        assert!(!out_dir.join("stale.js").exists());
    }

    #[test]
    fn write_site_refuses_foreign_non_empty_directory() {
        let site = site_fixture();
        let out = crate::testutil::tempdir();
        fs::write(out.path().join("important.txt"), b"keep").unwrap();

        let err = SiteOutput::site(site.path(), SPA, &linkcard::Offline).unwrap().write_site(out.path()).unwrap_err();
        assert!(err.to_string().contains("sqlite-cms の出力先でもありません"));
        assert_eq!(fs::read(out.path().join("important.txt")).unwrap(), b"keep");
    }

    #[test]
    fn write_data_replaces_only_db_and_media() {
        let site = site_fixture();
        let out = crate::testutil::tempdir();
        fs::create_dir_all(out.path().join("db")).unwrap();
        fs::write(out.path().join("db/articles-0000000000000000.sqlite"), b"old").unwrap();
        fs::write(out.path().join("keep.txt"), b"keep").unwrap();

        SiteOutput::data(site.path(), &linkcard::Offline).unwrap().write_data(out.path()).unwrap();

        let files = files_under(out.path());
        assert!(files.contains(&"keep.txt".to_string()));
        assert!(!files.contains(&"db/articles-0000000000000000.sqlite".to_string()));
        assert!(files.contains(&"db/manifest.json".to_string()));
        assert!(files.contains(&"media/a.png".to_string()));
    }
}
