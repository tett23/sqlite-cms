use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::linkcard::{self, Fetcher};
use crate::{db, favicon, feed, media, robots, site, sitemap};

pub const HEADERS: &str = "\
/db/*.sqlite
  Cache-Control: public, max-age=31536000, immutable

/db/manifest.json
  Cache-Control: no-cache
";

/// sqlite-cms が書き出した場所の目印（build の出力先と rsync の送り先）。
pub const MARKER: &str = ".sqlite-cms";

/// content/favicon.svg があればそれを、なければサイト名から作った仮の favicon を返す。
fn read_favicon(site_dir: &Path) -> Result<Vec<u8>> {
    let path = site_dir.join("content").join("favicon.svg");
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(favicon::placeholder(&site::read_site_config(site_dir)?.title).into_bytes())
        }
        Err(e) => Err(e).with_context(|| format!("{} を読めません", path.display())),
    }
}

/// content/robots.txt があればそれを、なければ既定の robots.txt（すべて許可）を返す（ADR 0037）。
fn read_robots(site_dir: &Path) -> Result<Vec<u8>> {
    let path = site_dir.join("content").join("robots.txt");
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(robots::DEFAULT.as_bytes().to_vec()),
        Err(e) => Err(e).with_context(|| format!("{} を読めません", path.display())),
    }
}

/// SPA の index.html を、サイトを置くパス（ADR 0030）に合わせる。
/// - SPA は相対のパス（`./assets/…`）でビルドしてあるので、`base_path` から始まるパスにする。深いパスの URL で 404.html として返しても読めるようにするためである。
/// - favicon のパス（`/favicon.svg`）も `base_path` から始める。
/// - SPA が DB や画像やページのパスを組み立てられるよう、`<meta name="sqlite-cms-base">` を入れる。
pub fn apply_base_path(html: &str, base_path: &str) -> String {
    let html = html
        .replace("=\"./", &format!("=\"{base_path}"))
        .replace("href=\"/favicon.svg\"", &format!("href=\"{base_path}favicon.svg\""));
    let meta = format!("<meta name=\"sqlite-cms-base\" content=\"{base_path}\" />\n    ");
    match html.find("<title>") {
        Some(index) => format!("{}{meta}{}", &html[..index], &html[index..]),
        None => html.replacen("</head>", &format!("{meta}</head>"), 1),
    }
}

/// index.html に、RSS のフィードの案内（`<link rel="alternate">`）を入れる。SPA はこれを見てフッターにリンクを出す。
pub fn add_feed_link(html: &str, base_path: &str, title: &str) -> String {
    let title = title.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;");
    let link = format!("<link rel=\"alternate\" type=\"application/rss+xml\" title=\"{title}\" href=\"{base_path}rss.xml\" />\n    ");
    match html.find("<title>") {
        Some(index) => format!("{}{link}{}", &html[..index], &html[index..]),
        None => html.replacen("</head>", &format!("{link}</head>"), 1),
    }
}

/// serve で記事の変更を反映したときに、ブラウザを再読み込みさせるスクリプト（ADR 0034）。
pub fn inject_live_reload(html: &str, base_path: &str) -> String {
    let script = format!(
        "<script>new EventSource(\"{base_path}{}\").addEventListener(\"reload\", () => location.reload());</script>\n  ",
        crate::serve::EVENTS_PATH
    );
    match html.rfind("</body>") {
        Some(index) => format!("{}{script}{}", &html[..index], &html[index..]),
        None => format!("{html}{script}"),
    }
}

/// index.html に、DB の先読みと、DB のパスを入れる（ADR 0038）。
/// SPA の JS を読み込んで実行するのを待たずに、DB の取得を始めるためである。
/// SPA は DB のパスがあればマニフェストを読まずに DB を取りに行く（取れなければマニフェストから読み直す）。
/// sql.js の wasm は先読みしない。ページの表示には自前の DB の読み手を使い、sql.js は検索のときだけ読み込む（ADR 0047）。
pub fn add_preloads(html: &str, base_path: &str, db_path: &str) -> String {
    let db = format!("{base_path}{}", db_path.trim_start_matches('/'));
    let tags = format!(
        "<meta name=\"sqlite-cms-db\" content=\"{db}\" />\n    <link rel=\"preload\" href=\"{db}\" as=\"fetch\" crossorigin=\"anonymous\" />\n    "
    );
    match html.find("<title>") {
        Some(index) => format!("{}{tags}{}", &html[..index], &html[index..]),
        None => html.replacen("</head>", &format!("{tags}</head>"), 1),
    }
}

/// index.html に、ヘッダを記事リポジトリの content/header.md で作ることを示す目印を入れる（ADR 0043）。
/// SPA は、これがあれば DB を読むまでヘッダを空にしておく。既定のヘッダを先に描くと、DB を読んだときに差し替わり、表示がずれるためである。
pub fn add_custom_header_mark(html: &str) -> String {
    let tag = "<meta name=\"sqlite-cms-header\" content=\"custom\" />\n    ";
    match html.find("<title>") {
        Some(index) => format!("{}{tag}{}", &html[..index], &html[index..]),
        None => html.replacen("</head>", &format!("{tag}</head>"), 1),
    }
}

pub struct SiteOutput {
    pub files: BTreeMap<String, Vec<u8>>,
}

impl SiteOutput {
    /// DB、マニフェスト、画像、favicon、リンクカードの画像を組み立てる。リンクカードの画像は fetcher で取得する。
    pub fn data(site_dir: &Path, fetcher: &dyn Fetcher) -> Result<Self> {
        let cards = linkcard::collect(site_dir, &db::markdown_sources(site_dir)?, fetcher);
        let media = media::read_media(&site_dir.join("content").join("media"))?;
        let bytes = db::build_db_bytes(site_dir, &cards, &media::sizes(&media))?;
        let db_path = format!("/db/{}", db::db_file_name(&bytes));

        let mut files = BTreeMap::new();
        files.insert("/db/manifest.json".to_string(), format!("{{\"db\":\"{db_path}\"}}\n").into_bytes());
        files.insert(db_path, bytes);
        for (path, bytes) in media {
            files.insert(format!("/media/{path}"), bytes);
        }
        files.insert("/favicon.svg".to_string(), read_favicon(site_dir)?);
        files.insert("/robots.txt".to_string(), read_robots(site_dir)?);
        for card in cards {
            files.insert(card.path, card.bytes);
        }
        Ok(Self { files })
    }

    pub fn site(site_dir: &Path, spa: &[(&str, &[u8])], fetcher: &dyn Fetcher) -> Result<Self> {
        if !spa.iter().any(|(path, _)| *path == "/index.html") {
            bail!(
                "この sqlite-cms は SPA を含まずにビルドされています。\
                 リリースのバイナリを使うか、開発者向けの手順（DEVELOPMENT.md）でビルドし直してください"
            );
        }
        let config = site::read_site_config(site_dir)?;
        let base_path = config.base_path();
        let mut output = Self::data(site_dir, fetcher)?;
        for (path, bytes) in spa {
            output.files.entry(path.to_string()).or_insert_with(|| bytes.to_vec());
        }
        let mut index = apply_base_path(&String::from_utf8_lossy(&output.files["/index.html"]), &base_path);
        let db_path = output.files.keys().find(|path| path.starts_with("/db/articles-")).cloned().expect("DB がない");
        index = add_preloads(&index, &base_path, &db_path);
        if site_dir.join("content").join("header.md").is_file() {
            index = add_custom_header_mark(&index);
        }
        // url を書いたときは RSS のフィードを作り、index.html に案内を入れる（ADR 0033）。
        // サイトマップも作り、robots.txt にその場所を足す（ADR 0037）。
        if let Some(url) = config.url() {
            let (posts, articles) = db::posts_and_articles(site_dir)?;
            let rss = feed::rss(&config, &url, &feed::items(&posts, &articles));
            output.files.insert("/rss.xml".to_string(), rss.into_bytes());
            index = add_feed_link(&index, &base_path, &config.title);

            let map = sitemap::sitemap(&url, &posts, &articles, &db::pages(site_dir)?);
            output.files.insert("/sitemap.xml".to_string(), map.into_bytes());
            let robots = String::from_utf8_lossy(&output.files["/robots.txt"]).into_owned();
            output.files.insert("/robots.txt".to_string(), sitemap::add_to_robots(&robots, &url).into_bytes());
        }
        let index = index.into_bytes();
        // SPA のフォールバックの設定がない配信先（GitHub Pages など）では、知らないパスに 404.html が返る。
        output.files.insert("/404.html".to_string(), index.clone());
        output.files.insert("/index.html".to_string(), index);
        Ok(output)
    }

    /// serve で使う。index.html と 404.html に再読み込みのスクリプトを入れる。
    pub fn with_live_reload(mut self, base_path: &str) -> Self {
        for path in ["/index.html", "/404.html"] {
            if let Some(bytes) = self.files.get(path) {
                let html = inject_live_reload(&String::from_utf8_lossy(bytes), base_path);
                self.files.insert(path.to_string(), html.into_bytes());
            }
        }
        self
    }

    pub fn total_bytes(&self) -> usize {
        self.files.values().map(Vec::len).sum()
    }

    pub fn write_site(&self, out_dir: &Path) -> Result<()> {
        if out_dir.exists() {
            let is_empty = fs::read_dir(out_dir)?.next().is_none();
            if !is_empty && !out_dir.join(MARKER).is_file() {
                bail!(
                    "{} は空ではなく、sqlite-cms の出力先でもありません。\
                     別のディレクトリを指定するか、中身を確かめてから消してください",
                    out_dir.display()
                );
            }
            fs::remove_dir_all(out_dir)?;
        }
        self.write_files(out_dir)?;
        fs::write(out_dir.join("_headers"), HEADERS)?;
        fs::write(out_dir.join(MARKER), "sqlite-cms の出力先。ビルドのたびに作り直される。\n")?;
        Ok(())
    }

    pub fn write_data(&self, out_dir: &Path) -> Result<()> {
        for dir in ["db", "media", linkcard::PATH_PREFIX.trim_matches('/')] {
            let dir = out_dir.join(dir);
            if dir.exists() {
                fs::remove_dir_all(&dir)?;
            }
        }
        self.write_files(out_dir)
    }

    fn write_files(&self, out_dir: &Path) -> Result<()> {
        for (path, bytes) in &self.files {
            let target = out_dir.join(path.trim_start_matches('/'));
            fs::create_dir_all(target.parent().unwrap())?;
            fs::write(&target, bytes).with_context(|| format!("{} に書き込めません", target.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPA: &[(&str, &[u8])] = &[("/index.html", b"<html>"), ("/assets/app.js", b"js")];

    fn site_fixture() -> crate::testutil::TempDir {
        let dir = crate::testutil::tempdir();
        fs::write(dir.path().join("site.toml"), "title = \"t\"\n").unwrap();
        fs::create_dir_all(dir.path().join("content/posts")).unwrap();
        fs::create_dir_all(dir.path().join("content/media")).unwrap();
        fs::write(dir.path().join("content/media/a.png"), b"png").unwrap();
        dir
    }

    fn paths(output: &SiteOutput) -> Vec<&str> {
        output.files.keys().map(String::as_str).collect()
    }

    fn files_under(dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in fs::read_dir(d).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.push(path.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn base_path_is_applied_to_index_html() {
        let html = "<head>\n    <link rel=\"icon\" href=\"/favicon.svg\" />\n    <title></title>\n    <script type=\"module\" src=\"./assets/index.js\"></script>\n    <link rel=\"stylesheet\" href=\"./assets/index.css\">\n</head>";
        let root = apply_base_path(html, "/");
        assert!(root.contains("src=\"/assets/index.js\""), "{root}");
        assert!(root.contains("href=\"/assets/index.css\""), "{root}");
        assert!(root.contains("href=\"/favicon.svg\""), "{root}");
        assert!(root.contains("<meta name=\"sqlite-cms-base\" content=\"/\" />\n    <title>"), "{root}");

        let sub = apply_base_path(html, "/my-blog/");
        assert!(sub.contains("src=\"/my-blog/assets/index.js\""), "{sub}");
        assert!(sub.contains("href=\"/my-blog/assets/index.css\""), "{sub}");
        assert!(sub.contains("href=\"/my-blog/favicon.svg\""), "{sub}");
        assert!(sub.contains("content=\"/my-blog/\""), "{sub}");
    }

    #[test]
    fn rss_is_written_only_when_the_url_is_set() {
        let site = site_fixture();
        let spa: &[(&str, &[u8])] = &[("/index.html", b"<head><title></title></head>")];
        let output = SiteOutput::site(site.path(), spa, &linkcard::Offline).unwrap();
        assert!(!output.files.contains_key("/rss.xml"));
        assert!(!output.files.contains_key("/sitemap.xml"));
        assert!(!String::from_utf8_lossy(&output.files["/robots.txt"]).contains("Sitemap:"));
        assert!(!String::from_utf8_lossy(&output.files["/index.html"]).contains("rss.xml"));

        fs::write(site.path().join("site.toml"), "title = \"A & B\"\nurl = \"https://example.com\"\n").unwrap();
        fs::write(site.path().join("content/posts/hello.md"), "---\ntitle: こんにちは\ndate: 2026-09-26\n---\n\n本文。\n").unwrap();
        let output = SiteOutput::site(site.path(), spa, &linkcard::Offline).unwrap();
        let map = String::from_utf8(output.files["/sitemap.xml"].clone()).unwrap();
        assert!(map.contains("<loc>https://example.com/posts/hello</loc><lastmod>2026-09-26</lastmod>"), "{map}");
        assert!(String::from_utf8_lossy(&output.files["/robots.txt"]).ends_with("\nSitemap: https://example.com/sitemap.xml\n"));
        let rss = String::from_utf8(output.files["/rss.xml"].clone()).unwrap();
        assert!(rss.contains("<link>https://example.com/posts/hello</link>"), "{rss}");
        let index = String::from_utf8(output.files["/index.html"].clone()).unwrap();
        assert!(
            index.contains("<link rel=\"alternate\" type=\"application/rss+xml\" title=\"A &amp; B\" href=\"/rss.xml\" />"),
            "{index}"
        );
        assert_eq!(output.files["/404.html"], output.files["/index.html"]);
    }

    #[test]
    fn live_reload_script_is_injected_before_the_body_end() {
        let html = inject_live_reload("<body>\n  <div id=\"root\"></div>\n</body>", "/blog/");
        assert_eq!(
            html,
            "<body>\n  <div id=\"root\"></div>\n<script>new EventSource(\"/blog/__sqlite-cms/events\").addEventListener(\"reload\", () => location.reload());</script>\n  </body>"
        );
        let site = site_fixture();
        let spa: &[(&str, &[u8])] = &[("/index.html", b"<head><title></title></head><body></body>")];
        let output = SiteOutput::site(site.path(), spa, &linkcard::Offline).unwrap().with_live_reload("/");
        for path in ["/index.html", "/404.html"] {
            assert!(String::from_utf8_lossy(&output.files[path]).contains("EventSource(\"/__sqlite-cms/events\")"), "{path}");
        }
    }

    #[test]
    fn custom_header_is_marked_in_index_html() {
        let spa: &[(&str, &[u8])] = &[("/index.html", b"<head><title></title></head>")];
        let site = site_fixture();
        let output = SiteOutput::site(site.path(), spa, &linkcard::Offline).unwrap();
        assert!(!String::from_utf8_lossy(&output.files["/index.html"]).contains("sqlite-cms-header"));

        fs::write(site.path().join("content/header.md"), "[{{title}}](/)\n").unwrap();
        let output = SiteOutput::site(site.path(), spa, &linkcard::Offline).unwrap();
        let index = String::from_utf8_lossy(&output.files["/index.html"]).into_owned();
        assert!(index.contains("<meta name=\"sqlite-cms-header\" content=\"custom\" />\n    <title>"), "{index}");
    }

    #[test]
    fn index_preloads_the_db_but_not_the_wasm() {
        let html = add_preloads("<head>\n    <title></title>\n</head>", "/blog/", "/db/articles-abc.sqlite");
        assert!(html.contains("<meta name=\"sqlite-cms-db\" content=\"/blog/db/articles-abc.sqlite\" />"), "{html}");
        assert!(html.contains("<link rel=\"preload\" href=\"/blog/db/articles-abc.sqlite\" as=\"fetch\" crossorigin=\"anonymous\" />"), "{html}");

        let site = site_fixture();
        let spa: &[(&str, &[u8])] = &[("/index.html", b"<head><title></title></head>"), ("/assets/sql-wasm-q.wasm", b"wasm")];
        let output = SiteOutput::site(site.path(), spa, &linkcard::Offline).unwrap();
        let index = String::from_utf8(output.files["/index.html"].clone()).unwrap();
        let db = output.files.keys().find(|p| p.starts_with("/db/articles-")).unwrap();
        assert!(index.contains(&format!("content=\"{db}\"")), "{index}");
        // 表示に使わない sql.js の wasm は、先読みしない（ADR 0047）。
        assert!(!index.contains("sql-wasm"), "{index}");
    }

    #[test]
    fn site_has_a_404_page_equal_to_the_index() {
        let site = site_fixture();
        fs::write(site.path().join("site.toml"), "title = \"t\"\nbase_path = \"/blog\"\n").unwrap();
        let spa: &[(&str, &[u8])] = &[("/index.html", b"<head><title></title><script src=\"./assets/app.js\"></script></head>")];
        let output = SiteOutput::site(site.path(), spa, &linkcard::Offline).unwrap();
        let index = String::from_utf8(output.files["/index.html"].clone()).unwrap();
        assert!(index.contains("src=\"/blog/assets/app.js\""), "{index}");
        assert_eq!(output.files["/404.html"], output.files["/index.html"]);
    }

    #[test]
    fn data_contains_db_manifest_and_media() {
        let site = site_fixture();
        let output = SiteOutput::data(site.path(), &linkcard::Offline).unwrap();
        let paths = paths(&output);
        assert_eq!(paths.len(), 5);
        assert!(paths[0].starts_with("/db/articles-"));
        assert_eq!(&paths[1..], ["/db/manifest.json", "/favicon.svg", "/media/a.png", "/robots.txt"]);
        let manifest = String::from_utf8(output.files["/db/manifest.json"].clone()).unwrap();
        assert_eq!(manifest, format!("{{\"db\":\"{}\"}}\n", paths[0]));
    }

    #[test]
    fn favicon_comes_from_content_or_falls_back_to_a_placeholder() {
        let site = site_fixture();
        let output = SiteOutput::data(site.path(), &linkcard::Offline).unwrap();
        assert_eq!(output.files["/favicon.svg"], crate::favicon::placeholder("t").into_bytes());

        fs::write(site.path().join("content/favicon.svg"), b"<svg>mine</svg>").unwrap();
        let output = SiteOutput::data(site.path(), &linkcard::Offline).unwrap();
        assert_eq!(output.files["/favicon.svg"], b"<svg>mine</svg>");
    }

    #[test]
    fn robots_comes_from_content_or_falls_back_to_allow_all() {
        let site = site_fixture();
        let output = SiteOutput::data(site.path(), &linkcard::Offline).unwrap();
        assert_eq!(output.files["/robots.txt"], crate::robots::DEFAULT.as_bytes());

        fs::write(site.path().join("content/robots.txt"), b"User-agent: *\nDisallow: /drafts/\n").unwrap();
        let output = SiteOutput::data(site.path(), &linkcard::Offline).unwrap();
        assert_eq!(output.files["/robots.txt"], b"User-agent: *\nDisallow: /drafts/\n");
    }

    #[test]
    fn site_adds_spa_files() {
        let site = site_fixture();
        let output = SiteOutput::site(site.path(), SPA, &linkcard::Offline).unwrap();
        assert!(output.files.contains_key("/index.html"));
        assert!(output.files.contains_key("/assets/app.js"));
        assert!(output.files.contains_key("/db/manifest.json"));
    }

    #[test]
    fn site_without_spa_is_error() {
        let site = site_fixture();
        let err = SiteOutput::site(site.path(), &[], &linkcard::Offline).err().unwrap();
        assert!(err.to_string().contains("SPA を含まずに"));
    }

    #[test]
    fn write_site_creates_complete_output_with_headers_and_marker() {
        let site = site_fixture();
        let out = crate::testutil::tempdir();
        let out_dir = out.path().join("dist");
        SiteOutput::site(site.path(), SPA, &linkcard::Offline).unwrap().write_site(&out_dir).unwrap();

        let files = files_under(&out_dir);
        for expected in [".sqlite-cms", "_headers", "assets/app.js", "db/manifest.json", "favicon.svg", "index.html", "media/a.png"] {
            assert!(files.iter().any(|f| f == expected), "{expected} がありません: {files:?}");
        }
        assert_eq!(fs::read_to_string(out_dir.join("_headers")).unwrap(), HEADERS);
    }

    #[test]
    fn write_site_replaces_previous_output() {
        let site = site_fixture();
        let out = crate::testutil::tempdir();
        let out_dir = out.path().join("dist");
        let output = SiteOutput::site(site.path(), SPA, &linkcard::Offline).unwrap();
        output.write_site(&out_dir).unwrap();
        fs::write(out_dir.join("stale.js"), b"old").unwrap();

        output.write_site(&out_dir).unwrap();
        assert!(!out_dir.join("stale.js").exists());
    }

    #[test]
    fn write_site_refuses_foreign_non_empty_directory() {
        let site = site_fixture();
        let out = crate::testutil::tempdir();
        fs::write(out.path().join("important.txt"), b"keep").unwrap();

        let err = SiteOutput::site(site.path(), SPA, &linkcard::Offline).unwrap().write_site(out.path()).unwrap_err();
        assert!(err.to_string().contains("sqlite-cms の出力先でもありません"));
        assert_eq!(fs::read(out.path().join("important.txt")).unwrap(), b"keep");
    }

    #[test]
    fn write_data_replaces_only_db_and_media() {
        let site = site_fixture();
        let out = crate::testutil::tempdir();
        fs::create_dir_all(out.path().join("db")).unwrap();
        fs::write(out.path().join("db/articles-0000000000000000.sqlite"), b"old").unwrap();
        fs::write(out.path().join("keep.txt"), b"keep").unwrap();

        SiteOutput::data(site.path(), &linkcard::Offline).unwrap().write_data(out.path()).unwrap();

        let files = files_under(out.path());
        assert!(files.contains(&"keep.txt".to_string()));
        assert!(!files.contains(&"db/articles-0000000000000000.sqlite".to_string()));
        assert!(files.contains(&"db/manifest.json".to_string()));
        assert!(files.contains(&"media/a.png".to_string()));
    }
}
