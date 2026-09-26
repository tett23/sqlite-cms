//! RSS フィード（ADR 0033）。
//!
//! post と article を日付の新しい順に並べた RSS 2.0 のフィードを、`/rss.xml` として作る。
//! フィードの中のリンクは絶対 URL である必要があるので、`site.toml` の `url` を書いたときだけ作る。
//! CLI は Markdown を HTML にしないので、本文は載せず、article の要約か、本文の書き出しを説明として載せる。

use crate::content::{Article, Post};
use crate::date::TimeZone;
use crate::site::SiteConfig;

/// フィードに載せる数。
pub const MAX_ITEMS: usize = 20;
/// 本文の書き出しの長さ（文字数）。
const EXCERPT_CHARS: usize = 120;

#[derive(Debug, PartialEq)]
pub struct FeedItem {
    /// サイトの中のパス（`posts/hello` など。先頭の / は付けない）。
    pub path: String,
    pub title: String,
    pub date: String,
    pub summary: String,
}

/// post と article を、日付の新しい順（同じ日付ならパスの逆順）に並べ、先頭の MAX_ITEMS 件を返す。
pub fn items(posts: &[Post], articles: &[Article]) -> Vec<FeedItem> {
    let mut items: Vec<FeedItem> = posts
        .iter()
        .map(|p| FeedItem {
            path: format!("posts/{}", p.slug),
            title: p.title.clone(),
            date: p.published_at.clone(),
            summary: excerpt(&p.body_md),
        })
        .chain(articles.iter().map(|a| FeedItem {
            path: format!("articles/{}", a.slug),
            title: a.title.clone(),
            date: a.published_at.clone(),
            summary: a.description.clone().filter(|d| !d.trim().is_empty()).unwrap_or_else(|| excerpt(&a.body_md)),
        }))
        .collect();
    items.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.path.cmp(&a.path)));
    items.truncate(MAX_ITEMS);
    items
}

/// Markdown の本文から、記号を大まかに取り除いた書き出しを作る。コードブロックと HTML のタグは除く。
pub fn excerpt(markdown: &str) -> String {
    let mut text = String::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || trimmed.starts_with(":::") || trimmed.starts_with("$$") || trimmed.is_empty() {
            continue;
        }
        let content = trimmed.trim_start_matches(['#', '>', '-', '*', '+']).trim_start();
        text.push_str(&strip_inline(content));
        text.push(' ');
        if text.chars().count() > EXCERPT_CHARS {
            break;
        }
    }
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() > EXCERPT_CHARS {
        format!("{}…", text.chars().take(EXCERPT_CHARS).collect::<String>())
    } else {
        text
    }
}

/// 行の中の HTML のタグ、リンクと画像の URL、強調の記号を取り除く。
fn strip_inline(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '<' => {
                // タグ（とコメント）を読み飛ばす。
                for next in chars.by_ref() {
                    if next == '>' {
                        break;
                    }
                }
            }
            ']' if chars.peek() == Some(&'(') => {
                // リンクと画像の URL を読み飛ばす。
                for next in chars.by_ref() {
                    if next == ')' {
                        break;
                    }
                }
            }
            '[' | '!' if c == '[' || chars.peek() == Some(&'[') => {}
            '*' | '_' | '`' | '~' => {}
            _ => out.push(c),
        }
    }
    out
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// 暦の日付から、1970-01-01 からの日数を求める（Howard Hinnant の days_from_civil）。
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = i64::from((month + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `YYYY-MM-DD` を、その日の 0 時の RFC 822 の日時（`Sat, 26 Sep 2026 00:00:00 +0900`）にする。
pub fn rfc822(date: &str, offset_minutes: i32) -> String {
    let parts: Vec<i64> = date.split('-').filter_map(|p| p.parse().ok()).collect();
    let [year, month, day] = parts[..] else { return String::new() };
    let days = days_from_civil(year, month as u32, day as u32);
    const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let weekday = WEEKDAYS[days.rem_euclid(7) as usize];
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let offset = offset_minutes.abs();
    format!(
        "{weekday}, {day:02} {} {year:04} 00:00:00 {sign}{:02}{:02}",
        MONTHS[(month - 1) as usize],
        offset / 60,
        offset % 60
    )
}

/// 日時の時差。`timezone` を時差（`+09:00` など）で書いたときはそれを使い、名前で書いたときと書かないときは UTC にする。
pub fn offset_minutes(timezone: Option<&TimeZone>) -> i32 {
    match timezone {
        Some(TimeZone::Offset(minutes)) => *minutes,
        _ => 0,
    }
}

/// RSS 2.0 のフィード。url はサイトの URL（`/` で終わる）。
pub fn rss(site: &SiteConfig, url: &str, items: &[FeedItem]) -> String {
    let offset = offset_minutes(site.timezone().as_ref());
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n<channel>\n");
    xml.push_str(&format!("<title>{}</title>\n", escape(&site.title)));
    xml.push_str(&format!("<link>{}</link>\n", escape(url)));
    xml.push_str(&format!("<description>{}</description>\n", escape(&site.description())));
    xml.push_str("<language>ja</language>\n");
    xml.push_str(&format!(
        "<atom:link href=\"{}rss.xml\" rel=\"self\" type=\"application/rss+xml\" />\n",
        escape(url)
    ));
    if let Some(latest) = items.first() {
        xml.push_str(&format!("<lastBuildDate>{}</lastBuildDate>\n", rfc822(&latest.date, offset)));
    }
    for item in items {
        let link = format!("{url}{}", item.path);
        xml.push_str("<item>\n");
        xml.push_str(&format!("<title>{}</title>\n", escape(&item.title)));
        xml.push_str(&format!("<link>{}</link>\n", escape(&link)));
        xml.push_str(&format!("<guid isPermaLink=\"true\">{}</guid>\n", escape(&link)));
        xml.push_str(&format!("<pubDate>{}</pubDate>\n", rfc822(&item.date, offset)));
        if !item.summary.is_empty() {
            xml.push_str(&format!("<description>{}</description>\n", escape(&item.summary)));
        }
        xml.push_str("</item>\n");
    }
    xml.push_str("</channel>\n</rss>\n");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::site::parse_site_config;

    fn post(slug: &str, date: &str, body: &str) -> Post {
        Post { slug: slug.into(), title: format!("{slug} の題"), published_at: date.into(), body_md: body.into() }
    }

    fn article(slug: &str, date: &str, description: Option<&str>) -> Article {
        Article {
            slug: slug.into(),
            title: format!("{slug} & <題>"),
            published_at: date.into(),
            updated_at: None,
            description: description.map(str::to_string),
            body_md: "本文の**書き出し**。".into(),
        }
    }

    #[test]
    fn items_are_merged_newest_first_and_limited() {
        let posts: Vec<Post> = (1..=25).map(|n| post(&format!("p{n:02}"), &format!("2026-08-{n:02}"), "本文")).collect();
        let articles = [article("a", "2026-09-01", Some("要約")), article("b", "2026-08-25", None)];
        let items = items(&posts, &articles);
        assert_eq!(items.len(), MAX_ITEMS);
        assert_eq!(items[0].path, "articles/a");
        assert_eq!(items[0].summary, "要約");
        assert_eq!(items[1].path, "posts/p25");
        // 同じ日付（08-25）なら、パスの逆順（posts が先）。
        assert_eq!(items[2].path, "articles/b");
        assert_eq!(items[3].path, "posts/p24");
        let b = items.iter().find(|i| i.path == "articles/b").unwrap();
        assert_eq!(b.summary, "本文の書き出し。");
    }

    #[test]
    fn excerpt_strips_markdown_and_code() {
        let body = "## 見出し\n\n**強調**と[リンク](https://example.com)と![画像](/a.png)。<kbd>Ctrl</kbd>\n\n```sh\nsecret\n```\n\n- 箇条書き\n\n:::message\n中\n:::";
        assert_eq!(excerpt(body), "見出し 強調とリンクと画像。Ctrl 箇条書き 中");
        let long = "あ".repeat(200);
        assert_eq!(excerpt(&long), format!("{}…", "あ".repeat(EXCERPT_CHARS)));
    }

    #[test]
    fn dates_are_formatted_as_rfc822() {
        assert_eq!(rfc822("2026-09-26", 540), "Sat, 26 Sep 2026 00:00:00 +0900");
        assert_eq!(rfc822("1970-01-01", 0), "Thu, 01 Jan 1970 00:00:00 +0000");
        assert_eq!(rfc822("2024-02-29", -330), "Thu, 29 Feb 2024 00:00:00 -0530");
        assert_eq!(rfc822("2000-03-01", 0), "Wed, 01 Mar 2000 00:00:00 +0000");
    }

    #[test]
    fn rss_escapes_text_and_uses_absolute_links() {
        let site = parse_site_config("title = \"A & B\"\ntimezone = \"+09:00\"\n").unwrap();
        let items = items(&[post("hello", "2026-09-26", "こんにちは")], &[article("x", "2026-09-20", None)]);
        let xml = rss(&site, "https://example.com/blog/", &items);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\""));
        assert!(xml.contains("<title>A &amp; B</title>"));
        assert!(xml.contains("<link>https://example.com/blog/</link>"));
        assert!(xml.contains("<atom:link href=\"https://example.com/blog/rss.xml\" rel=\"self\""));
        assert!(xml.contains("<lastBuildDate>Sat, 26 Sep 2026 00:00:00 +0900</lastBuildDate>"));
        assert!(xml.contains("<link>https://example.com/blog/posts/hello</link>"));
        assert!(xml.contains("<guid isPermaLink=\"true\">https://example.com/blog/posts/hello</guid>"));
        assert!(xml.contains("<title>x &amp; &lt;題&gt;</title>"));
        assert!(xml.contains("<description>こんにちは</description>"));
        assert!(xml.trim_end().ends_with("</rss>"));
    }
}
