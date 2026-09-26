use anyhow::{anyhow, bail, Result};
use serde_yaml::Value;

#[derive(Debug, PartialEq)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub published_at: String,
    pub body_md: String,
}

#[derive(Debug, PartialEq)]
pub struct Article {
    pub slug: String,
    pub title: String,
    pub published_at: String,
    pub updated_at: Option<String>,
    pub description: Option<String>,
    pub body_md: String,
}

#[derive(Debug, PartialEq)]
pub struct Page {
    pub slug: String,
    pub title: String,
    pub body_md: String,
}

struct Doc {
    frontmatter: Value,
    title: String,
    body_md: String,
}

fn split_frontmatter(raw: &str) -> (&str, &str) {
    let Some(rest) = raw.strip_prefix("---\n") else {
        return ("", raw);
    };
    if let Some(body) = rest.strip_prefix("---\n") {
        return ("", body);
    }
    match rest.find("\n---\n") {
        Some(end) => (&rest[..end], &rest[end + "\n---\n".len()..]),
        None => match rest.strip_suffix("\n---") {
            Some(yaml) => (yaml, ""),
            None => ("", raw),
        },
    }
}

fn parse_doc(slug: &str, raw: &str) -> Result<Doc> {
    let (yaml, body) = split_frontmatter(raw);
    let frontmatter: Value = if yaml.trim().is_empty() {
        Value::Null
    } else {
        serde_yaml::from_str(yaml).map_err(|e| anyhow!("{slug}: frontmatter を解釈できません: {e}"))?
    };

    let title = match frontmatter.get("title") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        _ => bail!("{slug}: frontmatter に title がありません"),
    };

    Ok(Doc {
        frontmatter,
        title,
        body_md: body.to_string(),
    })
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
}

fn date_field(frontmatter: &Value, key: &str) -> Option<String> {
    match frontmatter.get(key) {
        Some(Value::String(s)) if is_date(s) => Some(s.clone()),
        _ => None,
    }
}

fn require_date(slug: &str, frontmatter: &Value) -> Result<String> {
    date_field(frontmatter, "date")
        .ok_or_else(|| anyhow!("{slug}: frontmatter に date (YYYY-MM-DD) がありません"))
}

pub fn parse_post(slug: &str, raw: &str) -> Result<Post> {
    let doc = parse_doc(slug, raw)?;
    Ok(Post {
        slug: slug.to_string(),
        published_at: require_date(slug, &doc.frontmatter)?,
        title: doc.title,
        body_md: doc.body_md,
    })
}

pub fn parse_article(slug: &str, raw: &str) -> Result<Article> {
    let doc = parse_doc(slug, raw)?;

    let description = match doc.frontmatter.get("description") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => bail!("{slug}: description は文字列で指定してください"),
    };

    let updated_at = match doc.frontmatter.get("updated") {
        None | Some(Value::Null) => None,
        Some(_) => Some(
            date_field(&doc.frontmatter, "updated")
                .ok_or_else(|| anyhow!("{slug}: updated は YYYY-MM-DD で指定してください"))?,
        ),
    };

    Ok(Article {
        slug: slug.to_string(),
        published_at: require_date(slug, &doc.frontmatter)?,
        updated_at,
        description,
        title: doc.title,
        body_md: doc.body_md,
    })
}

pub fn parse_page(slug: &str, raw: &str) -> Result<Page> {
    let doc = parse_doc(slug, raw)?;
    Ok(Page {
        slug: slug.to_string(),
        title: doc.title,
        body_md: doc.body_md,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const POST: &str = "---\ntitle: テスト記事\ndate: 2026-09-17\n---\n\n本文の**段落**。\n";
    const ARTICLE: &str = "---\ntitle: 長い読み物\ndate: 2026-09-15\nupdated: 2026-09-16\ndescription: 一覧に出す要約。\n---\n\n本文。\n";
    const PAGE: &str = "---\ntitle: 自己紹介\n---\n\n自己紹介の本文。\n";

    #[test]
    fn post_parses_frontmatter_and_body() {
        let p = parse_post("test", POST).unwrap();
        assert_eq!(p.slug, "test");
        assert_eq!(p.title, "テスト記事");
        assert_eq!(p.published_at, "2026-09-17");
        assert_eq!(p.body_md, "\n本文の**段落**。\n");
    }

    #[test]
    fn body_is_stored_verbatim_without_rendering() {
        let raw = "---\ntitle: t\ndate: 2026-01-01\n---\n| a | b |\n|---|---|\n| 1 | 2 |\n\n~~消す~~ <span>生</span>\n";
        let p = parse_post("gfm", raw).unwrap();
        assert_eq!(p.body_md, "| a | b |\n|---|---|\n| 1 | 2 |\n\n~~消す~~ <span>生</span>\n");
    }

    #[test]
    fn post_without_title_is_error() {
        let err = parse_post("bad", "---\ndate: 2026-01-01\n---\n本文").unwrap_err();
        assert!(err.to_string().contains("title"));
    }

    #[test]
    fn post_without_date_is_error() {
        let err = parse_post("bad", "---\ntitle: t\n---\n本文").unwrap_err();
        assert!(err.to_string().contains("date"));
    }

    #[test]
    fn post_with_malformed_date_is_error() {
        let err = parse_post("bad", "---\ntitle: t\ndate: 2026/01/01\n---\n本文").unwrap_err();
        assert!(err.to_string().contains("date"));
    }

    #[test]
    fn article_reads_description_and_updated() {
        let a = parse_article("a", ARTICLE).unwrap();
        assert_eq!(a.published_at, "2026-09-15");
        assert_eq!(a.updated_at.as_deref(), Some("2026-09-16"));
        assert_eq!(a.description.as_deref(), Some("一覧に出す要約。"));
    }

    #[test]
    fn article_description_and_updated_are_optional() {
        let a = parse_article("a", POST).unwrap();
        assert_eq!(a.description, None);
        assert_eq!(a.updated_at, None);
    }

    #[test]
    fn article_with_malformed_updated_is_error() {
        let err =
            parse_article("bad", "---\ntitle: t\ndate: 2026-01-01\nupdated: きのう\n---\n本文").unwrap_err();
        assert!(err.to_string().contains("updated"));
    }

    #[test]
    fn page_needs_no_date() {
        let p = parse_page("about", PAGE).unwrap();
        assert_eq!(p.title, "自己紹介");
        assert_eq!(p.body_md, "\n自己紹介の本文。\n");
    }

    #[test]
    fn document_without_frontmatter_is_error() {
        let err = parse_page("bare", "本文だけ").unwrap_err();
        assert!(err.to_string().contains("title"));
    }
}
