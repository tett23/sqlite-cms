use anyhow::{anyhow, bail, Result};

use crate::date::is_valid_date;
use crate::frontmatter::{self, Frontmatter, Value};

#[derive(Debug, PartialEq)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub published_at: String,
    pub body_md: String,
    /// タグ（ADR 0048）。先頭の # は付けない。
    pub tags: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Article {
    /// URL の名前。frontmatter の slug があればそれ、なければファイル名から取ったもの（ADR 0058、0059）。
    pub slug: String,
    /// 同じ日付の記事を並べる順の鍵。ファイル名から取ったもの（UUIDv7 か、ファイル名）。
    pub order_key: String,
    pub title: String,
    pub published_at: String,
    pub updated_at: Option<String>,
    pub description: Option<String>,
    pub body_md: String,
    /// タグ（ADR 0048）。先頭の # は付けない。
    pub tags: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Page {
    pub slug: String,
    pub title: String,
    pub body_md: String,
}

struct Doc {
    frontmatter: Frontmatter,
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
    let frontmatter =
        frontmatter::parse(yaml).map_err(|e| anyhow!("{slug}: frontmatter を解釈できません: {e:#}"))?;

    // リストを書けるのは tags だけ。ほかのキーにリストを書いたら、値なしとして読まずに知らせる。
    if let Some(key) = frontmatter.iter().find(|(key, v)| *key != "tags" && matches!(v, Value::List(_))).map(|(key, _)| key) {
        bail!("{slug}: {key} にリストは書けません");
    }

    let title = match value(&frontmatter, "title") {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => bail!("{slug}: frontmatter に title がありません"),
    };

    Ok(Doc {
        frontmatter,
        title,
        body_md: body.to_string(),
    })
}

fn value<'a>(frontmatter: &'a Frontmatter, key: &str) -> Option<&'a str> {
    frontmatter.get(key).and_then(Value::as_str)
}

/// タグを読む（ADR 0048）。`tags: [組版, SQLite]` のリストで書く。
/// 先頭の # は外す（`"#組版"` とクォートして書いてもよい）。同じタグは一つにまとめる。
fn tags(slug: &str, frontmatter: &Frontmatter) -> Result<Vec<String>> {
    let items = match frontmatter.get("tags") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Scalar(_)) => bail!("{slug}: tags は [組版, SQLite] のようにリストで書いてください"),
        Some(Value::List(items)) => items,
    };
    let mut tags: Vec<String> = Vec::new();
    for item in items {
        let tag = item.trim();
        let tag = tag.strip_prefix(['#', '＃']).unwrap_or(tag);
        if tag.is_empty() {
            bail!("{slug}: tags に空のタグがあります");
        }
        if tag.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '#' | '＃')) {
            bail!("{slug}: タグ {tag:?} に、空白か # が含まれています（検索では空白で語を区切るので、タグには使えません）");
        }
        if !tags.iter().any(|t| t == tag) {
            tags.push(tag.to_string());
        }
    }
    Ok(tags)
}

fn date_field(frontmatter: &Frontmatter, key: &str) -> Option<String> {
    value(frontmatter, key).filter(|s| is_valid_date(s)).map(str::to_string)
}

fn require_date(slug: &str, frontmatter: &Frontmatter) -> Result<String> {
    date_field(frontmatter, "date")
        .ok_or_else(|| anyhow!("{slug}: frontmatter に date (YYYY-MM-DD) がありません"))
}

pub fn parse_post(slug: &str, raw: &str) -> Result<Post> {
    let doc = parse_doc(slug, raw)?;
    Ok(Post {
        slug: slug.to_string(),
        published_at: require_date(slug, &doc.frontmatter)?,
        tags: tags(slug, &doc.frontmatter)?,
        title: doc.title,
        body_md: doc.body_md,
    })
}

pub fn parse_article(slug: &str, raw: &str) -> Result<Article> {
    let doc = parse_doc(slug, raw)?;

    let description = value(&doc.frontmatter, "description").map(str::to_string);

    let updated_at = match value(&doc.frontmatter, "updated") {
        None => None,
        Some(_) => Some(
            date_field(&doc.frontmatter, "updated")
                .ok_or_else(|| anyhow!("{slug}: updated は YYYY-MM-DD で指定してください"))?,
        ),
    };

    // frontmatter に slug を書けば、それを URL にする（ADR 0059）。
    let url_slug = match value(&doc.frontmatter, "slug") {
        None => slug.to_string(),
        Some(custom) if is_valid_slug(custom) => custom.to_string(),
        Some(custom) => bail!("{slug}: slug {custom:?} には文字、数字、-、_ だけを使ってください（URL になります）"),
    };

    Ok(Article {
        slug: url_slug,
        order_key: slug.to_string(),
        published_at: require_date(slug, &doc.frontmatter)?,
        updated_at,
        description,
        tags: tags(slug, &doc.frontmatter)?,
        title: doc.title,
        body_md: doc.body_md,
    })
}

/// slug に使える文字（文字、数字、-、_）だけでできているか。
pub fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
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
    fn post_with_impossible_date_is_error() {
        let err = parse_post("bad", "---\ntitle: t\ndate: 2026-02-30\n---\n本文").unwrap_err();
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

    #[test]
    fn tags_are_read_from_a_list() {
        let raw = "---\ntitle: t\ndate: 2026-01-01\ntags: [組版, \"#SQLite\", ＃日本語, 組版]\n---\n本文";
        assert_eq!(parse_post("p", raw).unwrap().tags, ["組版", "SQLite", "日本語"]);
        assert_eq!(parse_article("a", raw).unwrap().tags, ["組版", "SQLite", "日本語"]);
        let block = "---\ntitle: t\ndate: 2026-01-01\ntags:\n  - a\n  - b\n---\n本文";
        assert_eq!(parse_post("p", block).unwrap().tags, ["a", "b"]);
    }

    #[test]
    fn tags_are_optional() {
        assert!(parse_post("p", POST).unwrap().tags.is_empty());
        assert!(parse_post("p", "---\ntitle: t\ndate: 2026-01-01\ntags:\n---\n").unwrap().tags.is_empty());
        assert!(parse_post("p", "---\ntitle: t\ndate: 2026-01-01\ntags: []\n---\n").unwrap().tags.is_empty());
    }

    #[test]
    fn broken_tags_are_errors() {
        let post = |tags: &str| parse_post("p", &format!("---\ntitle: t\ndate: 2026-01-01\ntags: {tags}\n---\n")).unwrap_err().to_string();
        assert!(post("組版").contains("リストで書いてください"));
        assert!(post("[\"a b\"]").contains("空白か # が含まれています"));
        assert!(post("[\"a#b\"]").contains("空白か # が含まれています"));
        assert!(post("[\"#\"]").contains("空のタグ"));
        assert!(post("[\"\"]").contains("空のタグ"));
    }

    #[test]
    fn article_slug_in_frontmatter_becomes_the_url() {
        // frontmatter の slug を URL にし、並べる順の鍵はファイル名から取ったものにする（ADR 0059）。
        let raw = "---\ntitle: t\nslug: long-read\ndate: 2026-01-01\n---\n本文";
        let article = parse_article("017f22e2-79b0-7cc3-98c4-dc0c0c07398f", raw).unwrap();
        assert_eq!(article.slug, "long-read");
        assert_eq!(article.order_key, "017f22e2-79b0-7cc3-98c4-dc0c0c07398f");
        let article = parse_article("plain", ARTICLE).unwrap();
        assert_eq!((article.slug.as_str(), article.order_key.as_str()), ("plain", "plain"));
        for bad in ["\"a b\"", "../x", "\"\""] {
            let raw = format!("---\ntitle: t\nslug: {bad}\ndate: 2026-01-01\n---\n");
            let err = parse_article("a", &raw).unwrap_err().to_string();
            assert!(err.contains("文字、数字、-、_ だけ"), "{bad}: {err}");
        }
    }

    #[test]
    fn pages_ignore_tags() {
        // page にはタグを付けない（ADR 0048）。書いても読まないので、リストでない値でもエラーにしない。
        for tags in ["[組版, SQLite]", "組版", "[\"a b\"]"] {
            let page = parse_page("about", &format!("---\ntitle: 自己紹介\ntags: {tags}\n---\n本文\n")).unwrap();
            assert_eq!(page.title, "自己紹介", "{tags}");
            assert_eq!(page.body_md, "本文\n", "{tags}");
        }
    }

    #[test]
    fn lists_are_only_allowed_for_tags() {
        let err = parse_post("p", "---\ntitle: [a, b]\ndate: 2026-01-01\n---\n").unwrap_err();
        assert!(err.to_string().contains("p: title にリストは書けません"), "{err}");
    }
}
