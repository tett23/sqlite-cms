use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;

#[derive(Debug, PartialEq)]
pub struct Migration {
    pub version: String,
    pub name: String,
    pub sql: String,
}

fn parse_file_name(file_name: &str) -> Option<(String, String)> {
    let stem = file_name.strip_suffix(".sql")?;
    let (version, name) = stem.split_once('_')?;
    if version.is_empty() || name.is_empty() || !version.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((version.to_string(), name.to_string()))
}

pub fn read_migrations(dir: &Path) -> Result<Vec<Migration>> {
    let mut file_names: Vec<String> = fs::read_dir(dir)
        .with_context(|| format!("マイグレーションディレクトリを読めません: {}", dir.display()))?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".sql"))
        .collect();
    file_names.sort();

    file_names
        .into_iter()
        .map(|file_name| {
            let (version, name) = parse_file_name(&file_name).ok_or_else(|| {
                anyhow!("マイグレーションのファイル名が不正です: {file_name}（NNNN_名前.sql）")
            })?;
            let sql = fs::read_to_string(dir.join(&file_name))?;
            Ok(Migration { version, name, sql })
        })
        .collect()
}

pub fn apply_migrations(conn: &Connection, migrations: &[Migration]) -> Result<Vec<String>> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           version TEXT PRIMARY KEY,
           name    TEXT NOT NULL
         )",
    )?;

    let applied: HashSet<String> = conn
        .prepare("SELECT version FROM schema_migrations")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;

    let mut newly_applied = Vec::new();
    for m in migrations {
        if applied.contains(&m.version) {
            continue;
        }
        conn.execute_batch(&m.sql)
            .with_context(|| format!("マイグレーション {}_{} の適用に失敗しました", m.version, m.name))?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            (&m.version, &m.name),
        )?;
        newly_applied.push(m.version.clone());
    }
    Ok(newly_applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_migrations() -> Vec<Migration> {
        read_migrations(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../migrations")).unwrap()
    }

    #[test]
    fn reads_versions_and_names_in_file_name_order() {
        let migrations = repo_migrations();
        let versions: Vec<&str> = migrations.iter().map(|m| m.version.as_str()).collect();
        assert_eq!(versions, ["0001", "0002", "0003"]);
        assert_eq!(migrations[0].name, "initial");
        assert_eq!(migrations[1].name, "content_types");
    }

    #[test]
    fn rejects_malformed_file_names() {
        assert_eq!(parse_file_name("0001_initial.sql"), Some(("0001".into(), "initial".into())));
        assert_eq!(parse_file_name("initial.sql"), None);
        assert_eq!(parse_file_name("abc_initial.sql"), None);
    }

    #[test]
    fn applies_all_to_empty_db_and_records_them() {
        let conn = Connection::open_in_memory().unwrap();
        let applied = apply_migrations(&conn, &repo_migrations()).unwrap();
        assert_eq!(applied, ["0001", "0002", "0003"]);

        let recorded: Vec<(String, String)> = conn
            .prepare("SELECT version, name FROM schema_migrations ORDER BY version")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            recorded,
            [
                ("0001".into(), "initial".into()),
                ("0002".into(), "content_types".into()),
                ("0003".into(), "markdown_body".into()),
            ]
        );

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        for table in ["posts", "articles", "pages"] {
            assert!(tables.iter().any(|t| t == table), "{table} がありません");
        }
    }

    #[test]
    fn skips_already_applied_versions() {
        let conn = Connection::open_in_memory().unwrap();
        let migrations = repo_migrations();
        apply_migrations(&conn, &migrations[..1]).unwrap();
        let applied = apply_migrations(&conn, &migrations).unwrap();
        assert_eq!(applied, ["0002", "0003"]);
    }
}
