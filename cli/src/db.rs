use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, MAIN_DB};

use crate::content::{parse_article, parse_page, parse_post};
use crate::linkcard::LinkCard;
use crate::migrations::{apply_migrations, embedded_migrations};
use crate::site::read_site_config;

fn read_docs<T>(dir: &Path, parse: impl Fn(&str, &str) -> Result<T>) -> Result<Vec<T>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    paths
        .iter()
        .map(|path| {
            let slug = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            let raw = fs::read_to_string(path).with_context(|| format!("{} を読めません", path.display()))?;
            parse(slug, &raw)
        })
        .collect()
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("{} を読めません", path.display())),
    }
}

/// 本文の Markdown をそのまま集める（トップページ、post、article、page）。リンクカードの URL を探すのに使う。
pub fn markdown_sources(site_dir: &Path) -> Result<Vec<String>> {
    let content_dir = site_dir.join("content");
    let mut sources: Vec<String> = read_optional(&content_dir.join("index.md"))?.into_iter().collect();
    for kind in ["posts", "articles", "pages"] {
        sources.extend(read_docs(&content_dir.join(kind), |_, raw| Ok(raw.to_string()))?);
    }
    Ok(sources)
}

pub fn build_db_bytes(site_dir: &Path, link_cards: &[LinkCard]) -> Result<Vec<u8>> {
    let config = read_site_config(site_dir)?;

    let content_dir = site_dir.join("content");
    if !content_dir.is_dir() {
        bail!("コンテンツのディレクトリがありません: {}", content_dir.display());
    }
    let home_md = read_optional(&content_dir.join("index.md"))?;

    let conn = Connection::open_in_memory()?;
    apply_migrations(&conn, &embedded_migrations()?)?;

    let (license_name, license_url) = match &config.license {
        Some(license) => (Some(&license.name), license.url.as_ref()),
        None => (None, None),
    };
    conn.execute(
        "INSERT INTO site (id, title, author, license_name, license_url, home_md, description)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
        (&config.title, &config.author, license_name, license_url, &home_md, config.description()),
    )?;

    for p in read_docs(&content_dir.join("posts"), parse_post)? {
        conn.execute(
            "INSERT INTO posts (slug, title, published_at, body_md) VALUES (?1, ?2, ?3, ?4)",
            (&p.slug, &p.title, &p.published_at, &p.body_md),
        )?;
    }

    for a in read_docs(&content_dir.join("articles"), parse_article)? {
        conn.execute(
            "INSERT INTO articles (slug, title, published_at, updated_at, description, body_md)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (&a.slug, &a.title, &a.published_at, &a.updated_at, &a.description, &a.body_md),
        )?;
    }

    for p in read_docs(&content_dir.join("pages"), parse_page)? {
        conn.execute(
            "INSERT INTO pages (slug, title, body_md) VALUES (?1, ?2, ?3)",
            (&p.slug, &p.title, &p.body_md),
        )?;
    }

    for card in link_cards {
        conn.execute("INSERT INTO link_cards (url, image_path) VALUES (?1, ?2)", (&card.url, &card.path))?;
    }

    Ok(conn.serialize(MAIN_DB)?.to_vec())
}

pub fn db_file_name(bytes: &[u8]) -> String {
    format!("articles-{}.sqlite", &blake3::hash(bytes).to_hex()[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    const POST: &str = "---\ntitle: テスト記事\ndate: 2026-09-17\n---\n\n本文の**段落**。\n";
    const ARTICLE: &str = "---\ntitle: 長い読み物\ndate: 2026-09-15\nupdated: 2026-09-16\ndescription: 一覧に出す要約。\n---\n\n本文。\n";
    const PAGE: &str = "---\ntitle: 自己紹介\n---\n\n自己紹介の本文。\n";

    const SITE: &str = "title = \"記事置き場\"\nauthor = \"tett23\"\n\n[license]\nname = \"CC0 1.0\"\n";

    fn site_fixture(content: &[(&str, &str)]) -> crate::testutil::TempDir {
        let mut files = vec![("site.toml".to_string(), SITE.to_string())];
        files.extend(content.iter().map(|(name, raw)| (format!("content/{name}"), raw.to_string())));
        fixture(&files)
    }

    fn fixture(files: &[(String, String)]) -> crate::testutil::TempDir {
        let dir = crate::testutil::tempdir();
        fs::create_dir_all(dir.path().join("content")).unwrap();
        for (name, raw) in files {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, raw).unwrap();
        }
        dir
    }

    fn open(bytes: &[u8]) -> (crate::testutil::TempDir, Connection) {
        let dir = crate::testutil::tempdir();
        let path = dir.path().join("db.sqlite");
        fs::write(&path, bytes).unwrap();
        let conn = Connection::open(&path).unwrap();
        (dir, conn)
    }

    fn column(conn: &Connection, sql: &str) -> Vec<String> {
        conn.prepare(sql)
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn three_content_types_go_into_their_tables() {
        let second = POST.replace("テスト記事", "二本目");
        let site = site_fixture(&[
            ("posts/a.md", POST),
            ("posts/b.md", &second),
            ("posts/note.txt", "md 以外は無視"),
            ("articles/long.md", ARTICLE),
            ("pages/about.md", PAGE),
        ]);
        let bytes = build_db_bytes(site.path(), &[]).unwrap();

        let (_dir, conn) = open(&bytes);
        assert_eq!(column(&conn, "SELECT slug FROM posts ORDER BY slug"), ["a", "b"]);
        assert_eq!(column(&conn, "SELECT description FROM articles"), ["一覧に出す要約。"]);
        assert_eq!(column(&conn, "SELECT slug FROM pages"), ["about"]);
        assert_eq!(
            column(&conn, "SELECT body_md FROM posts WHERE slug = 'a'"),
            ["\n本文の**段落**。\n"]
        );
        assert_eq!(
            column(&conn, "SELECT version FROM schema_migrations ORDER BY version"),
            ["0001", "0002", "0003", "0004", "0005", "0006"]
        );
    }

    #[test]
    fn link_cards_go_into_their_table_and_sources_include_every_body() {
        let site = site_fixture(&[("posts/a.md", POST), ("index.md", "https://example.com/home\n")]);
        let cards = [LinkCard { url: "https://example.com/".into(), path: "/link-cards/abc.png".into(), bytes: vec![1] }];
        let (_dir, conn) = open(&build_db_bytes(site.path(), &cards).unwrap());
        assert_eq!(column(&conn, "SELECT url || ' ' || image_path FROM link_cards"), ["https://example.com/ /link-cards/abc.png"]);

        let sources = markdown_sources(site.path()).unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources[0].contains("https://example.com/home"));
        assert!(sources[1].contains("本文の**段落**"));
    }

    #[test]
    fn missing_content_directories_are_treated_as_empty() {
        let site = site_fixture(&[("posts/a.md", POST)]);
        let bytes = build_db_bytes(site.path(), &[]).unwrap();
        let (_dir, conn) = open(&bytes);
        assert!(column(&conn, "SELECT slug FROM articles").is_empty());
    }

    #[test]
    fn site_metadata_and_home_go_into_site_table() {
        let site = site_fixture(&[("index.md", "トップの**導入**。\n")]);
        let bytes = build_db_bytes(site.path(), &[]).unwrap();
        let (_dir, conn) = open(&bytes);
        type Row = (String, Option<String>, Option<String>, Option<String>, Option<String>);
        let row: Row = conn
            .query_row(
                "SELECT title, author, license_name, license_url, home_md FROM site WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(
            row,
            (
                "記事置き場".into(),
                Some("tett23".into()),
                Some("CC0 1.0".into()),
                None,
                Some("トップの**導入**。\n".into())
            )
        );
    }

    #[test]
    fn description_is_stored_with_the_default_sentence() {
        let site = site_fixture(&[]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[]).unwrap());
        let description: String = conn.query_row("SELECT description FROM site", [], |r| r.get(0)).unwrap();
        assert_eq!(description, "記事置き場。記事とブログを置いているサイトです。");
    }

    #[test]
    fn home_is_optional() {
        let site = site_fixture(&[]);
        let bytes = build_db_bytes(site.path(), &[]).unwrap();
        let (_dir, conn) = open(&bytes);
        let home: Option<String> = conn.query_row("SELECT home_md FROM site", [], |r| r.get(0)).unwrap();
        assert_eq!(home, None);
    }

    #[test]
    fn missing_site_toml_is_error() {
        let dir = crate::testutil::tempdir();
        fs::create_dir_all(dir.path().join("content")).unwrap();
        let err = build_db_bytes(dir.path(), &[]).unwrap_err();
        assert!(err.to_string().contains("site.toml を読めません"));
    }

    #[test]
    fn missing_content_root_is_error() {
        let dir = crate::testutil::tempdir();
        fs::write(dir.path().join("site.toml"), SITE).unwrap();
        let err = build_db_bytes(dir.path(), &[]).unwrap_err();
        assert!(err.to_string().contains("コンテンツのディレクトリがありません"));
    }

    #[test]
    fn invalid_document_fails_the_build() {
        let site = site_fixture(&[("posts/bad.md", "---\ntitle: t\n---\n本文")]);
        let err = build_db_bytes(site.path(), &[]).unwrap_err();
        assert!(err.to_string().contains("date"));
    }

    #[test]
    fn same_input_yields_same_file_name() {
        let site = site_fixture(&[("posts/a.md", POST), ("articles/long.md", ARTICLE)]);
        let first = db_file_name(&build_db_bytes(site.path(), &[]).unwrap());
        let second = db_file_name(&build_db_bytes(site.path(), &[]).unwrap());
        assert_eq!(first, second);
        assert!(first.starts_with("articles-") && first.ends_with(".sqlite"));
        assert_eq!(first.len(), "articles-".len() + 16 + ".sqlite".len());
    }
}
