use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{Connection, MAIN_DB};

use crate::content::{parse_article, parse_category, parse_page, parse_post, Article, Category, Page, Post};
use crate::header;
use crate::linkcard::LinkCard;
use crate::media::MediaSize;
use crate::migrations::{apply_migrations, embedded_migrations};
use crate::site::{read_site_config, SiteConfig};

fn read_docs<T>(dir: &Path, parse: impl Fn(&str, &str) -> Result<T>) -> Result<Vec<T>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    let mut seen: HashMap<String, &Path> = HashMap::new();
    paths
        .iter()
        .map(|path| {
            let slug = slug_of(path);
            if let Some(other) = seen.insert(slug.to_string(), path) {
                bail!("{} と {} が同じ slug（{slug}）になります。どちらかの名前を変えてください", other.display(), path.display());
            }
            let raw = fs::read_to_string(path).with_context(|| format!("{} を読めません", path.display()))?;
            parse(slug, &raw)
        })
        .collect()
}

/// ファイルの slug（URL の名前、ADR 0058）。ファイル名が UUIDv7 で始まれば（`<UUIDv7>-<タイトル>.md`）UUIDv7、
/// そうでなければ（UUIDv7 を使う前に書いた記事や固定ページ）、拡張子を除いたファイル名。
fn slug_of(path: &Path) -> &str {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    crate::uuid::leading_v7(stem).unwrap_or(stem)
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("{} を読めません", path.display())),
    }
}

/// `content/header.md` を読み、site.toml の値を埋め込む（ADR 0043）。なければ None。
fn read_header(content_dir: &Path, config: &SiteConfig) -> Result<Option<String>> {
    let path = content_dir.join("header.md");
    let Some(template) = read_optional(&path)? else { return Ok(None) };
    header::render(&template, config).map(Some).map_err(|e| anyhow!("{} の {e}", path.display()))
}

/// 本文の Markdown をそのまま集める（トップページ、ヘッダ、post、article、page）。リンクカードの URL を探すのに使う。
/// ヘッダは、site.toml の値を埋め込んだものを集める。
pub fn markdown_sources(site_dir: &Path) -> Result<Vec<String>> {
    let content_dir = site_dir.join("content");
    let mut sources: Vec<String> = read_optional(&content_dir.join("index.md"))?.into_iter().collect();
    sources.extend(read_header(&content_dir, &read_site_config(site_dir)?)?);
    for kind in ["posts", "articles", "pages", "categories"] {
        sources.extend(read_docs(&content_dir.join(kind), |_, raw| Ok(raw.to_string()))?);
    }
    Ok(sources)
}

/// post と article を読む（RSS のフィードに使う、ADR 0033）。
pub fn posts_and_articles(site_dir: &Path) -> Result<(Vec<Post>, Vec<Article>)> {
    let content_dir = site_dir.join("content");
    Ok((read_docs(&content_dir.join("posts"), parse_post)?, read_docs(&content_dir.join("articles"), parse_article)?))
}

/// カテゴリの slug に使えない名前（ADR 0060）。カテゴリの URL はサイトの直下（`/<slug>`）なので、
/// SPA のページ（`/about`、`/archive` など、カテゴリの一覧の `/categories`）と、配信するディレクトリ、`serve` の自動反映のパスと重ならないようにする。
/// ファイル（`rss.xml` など）は `.` を含み、slug には `.` を使えないので、重ならない。
pub const RESERVED_CATEGORY_SLUGS: &[&str] =
    &["about", "archive", "search", "posts", "articles", "categories", "assets", "db", "media", "link-cards", "__sqlite-cms"];

/// カテゴリを読み、並び順（order の小さい順、order のないものは後ろに title の順）にそろえる（ADR 0060）。
pub fn categories(site_dir: &Path) -> Result<Vec<Category>> {
    let mut categories = read_docs(&site_dir.join("content").join("categories"), parse_category)?;
    if let Some(category) = categories.iter().find(|c| RESERVED_CATEGORY_SLUGS.contains(&c.slug.as_str())) {
        bail!(
            "カテゴリの slug {:?} は、サイトのほかのページと URL（/{}）が重なるので使えません。content/categories/ のファイル名を変えてください",
            category.slug,
            category.slug
        );
    }
    categories.sort_by(|a, b| {
        (a.order.is_none(), a.order, &a.title, &a.slug).cmp(&(b.order.is_none(), b.order, &b.title, &b.slug))
    });
    Ok(categories)
}

/// page を読む（サイトマップに自己紹介を載せるかに使う、ADR 0037）。
pub fn pages(site_dir: &Path) -> Result<Vec<Page>> {
    read_docs(&site_dir.join("content").join("pages"), parse_page)
}

/// 記事のタグを、書いた順に入れる（ADR 0048）。
fn insert_tags(conn: &Connection, kind: &str, slug: &str, tags: &[String]) -> Result<()> {
    for (position, tag) in tags.iter().enumerate() {
        conn.execute(
            "INSERT INTO tags (kind, slug, position, tag) VALUES (?1, ?2, ?3, ?4)",
            (kind, slug, position as i64, tag),
        )?;
    }
    Ok(())
}

pub fn build_db_bytes(site_dir: &Path, link_cards: &[LinkCard], media_sizes: &[MediaSize]) -> Result<Vec<u8>> {
    let config = read_site_config(site_dir)?;

    let content_dir = site_dir.join("content");
    if !content_dir.is_dir() {
        bail!("コンテンツのディレクトリがありません: {}", content_dir.display());
    }
    let home_md = read_optional(&content_dir.join("index.md"))?;
    let header_md = read_header(&content_dir, &config)?;

    let conn = Connection::open_in_memory()?;
    apply_migrations(&conn, &embedded_migrations()?)?;

    let (license_name, license_url) = match &config.license {
        Some(license) => (Some(&license.name), license.url.as_ref()),
        None => (None, None),
    };
    conn.execute(
        "INSERT INTO site (id, title, author, license_name, license_url, home_md, description, header_md)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (&config.title, &config.author, license_name, license_url, &home_md, config.description(), &header_md),
    )?;

    for p in read_docs(&content_dir.join("posts"), parse_post)? {
        conn.execute(
            "INSERT INTO posts (slug, title, published_at, body_md, sort_key) VALUES (?1, ?2, ?3, ?4, ?1)",
            (&p.slug, &p.title, &p.published_at, &p.body_md),
        )?;
        insert_tags(&conn, "post", &p.slug, &p.tags)?;
    }

    let categories = categories(site_dir)?;
    for (position, c) in categories.iter().enumerate() {
        conn.execute(
            "INSERT INTO categories (slug, title, description, body_md, position) VALUES (?1, ?2, ?3, ?4, ?5)",
            (&c.slug, &c.title, &c.description, &c.body_md, position as i64),
        )?;
    }

    let articles = read_docs(&content_dir.join("articles"), parse_article)?;
    // article の category に合うカテゴリがあること（ADR 0060）。書き誤りをビルドで知らせる。
    if let Some(a) = articles.iter().find(|a| a.category.as_ref().is_some_and(|c| !categories.iter().any(|k| &k.slug == c))) {
        let category = a.category.as_deref().unwrap_or_default();
        bail!("{}: category {category:?} のカテゴリがありません（content/categories/{category}.md を作るか、書き誤りを直してください）", a.order_key);
    }
    // frontmatter の slug（ADR 0059）が、ほかの article の slug と重ならないこと。
    let mut slugs: HashMap<&str, &str> = HashMap::new();
    for a in &articles {
        if let Some(other) = slugs.insert(&a.slug, &a.order_key) {
            bail!("article の {other} と {} が同じ slug（{}）になります。frontmatter の slug を変えてください", a.order_key, a.slug);
        }
    }
    for a in articles {
        conn.execute(
            "INSERT INTO articles (slug, title, published_at, updated_at, description, body_md, sort_key, category)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (&a.slug, &a.title, &a.published_at, &a.updated_at, &a.description, &a.body_md, &a.order_key, &a.category),
        )?;
        insert_tags(&conn, "article", &a.slug, &a.tags)?;
    }

    for p in read_docs(&content_dir.join("pages"), parse_page)? {
        conn.execute(
            "INSERT INTO pages (slug, title, body_md) VALUES (?1, ?2, ?3)",
            (&p.slug, &p.title, &p.body_md),
        )?;
    }

    for card in link_cards {
        conn.execute("INSERT INTO link_cards (url, image_path) VALUES (?1, ?2)", (&card.url, &card.path))?;
    }

    for size in media_sizes {
        conn.execute(
            "INSERT INTO media_sizes (path, width, height) VALUES (?1, ?2, ?3)",
            (&size.path, size.width, size.height),
        )?;
    }

    Ok(conn.serialize(MAIN_DB)?.to_vec())
}

pub fn db_file_name(bytes: &[u8]) -> String {
    format!("articles-{}.sqlite", &blake3::hash(bytes).to_hex()[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    const POST: &str = "---\ntitle: テスト記事\ndate: 2026-09-17\n---\n\n本文の**段落**。\n";
    const ARTICLE: &str = "---\ntitle: 長い読み物\ndate: 2026-09-15\nupdated: 2026-09-16\ndescription: 一覧に出す要約。\n---\n\n本文。\n";
    const PAGE: &str = "---\ntitle: 自己紹介\n---\n\n自己紹介の本文。\n";

    const SITE: &str = "title = \"記事置き場\"\nauthor = \"tett23\"\n\n[license]\nname = \"CC0 1.0\"\n";

    fn site_fixture(content: &[(&str, &str)]) -> crate::testutil::TempDir {
        let mut files = vec![("site.toml".to_string(), SITE.to_string())];
        files.extend(content.iter().map(|(name, raw)| (format!("content/{name}"), raw.to_string())));
        fixture(&files)
    }

    fn fixture(files: &[(String, String)]) -> crate::testutil::TempDir {
        let dir = crate::testutil::tempdir();
        fs::create_dir_all(dir.path().join("content")).unwrap();
        for (name, raw) in files {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, raw).unwrap();
        }
        dir
    }

    fn open(bytes: &[u8]) -> (crate::testutil::TempDir, Connection) {
        let dir = crate::testutil::tempdir();
        let path = dir.path().join("db.sqlite");
        fs::write(&path, bytes).unwrap();
        let conn = Connection::open(&path).unwrap();
        (dir, conn)
    }

    fn column(conn: &Connection, sql: &str) -> Vec<String> {
        conn.prepare(sql)
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn three_content_types_go_into_their_tables() {
        let second = POST.replace("テスト記事", "二本目");
        let site = site_fixture(&[
            ("posts/a.md", POST),
            ("posts/b.md", &second),
            ("posts/note.txt", "md 以外は無視"),
            ("articles/long.md", ARTICLE),
            ("pages/about.md", PAGE),
        ]);
        let bytes = build_db_bytes(site.path(), &[], &[]).unwrap();

        let (_dir, conn) = open(&bytes);
        assert_eq!(column(&conn, "SELECT slug FROM posts ORDER BY slug"), ["a", "b"]);
        assert_eq!(column(&conn, "SELECT description FROM articles"), ["一覧に出す要約。"]);
        assert_eq!(column(&conn, "SELECT slug FROM pages"), ["about"]);
        assert_eq!(
            column(&conn, "SELECT body_md FROM posts WHERE slug = 'a'"),
            ["\n本文の**段落**。\n"]
        );
        assert_eq!(
            column(&conn, "SELECT version FROM schema_migrations ORDER BY version"),
            ["0001", "0002", "0003", "0004", "0005", "0006", "0007", "0008", "0009", "0010", "0011"]
        );
    }

    #[test]
    fn link_cards_go_into_their_table_and_sources_include_every_body() {
        let site = site_fixture(&[("posts/a.md", POST), ("index.md", "https://example.com/home\n")]);
        let cards = [LinkCard { url: "https://example.com/".into(), path: "/link-cards/abc.png".into(), bytes: vec![1] }];
        let sizes = [MediaSize { path: "/media/a.png".into(), width: 640, height: 150 }];
        let (_dir, conn) = open(&build_db_bytes(site.path(), &cards, &sizes).unwrap());
        assert_eq!(column(&conn, "SELECT url || ' ' || image_path FROM link_cards"), ["https://example.com/ /link-cards/abc.png"]);
        assert_eq!(column(&conn, "SELECT path || ' ' || width || 'x' || height FROM media_sizes"), ["/media/a.png 640x150"]);

        let sources = markdown_sources(site.path()).unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources[0].contains("https://example.com/home"));
        assert!(sources[1].contains("本文の**段落**"));
    }

    #[test]
    fn missing_content_directories_are_treated_as_empty() {
        let site = site_fixture(&[("posts/a.md", POST)]);
        let bytes = build_db_bytes(site.path(), &[], &[]).unwrap();
        let (_dir, conn) = open(&bytes);
        assert!(column(&conn, "SELECT slug FROM articles").is_empty());
    }

    #[test]
    fn site_metadata_and_home_go_into_site_table() {
        let site = site_fixture(&[("index.md", "トップの**導入**。\n")]);
        let bytes = build_db_bytes(site.path(), &[], &[]).unwrap();
        let (_dir, conn) = open(&bytes);
        type Row = (String, Option<String>, Option<String>, Option<String>, Option<String>);
        let row: Row = conn
            .query_row(
                "SELECT title, author, license_name, license_url, home_md FROM site WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(
            row,
            (
                "記事置き場".into(),
                Some("tett23".into()),
                Some("CC0 1.0".into()),
                None,
                Some("トップの**導入**。\n".into())
            )
        );
    }

    #[test]
    fn description_is_stored_with_the_default_sentence() {
        let site = site_fixture(&[]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[], &[]).unwrap());
        let description: String = conn.query_row("SELECT description FROM site", [], |r| r.get(0)).unwrap();
        assert_eq!(description, "記事置き場。記事とブログを置いているサイトです。");
    }

    #[test]
    fn home_is_optional() {
        let site = site_fixture(&[]);
        let bytes = build_db_bytes(site.path(), &[], &[]).unwrap();
        let (_dir, conn) = open(&bytes);
        let home: Option<String> = conn.query_row("SELECT home_md FROM site", [], |r| r.get(0)).unwrap();
        assert_eq!(home, None);
    }

    #[test]
    fn header_is_rendered_with_site_values() {
        let header = "[{{title}}](/)\n\n{{#author}}\n{{author}} の記事\n{{/author}}\n{{> search}}\n";
        let site = site_fixture(&[("header.md", header)]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[], &[]).unwrap());
        let header: Option<String> = conn.query_row("SELECT header_md FROM site", [], |r| r.get(0)).unwrap();
        assert_eq!(header.as_deref(), Some("[記事置き場](/)\n\ntett23 の記事\n\n<div class=\"partial-search\"></div>\n\n"));
    }

    #[test]
    fn slug_is_the_leading_uuid_or_the_whole_file_name() {
        // UUIDv7 で始まるファイルは UUIDv7 を、そうでないもの（前からある記事、固定ページ）はファイル名を slug にする（ADR 0058）。
        let id = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";
        let other = "017f22e2-79b0-7cc3-98c4-dc0c0c073990";
        let site = site_fixture(&[
            (&format!("posts/{id}.md"), POST),
            ("posts/2026-09-17.md", POST),
            (&format!("articles/{other}-長い-読み物.md"), POST),
            ("articles/legacy.md", POST),
            ("pages/about.md", "---\ntitle: 自己紹介\n---\n本文\n"),
        ]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[], &[]).unwrap());
        assert_eq!(column(&conn, "SELECT slug FROM posts ORDER BY slug"), ["017f22e2-79b0-7cc3-98c4-dc0c0c07398f", "2026-09-17"]);
        assert_eq!(column(&conn, "SELECT slug FROM articles ORDER BY slug"), [other, "legacy"]);
        assert_eq!(column(&conn, "SELECT slug FROM pages"), ["about"]);
    }

    #[test]
    fn article_url_slug_and_sort_key_go_into_the_table() {
        let id = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";
        let custom = "---\ntitle: t\nslug: long-read\ndate: 2026-09-17\n---\n本文\n";
        let site = site_fixture(&[(&format!("articles/{id}-long-read.md"), custom), (&format!("posts/{id}.md"), POST)]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[], &[]).unwrap());
        assert_eq!(column(&conn, "SELECT slug || ' ' || sort_key FROM articles"), [format!("long-read {id}")]);
        assert_eq!(column(&conn, "SELECT slug || ' ' || sort_key FROM posts"), [format!("{id} {id}")]);
    }

    #[test]
    fn articles_with_the_same_frontmatter_slug_are_an_error() {
        let raw = "---\ntitle: t\nslug: same\ndate: 2026-09-17\n---\n本文\n";
        let site = site_fixture(&[("articles/a.md", raw), ("articles/b.md", raw)]);
        let err = build_db_bytes(site.path(), &[], &[]).unwrap_err().to_string();
        assert!(err.contains("article の a と b が同じ slug（same）になります"), "{err}");
    }

    #[test]
    fn categories_go_into_their_table_in_order_and_articles_point_to_them() {
        let category = |title: &str, order: &str| format!("---\ntitle: {title}\n{order}---\n本文\n");
        let article = "---\ntitle: t\ndate: 2026-09-17\ncategory: howto\n---\n本文\n";
        let site = site_fixture(&[
            ("categories/samples.md", &category("見本", "")),
            ("categories/howto.md", &category("使い方", "order: 2\n")),
            ("categories/typesetting.md", &category("組版", "order: 1\n")),
            ("categories/another.md", &category("あ", "")),
            ("articles/a.md", article),
            ("articles/plain.md", POST),
        ]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[], &[]).unwrap());
        // order の小さい順、order のないものは後ろに title の順。
        assert_eq!(
            column(&conn, "SELECT position || ' ' || slug || ' ' || title FROM categories ORDER BY position"),
            ["0 typesetting 組版", "1 howto 使い方", "2 another あ", "3 samples 見本"]
        );
        assert_eq!(column(&conn, "SELECT slug || ' ' || ifnull(category, '-') FROM articles ORDER BY slug"), ["a howto", "plain -"]);
    }

    #[test]
    fn unknown_category_and_reserved_slugs_are_errors() {
        let article = "---\ntitle: t\ndate: 2026-09-17\ncategory: missing\n---\n本文\n";
        let site = site_fixture(&[("articles/a.md", article)]);
        let err = build_db_bytes(site.path(), &[], &[]).unwrap_err().to_string();
        assert!(err.contains("a: category \"missing\" のカテゴリがありません"), "{err}");

        for reserved in RESERVED_CATEGORY_SLUGS {
            let site = site_fixture(&[(&format!("categories/{reserved}.md"), "---\ntitle: t\n---\n")]);
            let err = build_db_bytes(site.path(), &[], &[]).unwrap_err().to_string();
            assert!(err.contains(&format!("URL（/{reserved}）が重なる")), "{err}");
        }
    }

    #[test]
    fn files_with_the_same_slug_are_an_error() {
        let id = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";
        let site = site_fixture(&[(&format!("posts/{id}.md"), POST), (&format!("posts/{id}-copy.md"), POST)]);
        let err = build_db_bytes(site.path(), &[], &[]).unwrap_err().to_string();
        assert!(err.contains(&format!("同じ slug（{id}）になります")), "{err}");
    }

    #[test]
    fn tags_go_into_the_tags_table_in_order() {
        let post = "---\ntitle: p\ndate: 2026-09-17\ntags: [日記, SQLite]\n---\n本文\n";
        let article = "---\ntitle: a\ndate: 2026-09-17\ntags:\n  - 組版\n  - \"#SQLite\"\n---\n本文\n";
        // page に書いたタグは入れない。
        let page = "---\ntitle: 自己紹介\ntags: [自己紹介]\n---\n本文\n";
        let site = site_fixture(&[("posts/p.md", post), ("posts/plain.md", POST), ("articles/a.md", article), ("pages/about.md", page)]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[], &[]).unwrap());
        assert_eq!(
            column(&conn, "SELECT kind || '/' || slug || ' ' || position || ' ' || tag FROM tags ORDER BY kind, slug, position"),
            ["article/a 0 組版", "article/a 1 SQLite", "post/p 0 日記", "post/p 1 SQLite"]
        );
        assert_eq!(column(&conn, "SELECT slug FROM pages"), ["about"]);
    }

    #[test]
    fn header_is_optional() {
        let site = site_fixture(&[]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[], &[]).unwrap());
        let header: Option<String> = conn.query_row("SELECT header_md FROM site", [], |r| r.get(0)).unwrap();
        assert_eq!(header, None);
    }

    #[test]
    fn header_errors_name_the_file_and_line() {
        let site = site_fixture(&[("header.md", "a\n{{#title}}\n")]);
        let message = build_db_bytes(site.path(), &[], &[]).unwrap_err().to_string();
        assert!(message.ends_with("header.md の 2 行目: セクションが閉じていません: {{#title}}"), "{message}");
    }

    /// SPA の自前の DB の読み手（web/src/sqlite.ts、ADR 0047）が読める形であること。
    /// 読み手は、UTF-8 の、rowid を持つ表だけを読む。列を足す前の行の既定値は、定数だけを読む。
    #[test]
    fn db_stays_readable_by_the_spa_reader() {
        let site = site_fixture(&[("posts/a.md", POST), ("index.md", "トップ\n"), ("header.md", "[{{title}}](/)\n")]);
        let (_dir, conn) = open(&build_db_bytes(site.path(), &[], &[]).unwrap());
        let encoding: String = conn.query_row("PRAGMA encoding", [], |r| r.get(0)).unwrap();
        assert_eq!(encoding, "UTF-8");
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap();
        assert!((512..=65536).contains(&page_size), "{page_size}");

        let tables: Vec<(String, i64, String)> = conn
            .prepare("SELECT name, rootpage, sql FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!tables.is_empty());
        for (name, rootpage, sql) in &tables {
            let upper = sql.to_uppercase();
            assert!(*rootpage > 0 && !upper.contains("VIRTUAL"), "{name} は仮想表: {sql}");
            assert!(!upper.contains("WITHOUT ROWID"), "{name} は WITHOUT ROWID の表: {sql}");
            // 既定値は定数だけ（式の既定値は、読み手が読めない）。
            assert!(!upper.contains("DEFAULT ("), "{name} の既定値が式: {sql}");
        }
    }

    #[test]
    fn missing_site_toml_is_error() {
        let dir = crate::testutil::tempdir();
        fs::create_dir_all(dir.path().join("content")).unwrap();
        let err = build_db_bytes(dir.path(), &[], &[]).unwrap_err();
        assert!(err.to_string().contains("site.toml を読めません"));
    }

    #[test]
    fn missing_content_root_is_error() {
        let dir = crate::testutil::tempdir();
        fs::write(dir.path().join("site.toml"), SITE).unwrap();
        let err = build_db_bytes(dir.path(), &[], &[]).unwrap_err();
        assert!(err.to_string().contains("コンテンツのディレクトリがありません"));
    }

    #[test]
    fn invalid_document_fails_the_build() {
        let site = site_fixture(&[("posts/bad.md", "---\ntitle: t\n---\n本文")]);
        let err = build_db_bytes(site.path(), &[], &[]).unwrap_err();
        assert!(err.to_string().contains("date"));
    }

    #[test]
    fn same_input_yields_same_file_name() {
        let site = site_fixture(&[("posts/a.md", POST), ("articles/long.md", ARTICLE)]);
        let first = db_file_name(&build_db_bytes(site.path(), &[], &[]).unwrap());
        let second = db_file_name(&build_db_bytes(site.path(), &[], &[]).unwrap());
        assert_eq!(first, second);
        assert!(first.starts_with("articles-") && first.ends_with(".sqlite"));
        assert_eq!(first.len(), "articles-".len() + 16 + ".sqlite".len());
    }
}
