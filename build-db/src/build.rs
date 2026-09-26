use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, MAIN_DB};
use sha2::{Digest, Sha256};

use crate::content::{parse_article, parse_page, parse_post};
use crate::migrations::{apply_migrations, read_migrations};

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

pub fn build_db_bytes(content_dir: &Path, migrations_dir: &Path) -> Result<Vec<u8>> {
    let conn = Connection::open_in_memory()?;
    apply_migrations(&conn, &read_migrations(migrations_dir)?)?;

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

    Ok(conn.serialize(MAIN_DB)?.to_vec())
}

pub fn db_file_name(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    format!("articles-{hex}.sqlite")
}

pub fn write_output(out_dir: &Path, bytes: &[u8]) -> Result<String> {
    let name = db_file_name(bytes);
    if out_dir.exists() {
        fs::remove_dir_all(out_dir)?;
    }
    fs::create_dir_all(out_dir)?;
    fs::write(out_dir.join(&name), bytes)?;
    fs::write(out_dir.join("manifest.json"), format!("{{\"db\":\"/db/{name}\"}}\n"))?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const POST: &str = "---\ntitle: テスト記事\ndate: 2026-09-17\n---\n\n本文の**段落**。\n";
    const ARTICLE: &str = "---\ntitle: 長い読み物\ndate: 2026-09-15\nupdated: 2026-09-16\ndescription: 一覧に出す要約。\n---\n\n本文。\n";
    const PAGE: &str = "---\ntitle: 自己紹介\n---\n\n自己紹介の本文。\n";

    fn migrations_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../migrations")
    }

    fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, raw) in files {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, raw).unwrap();
        }
        dir
    }

    fn open(bytes: &[u8]) -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
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
        let content = fixture(&[
            ("posts/a.md", POST),
            ("posts/b.md", &second),
            ("posts/note.txt", "md 以外は無視"),
            ("articles/long.md", ARTICLE),
            ("pages/about.md", PAGE),
        ]);
        let bytes = build_db_bytes(content.path(), &migrations_dir()).unwrap();

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
            ["0001", "0002", "0003"]
        );
    }

    #[test]
    fn missing_content_directories_are_treated_as_empty() {
        let content = fixture(&[("posts/a.md", POST)]);
        let bytes = build_db_bytes(content.path(), &migrations_dir()).unwrap();
        let (_dir, conn) = open(&bytes);
        assert!(column(&conn, "SELECT slug FROM articles").is_empty());
    }

    #[test]
    fn invalid_document_fails_the_build() {
        let content = fixture(&[("posts/bad.md", "---\ntitle: t\n---\n本文")]);
        let err = build_db_bytes(content.path(), &migrations_dir()).unwrap_err();
        assert!(err.to_string().contains("date"));
    }

    #[test]
    fn same_input_yields_same_file_name() {
        let content = fixture(&[("posts/a.md", POST), ("articles/long.md", ARTICLE)]);
        let first = db_file_name(&build_db_bytes(content.path(), &migrations_dir()).unwrap());
        let second = db_file_name(&build_db_bytes(content.path(), &migrations_dir()).unwrap());
        assert_eq!(first, second);
        assert!(first.starts_with("articles-") && first.ends_with(".sqlite"));
        assert_eq!(first.len(), "articles-".len() + 16 + ".sqlite".len());
    }

    #[test]
    fn write_output_replaces_old_db_and_writes_manifest() {
        let out = tempfile::tempdir().unwrap();
        let out_dir = out.path().join("db");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(out_dir.join("articles-0000000000000000.sqlite"), b"old").unwrap();

        let name = write_output(&out_dir, b"new bytes").unwrap();

        let mut entries: Vec<String> = fs::read_dir(&out_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        entries.sort();
        assert_eq!(entries, [name.clone(), "manifest.json".to_string()]);
        assert_eq!(
            fs::read_to_string(out_dir.join("manifest.json")).unwrap(),
            format!("{{\"db\":\"/db/{name}\"}}\n")
        );
    }
}
