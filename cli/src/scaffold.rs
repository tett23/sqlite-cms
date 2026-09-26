use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Post,
    Article,
    Page,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "post" => Some(Self::Post),
            "article" => Some(Self::Article),
            "page" => Some(Self::Page),
            _ => None,
        }
    }

    fn dir(self) -> &'static str {
        match self {
            Self::Post => "posts",
            Self::Article => "articles",
            Self::Page => "pages",
        }
    }
}

pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() || !slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        bail!("slug {slug:?} には文字、数字、-、_ だけを使ってください（URL とファイル名になります）");
    }
    Ok(())
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn template(kind: Kind, title: &str, date: &str) -> String {
    let title = quote(title);
    match kind {
        Kind::Post => format!("---\ntitle: {title}\ndate: {date}\n---\n\n"),
        Kind::Article => format!("---\ntitle: {title}\ndate: {date}\ndescription:\n---\n\n"),
        Kind::Page => format!("---\ntitle: {title}\n---\n\n"),
    }
}

pub fn create(site_dir: &Path, kind: Kind, slug: &str, title: &str, date: &str) -> Result<PathBuf> {
    validate_slug(slug)?;
    if title.trim().is_empty() {
        bail!("タイトルが空です");
    }
    if !site_dir.join("site.toml").is_file() {
        bail!(
            "{} に site.toml がありません。記事リポジトリの中で実行するか、SITE_DIR を指定してください",
            site_dir.display()
        );
    }

    let dir = site_dir.join("content").join(kind.dir());
    fs::create_dir_all(&dir).with_context(|| format!("{} を作れません", dir.display()))?;
    let path = dir.join(format!("{slug}.md"));

    let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            bail!("{} はすでにあります。別の slug を指定してください", path.display())
        }
        Err(e) => return Err(e).with_context(|| format!("{} を作れません", path.display())),
    };
    file.write_all(template(kind, title, date).as_bytes())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{parse_article, parse_page, parse_post};

    const TRICKY_TITLES: &[&str] = &[
        "ふつうのタイトル",
        "記事: その2",
        "\"引用\" と \\ と # と [括弧]",
        "[先頭が記号",
        "- ハイフン始まり",
        "null",
        "改行\nを含む",
    ];

    #[test]
    fn templates_round_trip_through_the_builder() {
        for title in TRICKY_TITLES {
            let post = parse_post("s", &template(Kind::Post, title, "2026-09-26")).unwrap();
            assert_eq!(post.title, *title);
            assert_eq!(post.published_at, "2026-09-26");

            let article = parse_article("s", &template(Kind::Article, title, "2026-09-26")).unwrap();
            assert_eq!(article.title, *title);
            assert_eq!(article.description, None);

            let page = parse_page("s", &template(Kind::Page, title, "2026-09-26")).unwrap();
            assert_eq!(page.title, *title);
        }
    }

    #[test]
    fn templates_have_empty_bodies() {
        assert_eq!(template(Kind::Post, "t", "2026-09-26"), "---\ntitle: \"t\"\ndate: 2026-09-26\n---\n\n");
        assert!(template(Kind::Article, "t", "2026-09-26").contains("\ndescription:\n"));
        assert!(!template(Kind::Page, "t", "2026-09-26").contains("date:"));
    }

    #[test]
    fn validates_slugs() {
        for ok in ["hello", "hello-world", "2026_09", "日本語の記事"] {
            assert!(validate_slug(ok).is_ok(), "{ok}");
        }
        for ng in ["", "a/b", "../x", ".hidden", "a b", "a?b", "a#b", "a%20b", "a.md"] {
            assert!(validate_slug(ng).is_err(), "{ng}");
        }
    }

    fn site() -> crate::testutil::TempDir {
        let dir = crate::testutil::tempdir();
        fs::write(dir.path().join("site.toml"), "title = \"t\"\n").unwrap();
        dir
    }

    #[test]
    fn creates_file_in_kind_directory() {
        let site = site();
        let path = create(site.path(), Kind::Article, "long-read", "長い読み物", "2026-09-26").unwrap();
        assert_eq!(path, site.path().join("content/articles/long-read.md"));
        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(parse_article("long-read", &written).unwrap().title, "長い読み物");
    }

    #[test]
    fn refuses_to_overwrite_existing_file() {
        let site = site();
        create(site.path(), Kind::Post, "hello", "一本目", "2026-09-26").unwrap();
        let err = create(site.path(), Kind::Post, "hello", "二本目", "2026-09-27").unwrap_err();
        assert!(err.to_string().contains("すでにあります"));
        let written = fs::read_to_string(site.path().join("content/posts/hello.md")).unwrap();
        assert!(written.contains("一本目"));
    }

    #[test]
    fn same_slug_in_different_kinds_is_allowed() {
        let site = site();
        create(site.path(), Kind::Post, "about", "t", "2026-09-26").unwrap();
        create(site.path(), Kind::Page, "about", "t", "2026-09-26").unwrap();
    }

    #[test]
    fn requires_site_toml() {
        let dir = crate::testutil::tempdir();
        let err = create(dir.path(), Kind::Post, "hello", "t", "2026-09-26").unwrap_err();
        assert!(err.to_string().contains("site.toml がありません"));
        assert!(!dir.path().join("content").exists());
    }

    #[test]
    fn rejects_empty_title_and_bad_slug() {
        let site = site();
        assert!(create(site.path(), Kind::Post, "hello", "  ", "2026-09-26").unwrap_err().to_string().contains("タイトル"));
        assert!(create(site.path(), Kind::Post, "../escape", "t", "2026-09-26").is_err());
        assert!(!site.path().join("content/escape.md").exists());
    }
}
