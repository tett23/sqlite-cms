//! 記事リポジトリの `.env` から環境変数を読む（ADR 0027）。
//!
//! 実際の環境変数があれば、そちらを優先する（CI のシークレットなどを上書きしない）。
//! プロセスの環境変数は書き換えず、CLI が読む変数を探すときにだけ `.env` の値を使う。

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{bail, Context, Result};

pub const FILE_NAME: &str = ".env";
pub const EXAMPLE_FILE_NAME: &str = ".env.example";

/// `.env` の中身。書き方は次のとおり。
/// - 1 行に `名前=値` を一つ。名前は英字か `_` で始まり、英数字と `_` だけからなる。先頭の `export ` は無視する。
/// - 空行と、`#` で始まる行は読み飛ばす。
/// - 値は、そのまま書くか、`"..."`（`\n`、`\"`、`\\` のエスケープが使える）か `'...'`（そのまま）で囲む。
/// - 囲まない値では、空白の後の `#` から行末までを注釈とみなす。前後の空白は除く。
/// - 同じ名前が何度も出てきたら、後のものを使う。
/// - 複数行にわたる値は書けない。
pub fn parse(text: &str) -> std::result::Result<HashMap<String, String>, (usize, String)> {
    let mut values = HashMap::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").map_or(line, str::trim_start);
        let Some((name, rest)) = line.split_once('=') else {
            return Err((number, "`名前=値` の形で書いてください".into()));
        };
        let name = name.trim();
        if !valid_name(name) {
            return Err((number, format!("名前 `{name}` が不正です（英字か _ で始まり、英数字と _ だけを使えます）")));
        }
        let value = parse_value(rest.trim()).map_err(|message| (number, message))?;
        values.insert(name.to_string(), value);
    }
    Ok(values)
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_value(raw: &str) -> std::result::Result<String, String> {
    let (value, rest) = if let Some(body) = raw.strip_prefix('"') {
        let mut value = String::new();
        let mut chars = body.char_indices();
        let end = loop {
            match chars.next() {
                None => return Err("\" で始めた値が閉じていません（複数行の値は書けません）".into()),
                Some((i, '"')) => break i + 1,
                Some((_, '\\')) => match chars.next() {
                    Some((_, 'n')) => value.push('\n'),
                    Some((_, '"')) => value.push('"'),
                    Some((_, '\\')) => value.push('\\'),
                    Some((_, c)) => return Err(format!("使えないエスケープ `\\{c}` があります（\\n、\\\"、\\\\ が使えます）")),
                    None => return Err("\" で始めた値が閉じていません".into()),
                },
                Some((_, c)) => value.push(c),
            }
        };
        (value, &body[end..])
    } else if let Some(body) = raw.strip_prefix('\'') {
        let Some(end) = body.find('\'') else {
            return Err("' で始めた値が閉じていません（複数行の値は書けません）".into());
        };
        (body[..end].to_string(), &body[end + 1..])
    } else {
        let value = match raw.find(" #").or_else(|| raw.find("\t#")) {
            Some(comment) => &raw[..comment],
            None => raw,
        };
        return Ok(value.trim().to_string());
    };
    let rest = rest.trim();
    if !rest.is_empty() && !rest.starts_with('#') {
        return Err(format!("閉じた引用符の後に `{rest}` があります"));
    }
    Ok(value)
}

/// 環境変数と `.env` を合わせて引く。
pub struct Env {
    file: HashMap<String, String>,
}

impl Env {
    /// `site_dir/.env` を読む。ファイルがなければ、環境変数だけを使う。
    pub fn load(site_dir: &Path) -> Result<Env> {
        let path = site_dir.join(FILE_NAME);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Env { file: HashMap::new() }),
            Err(error) => return Err(error).with_context(|| format!("{} を読めません", path.display())),
        };
        match parse(&text) {
            Ok(file) => Ok(Env { file }),
            Err((line, message)) => bail!("{} の {line} 行目: {message}", path.display()),
        }
    }

    #[cfg(test)]
    pub fn from_file(file: HashMap<String, String>) -> Env {
        Env { file }
    }

    /// 環境変数（空でないもの）があればそれを、なければ `.env` の値（空でないもの）を返す。
    pub fn get(&self, name: &str) -> Option<String> {
        self.get_with(name, |name| std::env::var(name).ok())
    }

    fn get_with(&self, name: &str, process: impl Fn(&str) -> Option<String>) -> Option<String> {
        process(name)
            .filter(|value| !value.is_empty())
            .or_else(|| self.file.get(name).filter(|value| !value.is_empty()).cloned())
    }
}

/// `init` が作る `.env.example`。
pub const EXAMPLE: &str = "\
# sqlite-cms deploy が使う Cloudflare の認証情報。
# このファイルを .env という名前でコピーし、値を書く。
# .env は Git に入れない（init が .gitignore に書く）。実際の環境変数があれば、そちらを優先する。

# API トークン（権限はアカウントの「Workers スクリプト：編集」だけでよい）
CLOUDFLARE_API_TOKEN=
# Cloudflare のアカウント ID（ダッシュボードの Workers & Pages の画面に表示される）
CLOUDFLARE_ACCOUNT_ID=
";

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn parses_plain_quoted_and_commented_lines() {
        let text = "\
# 注釈
CLOUDFLARE_API_TOKEN=abc123

export CLOUDFLARE_ACCOUNT_ID = def456  # 行末の注釈
DOUBLE=\"a b\\n\\\"c\\\" \\\\ # 注釈ではない\"  # 注釈
SINGLE='そのまま \\n # も'
EMPTY=
HASH=a#b
";
        assert_eq!(
            parse(text).unwrap(),
            map(&[
                ("CLOUDFLARE_API_TOKEN", "abc123"),
                ("CLOUDFLARE_ACCOUNT_ID", "def456"),
                ("DOUBLE", "a b\n\"c\" \\ # 注釈ではない"),
                ("SINGLE", "そのまま \\n # も"),
                ("EMPTY", ""),
                ("HASH", "a#b"),
            ])
        );
    }

    #[test]
    fn later_values_win_and_crlf_is_accepted() {
        assert_eq!(parse("A=1\r\nA=2\r\n").unwrap(), map(&[("A", "2")]));
    }

    #[test]
    fn reports_the_line_of_errors() {
        assert_eq!(parse("A=1\nnot an assignment\n").unwrap_err().0, 2);
        assert_eq!(parse("1A=x").unwrap_err().0, 1);
        assert_eq!(parse("A-B=x").unwrap_err().0, 1);
        assert_eq!(parse("=x").unwrap_err().0, 1);
        assert_eq!(parse("A=\"open\nB=1").unwrap_err().0, 1);
        assert_eq!(parse("A='open").unwrap_err().0, 1);
        assert_eq!(parse("A=\"x\" y").unwrap_err().0, 1);
        assert_eq!(parse("A=\"\\t\"").unwrap_err().0, 1);
    }

    #[test]
    fn process_environment_wins_over_the_file() {
        let env = Env::from_file(map(&[("A", "file"), ("B", "file"), ("C", "")]));
        let process = |name: &str| match name {
            "A" => Some("process".to_string()),
            "B" => Some(String::new()),
            _ => None,
        };
        assert_eq!(env.get_with("A", process).as_deref(), Some("process"));
        // 空の環境変数は設定されていないものとみなし、.env を使う。
        assert_eq!(env.get_with("B", process).as_deref(), Some("file"));
        // .env の空の値も設定されていないものとみなす（.env.example をコピーしただけの状態）。
        assert_eq!(env.get_with("C", process), None);
        assert_eq!(env.get_with("D", process), None);
    }

    #[test]
    fn missing_file_is_empty_and_broken_file_is_an_error() {
        let dir = crate::testutil::tempdir();
        let env = Env::load(dir.path()).unwrap();
        assert!(env.file.is_empty());

        fs::write(dir.path().join(FILE_NAME), "A=1\nB\n").unwrap();
        let message = Env::load(dir.path()).err().unwrap().to_string();
        assert!(message.contains(".env の 2 行目"), "{message}");
    }

    #[test]
    fn example_parses_to_empty_credentials() {
        assert_eq!(
            parse(EXAMPLE).unwrap(),
            map(&[("CLOUDFLARE_API_TOKEN", ""), ("CLOUDFLARE_ACCOUNT_ID", "")])
        );
    }
}
