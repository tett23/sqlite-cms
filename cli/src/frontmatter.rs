use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};

/// frontmatter の値。
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// 値なし（空、`~`、`null`）。
    Null,
    Scalar(String),
    /// リスト（`[a, b]` か、`- a` の行の並び）。
    List(Vec<String>),
}

impl Value {
    /// 文字列の値。値なしとリストは None。
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Scalar(s) => Some(s),
            _ => None,
        }
    }
}

/// frontmatter のキーと値。
pub type Frontmatter = HashMap<String, Value>;

/// YAML のうち、frontmatter に使う「キー: 値」の平らな形と、文字列のリストだけを読む。
/// リストは、一行の形（`tags: [a, b]`）と、複数行の形（`tags:` の次の行から `- a`）で書ける（ADR 0048）。
/// 複数行の値、入れ子、アンカーなどは誤読せずにエラーにする。
pub fn parse(yaml: &str) -> Result<Frontmatter> {
    let mut map = HashMap::new();
    let lines: Vec<&str> = yaml.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let n = index + 1;
        index += 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            bail!("{n} 行目: 字下げした行（複数行の値や入れ子）には対応していません");
        }
        let Some((key, rest)) = line.split_once(':') else {
            bail!("{n} 行目: 「キー: 値」の形になっていません");
        };
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
            bail!("{n} 行目: キー {key:?} には英数字、_、- だけを使ってください");
        }
        if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
            bail!("{n} 行目: {key}: の後ろに空白を入れてください");
        }
        let rest = rest.trim();
        let mut value = if rest.starts_with('[') {
            Value::List(flow_list(rest).map_err(|e| anyhow!("{n} 行目（{key}）: {e}"))?)
        } else {
            match parse_value(rest).map_err(|e| anyhow!("{n} 行目（{key}）: {e}"))? {
                Some(s) => Value::Scalar(s),
                None => Value::Null,
            }
        };
        // 値のないキーの次の行から `- 値` が続けば、複数行のリストとして読む。
        if value == Value::Null {
            let mut items = Vec::new();
            while let Some(item) = lines.get(index).and_then(|l| l.trim_start().strip_prefix('-')) {
                let m = index + 1;
                if !(item.is_empty() || item.starts_with([' ', '\t'])) {
                    bail!("{m} 行目（{key}）: - の後ろに空白を入れてください");
                }
                match parse_value(item.trim()).map_err(|e| anyhow!("{m} 行目（{key}）: {e}"))? {
                    Some(s) => items.push(s),
                    None => bail!("{m} 行目（{key}）: リストの値が空です{}", hash_hint(item.trim())),
                }
                index += 1;
            }
            if !items.is_empty() {
                value = Value::List(items);
            }
        }
        if map.insert(key.to_string(), value).is_some() {
            bail!("{n} 行目: キー {key} が重複しています");
        }
    }
    Ok(map)
}

/// `#` で始まる値は YAML では注釈になる。ハッシュタグを `#` 付きで書いたときに、直し方を添える。
fn hash_hint(item: &str) -> &'static str {
    if item.starts_with('#') {
        "（# で始まる値は注釈として読まれます。# を外すか、クォートで囲んでください）"
    } else {
        ""
    }
}

/// 一行のリスト（`[a, "b, c", 'd']`）を読む。値は文字列だけで、入れ子は読まない。最後の `,` は許す。
fn flow_list(s: &str) -> std::result::Result<Vec<String>, String> {
    let mut items = Vec::new();
    let mut rest = s[1..].trim_start();
    loop {
        if let Some(after) = rest.strip_prefix(']') {
            after_closing_quote(after).map_err(|_| format!("] の後ろに {:?} があります", after.trim()))?;
            return Ok(items);
        }
        let (item, after) = match rest.chars().next() {
            None => return Err("リストが ] で閉じていません".to_string()),
            Some('"') => {
                let end = closing_double_quote(&rest[1..]).ok_or("ダブルクォートが閉じていません")?;
                (double_quoted(&rest[1..=end + 1])?, &rest[end + 2..])
            }
            Some('\'') => {
                let end = closing_single_quote(&rest[1..]).ok_or("シングルクォートが閉じていません")?;
                (single_quoted(&rest[1..=end + 1])?, &rest[end + 2..])
            }
            Some(_) => {
                let end = rest.find([',', ']']).ok_or("リストが ] で閉じていません")?;
                let plain = rest[..end].trim();
                if plain.is_empty() {
                    return Err("リストの値が空です".to_string());
                }
                if plain.starts_with('#') {
                    return Err(format!("リストの値 {plain:?} を読めません{}", hash_hint(plain)));
                }
                if plain.contains(['[', '{', '}']) {
                    return Err(format!("リストの値 {plain:?} を読めません（入れ子には対応していません）"));
                }
                (plain.to_string(), &rest[end..])
            }
        };
        items.push(item);
        let after = after.trim_start();
        rest = match after.strip_prefix(',') {
            Some(next) => next.trim_start(),
            None if after.starts_with(']') => after,
            None if after.is_empty() => return Err("リストが ] で閉じていません".to_string()),
            None => return Err(format!("リストの値の後ろに {after:?} があります（, で区切ってください）")),
        };
    }
}

/// ダブルクォートの中身の先頭から、閉じるクォートの位置を探す（`\"` は飛ばす）。
fn closing_double_quote(body: &str) -> Option<usize> {
    let mut escaped = false;
    for (i, c) in body.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return Some(i),
            _ => {}
        }
    }
    None
}

/// シングルクォートの中身の先頭から、閉じるクォートの位置を探す（`''` は飛ばす）。
fn closing_single_quote(body: &str) -> Option<usize> {
    let mut chars = body.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\'' {
            if let Some((_, '\'')) = chars.peek() {
                chars.next();
                continue;
            }
            return Some(i);
        }
    }
    None
}

fn parse_value(s: &str) -> std::result::Result<Option<String>, String> {
    match s.chars().next() {
        None | Some('#') => Ok(None),
        Some('"') => double_quoted(&s[1..]).map(Some),
        Some('\'') => single_quoted(&s[1..]).map(Some),
        Some(c @ ('|' | '>' | '[' | ']' | '{' | '}' | ',' | '&' | '*' | '!' | '@' | '`' | '%')) => Err(format!(
            "{c} で始まる値には対応していません（複数行の値、リスト、アンカーなど）。値全体をクォートで囲んでください"
        )),
        Some(_) => {
            let plain = match s.find(" #") {
                Some(i) => s[..i].trim_end(),
                None => s,
            };
            match plain {
                "~" | "null" | "Null" | "NULL" => Ok(None),
                _ => Ok(Some(plain.to_string())),
            }
        }
    }
}

fn after_closing_quote(rest: &str) -> std::result::Result<(), String> {
    let rest = rest.trim();
    if rest.is_empty() || rest.starts_with('#') {
        Ok(())
    } else {
        Err(format!("閉じたクォートの後ろに {rest:?} があります"))
    }
}

fn double_quoted(body: &str) -> std::result::Result<String, String> {
    let mut out = String::new();
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => {
                after_closing_quote(&body[i + 1..])?;
                return Ok(out);
            }
            '\\' => match chars.next().map(|(_, c)| c) {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('/') => out.push('/'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                    let code = u32::from_str_radix(&hex, 16).ok().filter(|_| hex.len() == 4);
                    match code.and_then(char::from_u32) {
                        Some(c) => out.push(c),
                        None => return Err(format!("\\u{hex} は正しい文字のエスケープではありません")),
                    }
                }
                Some(other) => return Err(format!("\\{other} というエスケープには対応していません")),
                None => return Err("エスケープが途中で終わっています".to_string()),
            },
            _ => out.push(c),
        }
    }
    Err("ダブルクォートが閉じていません".to_string())
}

fn single_quoted(body: &str) -> std::result::Result<String, String> {
    let mut out = String::new();
    let mut chars = body.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\'' {
            if let Some((_, '\'')) = chars.peek() {
                chars.next();
                out.push('\'');
                continue;
            }
            after_closing_quote(&body[i + 1..])?;
            return Ok(out);
        }
        out.push(c);
    }
    Err("シングルクォートが閉じていません".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(yaml: &str, key: &str) -> Option<String> {
        parse(yaml).unwrap().remove(key).expect("キーがありません").as_str().map(str::to_string)
    }

    fn list(yaml: &str) -> Vec<String> {
        match parse(yaml).unwrap().remove("tags") {
            Some(Value::List(items)) => items,
            other => panic!("リストではありません: {other:?}"),
        }
    }

    fn err(yaml: &str) -> String {
        format!("{:#}", parse(yaml).unwrap_err())
    }

    #[test]
    fn reads_plain_values() {
        let fm = parse("title: 記事のタイトル\ndate: 2026-09-17\n").unwrap();
        assert_eq!(fm["title"].as_str(), Some("記事のタイトル"));
        assert_eq!(fm["date"].as_str(), Some("2026-09-17"));
    }

    #[test]
    fn plain_values_keep_inner_colons_and_trim_comments() {
        assert_eq!(get("title: 記事: その2", "title").as_deref(), Some("記事: その2"));
        assert_eq!(get("title: 本題 # メモ", "title").as_deref(), Some("本題"));
        assert_eq!(get("title: C#入門", "title").as_deref(), Some("C#入門"));
    }

    #[test]
    fn reads_double_quoted_values_with_escapes() {
        assert_eq!(get(r#"title: "a: \"b\" \\ c""#, "title").as_deref(), Some(r#"a: "b" \ c"#));
        assert_eq!(get(r#"title: "改行\nと\u3042""#, "title").as_deref(), Some("改行\nとあ"));
        assert_eq!(get(r##"title: "# ではない" # コメント"##, "title").as_deref(), Some("# ではない"));
    }

    #[test]
    fn reads_single_quoted_values() {
        assert_eq!(get("title: 'It''s \\n'", "title").as_deref(), Some("It's \\n"));
    }

    #[test]
    fn null_forms_are_none() {
        for yaml in ["description:", "description: ~", "description: null", "description:   # メモ"] {
            assert_eq!(get(yaml, "description"), None, "{yaml}");
        }
        assert_eq!(get("description: \"null\"", "description").as_deref(), Some("null"));
    }

    #[test]
    fn skips_blank_lines_and_comments() {
        let fm = parse("# 先頭のコメント\n\ntitle: t\n   \n# 途中\n").unwrap();
        assert_eq!(fm.len(), 1);
    }

    #[test]
    fn rejects_unsupported_yaml() {
        assert!(err("description: |\n  複数行").contains("| で始まる値"));
        assert!(err("description: >-\n  折り返し").contains("> で始まる値"));
        assert!(err("title: t\n  continued").contains("2 行目: 字下げした行"));
        assert!(err("tags: {a: b}").contains("{ で始まる値"));
        assert!(err("title: &anchor t").contains("& で始まる値"));
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(err("just text").contains("1 行目: 「キー: 値」の形"));
        assert!(err("title:t").contains("後ろに空白"));
        assert!(err("my title: t").contains("キー \"my title\""));
        assert!(err("title: a\ntitle: b").contains("2 行目: キー title が重複"));
    }

    #[test]
    fn rejects_broken_quotes() {
        assert!(err("title: \"open").contains("ダブルクォートが閉じていません"));
        assert!(err("title: 'open").contains("シングルクォートが閉じていません"));
        assert!(err("title: \"a\" b").contains("閉じたクォートの後ろ"));
        assert!(err("title: \"\\q\"").contains("\\q というエスケープ"));
        assert!(err("title: \"\\u12\"").contains("\\u12"));
    }

    #[test]
    fn reads_flow_lists() {
        assert_eq!(list("tags: [組版, SQLite]"), ["組版", "SQLite"]);
        assert_eq!(list("tags: [ a ,b,  c d ]  # メモ"), ["a", "b", "c d"]);
        assert_eq!(list(r##"tags: ["a, b", 'c''d', "\"e\"", "#f"]"##), ["a, b", "c'd", "\"e\"", "#f"]);
        assert_eq!(list("tags: [a, ]"), ["a"]);
        assert_eq!(list("tags: []"), Vec::<String>::new());
    }

    #[test]
    fn reads_block_lists() {
        assert_eq!(list("tags:\n  - 組版\n  - \"a b\"\n- c # メモ\ntitle: t"), ["組版", "a b", "c"]);
        let fm = parse("tags:\n  - a\ntitle: t").unwrap();
        assert_eq!(fm["title"].as_str(), Some("t"));
        // 値のないキーの次に - の行がなければ、値なしのまま。
        assert_eq!(parse("tags:\ntitle: t").unwrap()["tags"], Value::Null);
    }

    #[test]
    fn rejects_broken_lists() {
        assert!(err("tags: [a, b").contains("リストが ] で閉じていません"));
        assert!(err("tags: [a b] c").contains("] の後ろに \"c\""));
        assert!(err("tags: [a, , b]").contains("リストの値が空です"));
        assert!(err("tags: [[a]]").contains("入れ子には対応していません"));
        assert!(err("tags: [\"a\" b]").contains(", で区切ってください"));
        assert!(err("tags: [\"a]").contains("ダブルクォートが閉じていません"));
        assert!(err("tags:\n  -a").contains("2 行目（tags）: - の後ろに空白"));
        // ハッシュタグを # 付きで書くと YAML の注釈になるので、直し方を知らせる。
        assert!(err("tags: [#組版]").contains("# を外すか、クォートで囲んでください"));
        assert!(err("tags:\n  - #組版").contains("# を外すか、クォートで囲んでください"));
    }
}
