//! サイトマップ（ADR 0037）。
//!
//! サイトは SPA で、ページの一覧を HTML のリンクからたどれない（DB を読んだ後に描く）ので、
//! 検索エンジンにページの一覧を知らせるため `sitemap.xml` を作る。
//! URL は絶対 URL である必要があるので、`site.toml` の `url` を書いたときだけ作る（RSS と同じ、ADR 0033）。

use crate::content::{Article, Page, Post};

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// サイトマップに載せる URL と、最終更新日（あれば）。
struct Entry {
    location: String,
    last_modified: Option<String>,
}

/// url はサイトの URL（`/` で終わる）。トップ、一覧、自己紹介、post、article を載せる。
/// 検索のページ（結果が言葉で変わる）と、URL のないページは載せない。
pub fn sitemap(url: &str, posts: &[Post], articles: &[Article], pages: &[Page]) -> String {
    let latest = posts
        .iter()
        .map(|p| p.published_at.clone())
        .chain(articles.iter().map(|a| a.updated_at.clone().unwrap_or_else(|| a.published_at.clone())))
        .max();
    let mut entries = vec![
        Entry { location: url.to_string(), last_modified: latest.clone() },
        Entry { location: format!("{url}archive"), last_modified: latest },
    ];
    if pages.iter().any(|p| p.slug == "about") {
        entries.push(Entry { location: format!("{url}about"), last_modified: None });
    }
    entries.extend(articles.iter().map(|a| Entry {
        location: format!("{url}articles/{}", a.slug),
        last_modified: Some(a.updated_at.clone().unwrap_or_else(|| a.published_at.clone())),
    }));
    entries.extend(
        posts.iter().map(|p| Entry { location: format!("{url}posts/{}", p.slug), last_modified: Some(p.published_at.clone()) }),
    );

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for entry in entries {
        xml.push_str(&format!("<url><loc>{}</loc>", escape(&entry.location)));
        if let Some(date) = entry.last_modified {
            xml.push_str(&format!("<lastmod>{date}</lastmod>"));
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

/// robots.txt に、サイトマップの場所（`Sitemap:` の行）がなければ足す。書き手の robots.txt にすでにあれば変えない。
pub fn add_to_robots(robots: &str, url: &str) -> String {
    let has_sitemap = robots.lines().any(|line| line.trim_start().to_ascii_lowercase().starts_with("sitemap:"));
    if has_sitemap {
        return robots.to_string();
    }
    let separator = if robots.is_empty() || robots.ends_with('\n') { "" } else { "\n" };
    format!("{robots}{separator}\nSitemap: {url}sitemap.xml\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(slug: &str, date: &str) -> Post {
        Post { slug: slug.into(), title: "t".into(), published_at: date.into(), body_md: String::new() }
    }

    fn article(slug: &str, date: &str, updated: Option<&str>) -> Article {
        Article {
            slug: slug.into(),
            title: "t".into(),
            published_at: date.into(),
            updated_at: updated.map(str::to_string),
            description: None,
            body_md: String::new(),
        }
    }

    #[test]
    fn lists_pages_with_last_modified_dates() {
        let about = Page { slug: "about".into(), title: "t".into(), body_md: String::new() };
        let draft = Page { slug: "draft".into(), title: "t".into(), body_md: String::new() };
        let xml = sitemap(
            "https://example.com/blog/",
            &[post("hello", "2026-09-16")],
            &[article("long", "2026-09-15", Some("2026-09-20"))],
            &[about, draft],
        );
        assert_eq!(
            xml,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
             <url><loc>https://example.com/blog/</loc><lastmod>2026-09-20</lastmod></url>\n\
             <url><loc>https://example.com/blog/archive</loc><lastmod>2026-09-20</lastmod></url>\n\
             <url><loc>https://example.com/blog/about</loc></url>\n\
             <url><loc>https://example.com/blog/articles/long</loc><lastmod>2026-09-20</lastmod></url>\n\
             <url><loc>https://example.com/blog/posts/hello</loc><lastmod>2026-09-16</lastmod></url>\n\
             </urlset>\n"
        );
    }

    #[test]
    fn empty_site_has_top_and_archive_without_dates() {
        let xml = sitemap("https://example.com/", &[], &[], &[]);
        assert!(xml.contains("<url><loc>https://example.com/</loc></url>"), "{xml}");
        assert!(xml.contains("<url><loc>https://example.com/archive</loc></url>"), "{xml}");
    }

    #[test]
    fn robots_gets_a_sitemap_line_once() {
        let robots = "User-agent: *\nAllow: /\n";
        let added = add_to_robots(robots, "https://example.com/");
        assert_eq!(added, "User-agent: *\nAllow: /\n\nSitemap: https://example.com/sitemap.xml\n");
        assert_eq!(add_to_robots(&added, "https://example.com/"), added);
        assert_eq!(add_to_robots("SITEMAP: https://other.example/map.xml", "https://example.com/"), "SITEMAP: https://other.example/map.xml");
        assert_eq!(add_to_robots("User-agent: *", "https://example.com/"), "User-agent: *\n\nSitemap: https://example.com/sitemap.xml\n");
    }
}
