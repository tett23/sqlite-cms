use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};

/// frontmatter のキーと値。値が null（空、`~`、`null`）のときは `None`。
pub type Frontmatter = HashMap<String, Option<String>>;

/// YAML のうち、frontmatter に使う「キー: 値」の平らな形だけを読む。
/// 複数行の値、リスト、入れ子、アンカーなどは誤読せずにエラーにする。
pub fn parse(yaml: &str) -> Result<Frontmatter> {
    let mut map = HashMap::new();
    for (index, line) in yaml.lines().enumerate() {
        let n = index + 1;
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
        let value = parse_value(rest.trim()).map_err(|e| anyhow!("{n} 行目（{key}）: {e}"))?;
        if map.insert(key.to_string(), value).is_some() {
            bail!("{n} 行目: キー {key} が重複しています");
        }
    }
    Ok(map)
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
        parse(yaml).unwrap().remove(key).expect("キーがありません")
    }

    fn err(yaml: &str) -> String {
        format!("{:#}", parse(yaml).unwrap_err())
    }

    #[test]
    fn reads_plain_values() {
        let fm = parse("title: 記事のタイトル\ndate: 2026-09-17\n").unwrap();
        assert_eq!(fm["title"].as_deref(), Some("記事のタイトル"));
        assert_eq!(fm["date"].as_deref(), Some("2026-09-17"));
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
        assert!(err("tags: [a, b]").contains("[ で始まる値"));
        assert!(err("tags:\n  - a").contains("2 行目: 字下げした行"));
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
}
