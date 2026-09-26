//! リンクカードの画像（ADR 0028）。
//!
//! 本文の URL だけの行（SPA がリンクカードにする行）の URL を集め、そのページの `og:image` の画像を取得する。
//! 画像は `/link-cards/<ハッシュ>.<拡張子>` として自分のサイトから配信し、閲覧のときに外部と通信しないようにする。
//! 取得した結果は記事リポジトリの `.sqlite-cms-cache/link-cards/` に保存し、次のビルドからはそれを使う。

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

/// 取得の結果を保存するディレクトリ（記事リポジトリからの相対パス）。
pub const CACHE_DIR: &str = ".sqlite-cms-cache";
const CARD_CACHE: &str = "link-cards";

/// 配信するときのパスの接頭辞。
pub const PATH_PREFIX: &str = "/link-cards/";

/// ページの HTML は先頭のこの大きさまでを読む（`og:image` は head にある）。
const MAX_HTML_BYTES: u64 = 1024 * 1024;
/// これより大きい画像は使わない。
const MAX_IMAGE_BYTES: u64 = 5 * 1024 * 1024;

/// 外部から取得したもの。
pub struct Fetched {
    /// 転送された後の URL。
    pub url: String,
    pub body: Vec<u8>,
}

/// 外部のページと画像を取得する。テストでは偽物に差し替える。
pub trait Fetcher {
    /// URL を取得する。本文は最大で max_bytes + 1 バイトまで読む（上限を超えたかを呼び出し側で確かめられるように）。
    fn fetch(&self, url: &str, max_bytes: u64) -> Result<Fetched>;
    /// 外部と通信しないなら true（保存済みの結果だけを使う）。
    fn offline(&self) -> bool {
        false
    }
}

pub struct UreqFetcher {
    agent: ureq::Agent,
}

impl UreqFetcher {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .user_agent(concat!("sqlite-cms/", env!("CARGO_PKG_VERSION"), " (link card)"))
            .build()
            .into();
        Self { agent }
    }
}

impl Fetcher for UreqFetcher {
    fn fetch(&self, url: &str, max_bytes: u64) -> Result<Fetched> {
        use ureq::ResponseExt;
        let mut response = self.agent.get(url).call()?;
        let final_url = response.get_uri().to_string();
        let mut body = Vec::new();
        response.body_mut().as_reader().take(max_bytes + 1).read_to_end(&mut body)?;
        Ok(Fetched { url: final_url, body })
    }
}

/// 外部と通信しない（環境変数 SQLITE_CMS_OFFLINE を設定したとき）。
pub struct Offline;

impl Fetcher for Offline {
    fn fetch(&self, _url: &str, _max_bytes: u64) -> Result<Fetched> {
        bail!("オフラインです")
    }
    fn offline(&self) -> bool {
        true
    }
}

/// 環境変数 SQLITE_CMS_OFFLINE が空でも "0" でもなければ、外部と通信しない。
pub fn fetcher_from_env() -> Box<dyn Fetcher> {
    match std::env::var("SQLITE_CMS_OFFLINE") {
        Ok(value) if !value.is_empty() && value != "0" => Box::new(Offline),
        _ => Box::new(UreqFetcher::new()),
    }
}

/// GFM の自動リンクが URL の末尾に含めない文字。これで終わる行は、URL だけの段落にならない。
const TRAILING_PUNCTUATION: &[char] = &['?', '!', '.', ',', ':', '*', '_', '~', ')', '\'', '"'];

fn is_card_url(candidate: &str, bracketed: bool) -> bool {
    let Some(rest) = candidate.strip_prefix("https://").or_else(|| candidate.strip_prefix("http://")) else {
        return false;
    };
    !rest.is_empty()
        && !candidate.chars().any(|c| c.is_whitespace() || c == '<' || c == '>')
        && (bracketed || !candidate.ends_with(TRAILING_PUNCTUATION))
}

/// 行頭の引用（`>`）とリストの記号（`-`、`*`、`+`、`1.`）を外す。
fn strip_block_markers(mut line: &str) -> &str {
    loop {
        line = line.trim_start();
        if let Some(rest) = line.strip_prefix('>') {
            line = rest;
            continue;
        }
        for marker in ["- ", "* ", "+ "] {
            if let Some(rest) = line.strip_prefix(marker) {
                return rest.trim();
            }
        }
        let digits = line.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 {
            if let Some(rest) = line[digits..].strip_prefix(". ").or_else(|| line[digits..].strip_prefix(") ")) {
                return rest.trim();
            }
        }
        return line.trim();
    }
}

/// 本文から、URL だけの行の URL を集める（SPA がリンクカードにするもの）。
/// コードブロックの中は除く。Markdown を厳密には解釈しないので、段落の途中の URL だけの行も拾う（画像が使われないだけ）。
pub fn card_urls(markdown: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in markdown.lines() {
        let trimmed = line.trim();
        let run = |c: char| trimmed.chars().take_while(|&x| x == c).count();
        if let Some((marker, size)) = fence {
            if run(marker) >= size && trimmed.trim_start_matches(marker).trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if let Some(marker) = ['`', '~'].into_iter().find(|&c| run(c) >= 3) {
            fence = Some((marker, run(marker)));
            continue;
        }
        let content = strip_block_markers(line);
        let (candidate, bracketed) = match content.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
            Some(inner) => (inner, true),
            None => (content, false),
        };
        if is_card_url(candidate, bracketed) {
            urls.push(candidate.to_string());
        }
    }
    urls
}

/// HTML の文字参照のうち、属性値でよく使うものを戻す。
fn decode_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';').filter(|&end| end <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// タグの中の属性を（小文字の名前、値）で返す。
fn attributes(tag: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = tag.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' && bytes[i] != b'/' {
            i += 1;
        }
        let name = tag[name_start..i].to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                let start = i + 1;
                let end = tag[start..].find(quote as char).map_or(tag.len(), |e| start + e);
                value = tag[start..end].to_string();
                i = end + 1;
            } else {
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                value = tag[start..i].to_string();
            }
        }
        if !name.is_empty() {
            out.push((name, decode_entities(&value)));
        }
    }
    out
}

/// ページの画像を表す meta の名前。前にあるものほど優先する。
const IMAGE_PROPERTIES: &[&str] = &["og:image", "og:image:url", "og:image:secure_url", "twitter:image", "twitter:image:src"];

/// HTML の meta から、ページの画像の URL（書かれたまま）を探す。
pub fn og_image(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut from = 0;
    while let Some(start) = lower[from..].find("<meta").map(|i| from + i) {
        let body_start = start + "<meta".len();
        let Some(end) = lower[body_start..].find('>').map(|i| body_start + i) else { break };
        let attrs = attributes(&html[body_start..end]);
        let key = attrs.iter().find(|(name, _)| name == "property" || name == "name").map(|(_, v)| v.to_ascii_lowercase());
        let content = attrs.iter().find(|(name, _)| name == "content").map(|(_, v)| v.trim().to_string());
        if let (Some(key), Some(content)) = (key, content) {
            if let Some(rank) = IMAGE_PROPERTIES.iter().position(|p| *p == key) {
                if !content.is_empty() {
                    found.push((rank, content));
                }
            }
        }
        from = end;
    }
    found.into_iter().min_by_key(|(rank, _)| *rank).map(|(_, url)| url)
}

/// ページの URL を基準に、画像の URL を絶対 URL にする。http と https 以外は使わない。
pub fn resolve_url(base: &str, href: &str) -> Option<String> {
    let href = href.trim();
    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(href.to_string());
    }
    let (scheme, rest) = base.split_once("://")?;
    if let Some(without_scheme) = href.strip_prefix("//") {
        return Some(format!("{scheme}://{without_scheme}"));
    }
    // ほかのスキーム（data:、javascript: など）は使わない。
    if href.split(['/', '?', '#']).next().is_some_and(|head| head.contains(':')) || href.is_empty() {
        return None;
    }
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let origin = format!("{scheme}://{}", &rest[..host_end]);
    if href.starts_with('/') {
        return Some(format!("{origin}{href}"));
    }
    let path = rest[host_end..].split(['?', '#']).next().unwrap_or("");
    let dir = path.rfind('/').map_or("/", |i| &path[..=i]);
    Some(format!("{origin}{dir}{href}"))
}

/// 画像の形式を、ファイルの先頭のバイトで確かめる。配信する拡張子を返す。
pub fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && (&bytes[8..12] == b"avif" || &bytes[8..12] == b"avis") {
        Some("avif")
    } else {
        None
    }
}

/// URL から作る、ファイル名に使う名前。
fn key(url: &str) -> String {
    blake3::hash(url.as_bytes()).to_hex()[..16].to_string()
}

/// 保存する取得の結果。image が None なら、ページに画像がなかった。
#[derive(Serialize, Deserialize)]
struct CacheEntry {
    url: String,
    image: Option<String>,
}

/// リンクカードの画像。
pub struct LinkCard {
    pub url: String,
    /// 配信するパス（`/link-cards/<ハッシュ>.<拡張子>`）。
    pub path: String,
    pub bytes: Vec<u8>,
}

fn cache_dir(site_dir: &Path) -> PathBuf {
    site_dir.join(CACHE_DIR).join(CARD_CACHE)
}

/// 保存済みの結果を読む。Some(None) はページに画像がなかったこと、None はまだ取得していないことを表す。
fn read_cache(site_dir: &Path, url: &str) -> Option<Option<LinkCard>> {
    let dir = cache_dir(site_dir);
    let entry: CacheEntry = serde_json::from_slice(&fs::read(dir.join(format!("{}.json", key(url)))).ok()?).ok()?;
    if entry.url != url {
        return None;
    }
    let Some(file) = entry.image else { return Some(None) };
    let bytes = fs::read(dir.join(&file)).ok()?;
    Some(Some(LinkCard { url: url.to_string(), path: format!("{PATH_PREFIX}{file}"), bytes }))
}

fn write_cache(site_dir: &Path, url: &str, card: Option<&LinkCard>) -> Result<()> {
    let dir = cache_dir(site_dir);
    fs::create_dir_all(&dir).with_context(|| format!("{} を作れません", dir.display()))?;
    let image = card.map(|card| card.path.trim_start_matches(PATH_PREFIX).to_string());
    if let (Some(card), Some(file)) = (card, &image) {
        fs::write(dir.join(file), &card.bytes)?;
    }
    let entry = CacheEntry { url: url.to_string(), image };
    fs::write(dir.join(format!("{}.json", key(url))), serde_json::to_vec_pretty(&entry)?)?;
    Ok(())
}

/// ページの画像を取得する。ページに画像がなければ Ok(None)。
fn fetch_card(fetcher: &dyn Fetcher, url: &str) -> Result<Option<LinkCard>> {
    let page = fetcher.fetch(url, MAX_HTML_BYTES)?;
    // head にある meta を読めれば足りるので、上限を超えた分は捨てる。
    let html = String::from_utf8_lossy(&page.body[..page.body.len().min(MAX_HTML_BYTES as usize)]);
    let Some(href) = og_image(&html) else { return Ok(None) };
    let image_url = resolve_url(&page.url, &href).ok_or_else(|| anyhow!("画像の URL が不正です: {href}"))?;
    let image = fetcher.fetch(&image_url, MAX_IMAGE_BYTES).with_context(|| format!("画像 {image_url}"))?;
    if image.body.len() as u64 > MAX_IMAGE_BYTES {
        bail!("画像 {image_url} が {MAX_IMAGE_BYTES} バイトを超えています");
    }
    let extension =
        image_extension(&image.body).ok_or_else(|| anyhow!("画像 {image_url} は PNG、JPEG、GIF、WebP、AVIF のどれでもありません"))?;
    Ok(Some(LinkCard { url: url.to_string(), path: format!("{PATH_PREFIX}{}.{extension}", key(url)), bytes: image.body }))
}

/// 本文のリンクカードの画像を集める。保存済みのものはそれを使い、なければ取得して保存する。
/// 取得できなかったものは警告を出して飛ばす（ビルドは止めない）。
pub fn collect(site_dir: &Path, sources: &[String], fetcher: &dyn Fetcher) -> Vec<LinkCard> {
    let urls: BTreeSet<String> = sources.iter().flat_map(|source| card_urls(source)).collect();
    let mut cards = Vec::new();
    for url in urls {
        if let Some(cached) = read_cache(site_dir, &url) {
            cards.extend(cached);
            continue;
        }
        if fetcher.offline() {
            continue;
        }
        eprintln!("リンクカードの画像を取得しています: {url}");
        match fetch_card(fetcher, &url) {
            Ok(card) => {
                if let Err(error) = write_cache(site_dir, &url, card.as_ref()) {
                    eprintln!("警告: リンクカードの画像を保存できませんでした: {error:#}");
                }
                cards.extend(card);
            }
            Err(error) => eprintln!("警告: リンクカードの画像を取得できませんでした: {url}: {error:#}"),
        }
    }
    cards
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n rest of png";

    struct FakeFetcher {
        responses: HashMap<String, (String, Vec<u8>)>,
        requested: RefCell<Vec<String>>,
    }

    impl FakeFetcher {
        fn new(responses: &[(&str, &str, &[u8])]) -> Self {
            FakeFetcher {
                responses: responses.iter().map(|(url, final_url, body)| (url.to_string(), (final_url.to_string(), body.to_vec()))).collect(),
                requested: RefCell::new(Vec::new()),
            }
        }
    }

    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str, _max_bytes: u64) -> Result<Fetched> {
            self.requested.borrow_mut().push(url.to_string());
            let (final_url, body) = self.responses.get(url).ok_or_else(|| anyhow!("404"))?;
            Ok(Fetched { url: final_url.clone(), body: body.clone() })
        }
    }

    #[test]
    fn collects_url_only_lines_outside_code() {
        let markdown = "\
本文 https://inline.example.com は含めない。

https://example.com/a

<https://example.com/b?q=1>

- https://example.com/in-list
> https://example.com/in-quote

https://example.com/trailing.

```
https://example.com/in-code
```

~~~~
https://example.com/in-tilde
~~~~

ftp://example.com/x
https://
";
        assert_eq!(
            card_urls(markdown),
            [
                "https://example.com/a",
                "https://example.com/b?q=1",
                "https://example.com/in-list",
                "https://example.com/in-quote",
            ]
        );
    }

    #[test]
    fn finds_og_image_with_priority_and_entities() {
        let html = r#"<html><head>
<META name="twitter:image" content="/twitter.png">
<meta content='https://cdn.example.com/og.png?a=1&amp;b=2' property="og:image" />
<meta property=og:title content=title>
</head></html>"#;
        assert_eq!(og_image(html).as_deref(), Some("https://cdn.example.com/og.png?a=1&b=2"));
        assert_eq!(og_image(r#"<meta name="twitter:image" content="/t.png">"#).as_deref(), Some("/t.png"));
        assert_eq!(og_image(r#"<meta property="og:image" content="">"#), None);
        assert_eq!(og_image("<p>no meta</p>"), None);
    }

    #[test]
    fn decodes_numeric_and_named_entities() {
        assert_eq!(decode_entities("a&amp;b&#38;c&#x26;d&lt;&gt;&quot;&apos;"), "a&b&c&d<>\"'");
        assert_eq!(decode_entities("a & b &unknown; &#xZZ;"), "a & b &unknown; &#xZZ;");
    }

    #[test]
    fn resolves_relative_image_urls() {
        let base = "https://example.com/posts/hello?x=1";
        assert_eq!(resolve_url(base, "https://cdn.example.com/a.png").as_deref(), Some("https://cdn.example.com/a.png"));
        assert_eq!(resolve_url(base, "//cdn.example.com/a.png").as_deref(), Some("https://cdn.example.com/a.png"));
        assert_eq!(resolve_url(base, "/images/a.png").as_deref(), Some("https://example.com/images/a.png"));
        assert_eq!(resolve_url(base, "a.png").as_deref(), Some("https://example.com/posts/a.png"));
        assert_eq!(resolve_url("https://example.com", "a.png").as_deref(), Some("https://example.com/a.png"));
        assert_eq!(resolve_url(base, "data:image/png;base64,xx"), None);
        assert_eq!(resolve_url(base, "javascript:alert(1)"), None);
        assert_eq!(resolve_url(base, ""), None);
    }

    #[test]
    fn detects_image_formats_by_signature() {
        assert_eq!(image_extension(PNG), Some("png"));
        assert_eq!(image_extension(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(image_extension(b"GIF89a..."), Some("gif"));
        assert_eq!(image_extension(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(image_extension(b"\0\0\0\x1cftypavif"), Some("avif"));
        assert_eq!(image_extension(b"<!doctype html>"), None);
        assert_eq!(image_extension(b"<svg></svg>"), None);
    }

    #[test]
    fn fetches_images_caches_them_and_skips_failures() {
        let site = crate::testutil::tempdir();
        let fetcher = FakeFetcher::new(&[
            ("https://example.com/a", "https://example.com/a/", br#"<meta property="og:image" content="og.png">"#),
            ("https://example.com/a/og.png", "https://example.com/a/og.png", PNG),
            ("https://example.com/no-image", "https://example.com/no-image", b"<title>x</title>"),
            ("https://example.com/html-image", "https://example.com/html-image", br#"<meta property="og:image" content="/x">"#),
            ("https://example.com/x", "https://example.com/x", b"<html>not an image</html>"),
        ]);
        let sources = [
            "https://example.com/a\n\nhttps://example.com/no-image\n".to_string(),
            "https://example.com/html-image\n\nhttps://example.com/down\n\nhttps://example.com/a\n".to_string(),
        ];

        let cards = collect(site.path(), &sources, &fetcher);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].url, "https://example.com/a");
        assert_eq!(cards[0].path, format!("/link-cards/{}.png", key("https://example.com/a")));
        assert_eq!(cards[0].bytes, PNG);

        // 保存した結果を使い、画像のあるページとないページは取得し直さない。取得できなかったものは取得し直す。
        fetcher.requested.borrow_mut().clear();
        let cards = collect(site.path(), &sources, &fetcher);
        assert_eq!(cards.len(), 1);
        assert_eq!(
            *fetcher.requested.borrow(),
            ["https://example.com/down", "https://example.com/html-image", "https://example.com/x"]
        );

        // オフラインでも、保存した結果は使う。
        let cards = collect(site.path(), &sources, &Offline);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].bytes, PNG);
    }
}
