use std::collections::HashSet;

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

include!(concat!(env!("OUT_DIR"), "/migrations.rs"));

pub fn embedded_migrations() -> Result<Vec<Migration>> {
    EMBEDDED
        .iter()
        .map(|(file_name, sql)| {
            let (version, name) = parse_file_name(file_name).ok_or_else(|| {
                anyhow!("マイグレーションのファイル名が不正です: {file_name}（NNNN_名前.sql）")
            })?;
            Ok(Migration { version, name, sql: sql.to_string() })
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
        embedded_migrations().unwrap()
    }

    #[test]
    fn embeds_versions_and_names_in_file_name_order() {
        let migrations = repo_migrations();
        let versions: Vec<&str> = migrations.iter().map(|m| m.version.as_str()).collect();
        assert_eq!(versions, ["0001", "0002", "0003", "0004", "0005", "0006", "0007"]);
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
        assert_eq!(applied, ["0001", "0002", "0003", "0004", "0005", "0006", "0007"]);

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
                ("0004".into(), "site".into()),
                ("0005".into(), "site_description".into()),
                ("0006".into(), "link_cards".into()),
                ("0007".into(), "site_header".into()),
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
        assert_eq!(applied, ["0002", "0003", "0004", "0005", "0006", "0007"]);
    }
}
