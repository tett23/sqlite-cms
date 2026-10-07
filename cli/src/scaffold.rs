use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Post,
    Article,
    Page,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "post" => Some(Self::Post),
            "article" => Some(Self::Article),
            "page" => Some(Self::Page),
            _ => None,
        }
    }

    fn dir(self) -> &'static str {
        match self {
            Self::Post => "posts",
            Self::Article => "articles",
            Self::Page => "pages",
        }
    }
}

pub fn validate_slug(slug: &str) -> Result<()> {
    if !crate::content::is_valid_slug(slug) {
        bail!("slug {slug:?} には文字、数字、-、_ だけを使ってください（URL とファイル名になります）");
    }
    Ok(())
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn template(kind: Kind, title: &str, date: &str) -> String {
    let title = quote(title);
    match kind {
        Kind::Post => format!("---\ntitle: {title}\ndate: {date}\ntags: []\n---\n\n"),
        Kind::Article => format!("---\ntitle: {title}\ndate: {date}\ndescription:\ntags: []\n---\n\n"),
        Kind::Page => format!("---\ntitle: {title}\n---\n\n"),
    }
}

/// 雛形のファイルを作る。作れたら true、同じ名前のファイルがすでにあれば false。
fn write_new(path: &Path, body: &str) -> Result<bool> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(body.as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e).with_context(|| format!("{} を作れません", path.display())),
    }
}

/// ファイル名に入れるタイトルの長さの上限（文字数）。長いタイトルでも、ファイル名が長くなりすぎないようにする。
const FILE_NAME_TITLE_CHARS: usize = 50;

/// タイトルを、ファイル名に入れる形にする（ADR 0058）。文字、数字、`_` 以外を `-` にし、続く `-` は一つにまとめる。
pub fn file_name_part(title: &str) -> String {
    let joined = title
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    joined.chars().take(FILE_NAME_TITLE_CHARS).collect::<String>().trim_end_matches('-').to_string()
}

/// 記事の雛形を作る（ADR 0058）。
/// post と article のファイル名は `<UUIDv7>.md` か `<UUIDv7>-<名前>.md` にする。URL の slug は UUIDv7 になる。
/// 名前は、slug を指定すればそれ、なければ article ならタイトル（`file_name_part`）、post なら付けない。
/// post のタイトルを省くと日付をタイトルにする。article はタイトルを省けない。
/// page は今までどおり `<slug>.md` で、slug を省けない（タイトルを省くと slug をタイトルにする）。
pub fn create(site_dir: &Path, kind: Kind, slug: Option<&str>, title: Option<&str>, date: &str) -> Result<PathBuf> {
    if let Some(slug) = slug {
        validate_slug(slug)?;
    }
    if title.is_some_and(|title| title.trim().is_empty()) {
        bail!("タイトルが空です");
    }
    if slug.is_none() && kind == Kind::Page {
        bail!("page は日付を持たないので、slug を省略できません");
    }
    if title.is_none() && kind == Kind::Article {
        bail!("article にはタイトルが要ります（--title で指定してください）");
    }
    if !site_dir.join("site.toml").is_file() {
        bail!(
            "{} に site.toml がありません。記事リポジトリの中で実行するか、SITE_DIR を指定してください",
            site_dir.display()
        );
    }

    let dir = site_dir.join("content").join(kind.dir());
    fs::create_dir_all(&dir).with_context(|| format!("{} を作れません", dir.display()))?;

    let (file_name, title) = match kind {
        Kind::Page => {
            let slug = slug.expect("確かめ済み");
            (slug.to_string(), title.unwrap_or(slug).to_string())
        }
        Kind::Post | Kind::Article => {
            let id = crate::uuid::new_v7();
            let name = match (slug, kind) {
                (Some(slug), _) => slug.to_string(),
                (None, Kind::Article) => file_name_part(title.expect("確かめ済み")),
                (None, _) => String::new(),
            };
            let file_name = if name.is_empty() { id } else { format!("{id}-{name}") };
            (file_name, title.unwrap_or(date).to_string())
        }
    };
    let mut body = template(kind, &title, date);
    // article で slug を指定したら、frontmatter に書き、URL にする（ADR 0059）。ファイル名の後ろの名前だけでは、タイトルから作ったものと区別できない。
    if let (Kind::Article, Some(slug)) = (kind, slug) {
        body = body.replacen("\ndate: ", &format!("\nslug: {slug}\ndate: "), 1);
    }
    let path = dir.join(format!("{file_name}.md"));
    if !write_new(&path, &body)? {
        bail!("{} はすでにあります。別の slug を指定してください", path.display());
    }
    Ok(path)
}

const SITE_TOML_REST: &str = r#"
# author = "名前"             # フッターに表示する
# description = "サイトの説明" # ページの meta description。省略するとサイト名から組み立てる
# timezone = "Asia/Tokyo"     # new が入れる日付のタイムゾーン（"+09:00" の形も可）。省略すると環境のタイムゾーン
# base_path = "/my-blog/"     # サイトを置くパス。GitHub Pages のプロジェクトのページ（https://<user>.github.io/<repo>/）なら "/<repo>/"

# [license]                   # フッターに表示する
# name = "CC0 1.0"
# url = "https://creativecommons.org/publicdomain/zero/1.0/"

# [deploy]                    # sqlite-cms deploy の公開先。次のどれか一つ
# worker = "my-blog"          # Cloudflare Workers（target は省略できる）
#
# target = "github-pages"     # GitHub Pages（記事リポジトリの gh-pages ブランチに push する）
# branch = "gh-pages"         # 省略すると gh-pages
# remote = "origin"           # 省略すると origin
# cname = "blog.example.com"  # 独自ドメイン（任意）
#
# target = "rsync"            # rsync で任意のサーバーに送る
# destination = "user@example.com:/var/www/blog/"
"#;

const INDEX_MD: &str = "<!-- トップページの本文をここに Markdown で書く。このコメントは表示されない -->\n";

/// ヘッダ（ADR 0043、0056）。既定のヘッダと同じ見た目になるもの。見本（example/content/header.md）と同じにする。
pub const HEADER_MD: &str = "\
{{! ヘッダ（任意）。書かなければ既定のヘッダになる。site.toml の値を二重の波括弧で埋め込める }}
{{! 最初の段落はサイト名として大きく表示する。search のパーシャルの場所に検索ボックスを置く }}
[{{title}}](/)

- [トップ](/)
- [一覧](/archive)
- [自己紹介](/about)
- [検索](/search)

{{> search}}
";

fn site_toml(title: &str) -> String {
    format!(
        "# サイトの設定。書ける項目は sqlite-cms --help を参照\ntitle = {}\n{SITE_TOML_REST}",
        toml::Value::String(title.to_string())
    )
}

fn default_title(site_dir: &Path) -> String {
    site_dir
        .canonicalize()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "新しいサイト".to_string())
}

pub struct InitReport {
    pub written: Vec<PathBuf>,
    pub overwritten: Vec<PathBuf>,
    /// 行を足した既存の `.gitignore`。
    pub appended: Vec<PathBuf>,
}

const GITIGNORE: &str = ".gitignore";

/// `.gitignore` に書く行と、その前に置く注釈。
const IGNORED: &[(&str, &str)] = &[
    (".env", "# deploy の認証情報（sqlite-cms deploy が読む）"),
    (".sqlite-cms-cache/", "# リンクカードの画像など、ビルドのときに取得したもの（sqlite-cms が作る）"),
];

/// `.env` と取得したもののキャッシュが Git に入らないよう、`.gitignore` に書く。
/// なければ作り、あって行がなければ末尾に足す。すでにあれば何もしない。
fn write_gitignore(site_dir: &Path, report: &mut InitReport) -> Result<()> {
    let path = site_dir.join(GITIGNORE);
    let existing = match fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).with_context(|| format!("{} を読めません", path.display())),
    };
    let text = existing.as_deref().unwrap_or("");
    let present = |entry: &str| {
        let bare = entry.trim_end_matches('/');
        text.lines().map(str::trim).any(|line| {
            let line = line.trim_start_matches('/');
            line == bare || line == format!("{bare}/")
        })
    };
    let missing: Vec<String> =
        IGNORED.iter().filter(|(entry, _)| !present(entry)).map(|(entry, comment)| format!("{comment}\n{entry}\n")).collect();
    if missing.is_empty() {
        return Ok(());
    }
    let separator = if text.is_empty() || text.ends_with('\n') { "" } else { "\n" };
    fs::write(&path, format!("{text}{separator}{}", missing.join("")))
        .with_context(|| format!("{} に書き込めません", path.display()))?;
    if existing.is_some() {
        report.appended.push(path);
    } else {
        report.written.push(path);
    }
    Ok(())
}

/// ビルドに必要な site.toml と content/、公開の認証情報の見本 .env.example を作り、.gitignore に .env を書く。
/// すでにあるときは `force` がなければエラーにし、`force` なら init が作るファイルだけを作り直す（記事や画像、.env は消さない）。
pub fn init(site_dir: &Path, title: Option<&str>, force: bool) -> Result<InitReport> {
    if title.is_some_and(|t| t.trim().is_empty()) {
        bail!("タイトルが空です");
    }
    if site_dir.exists() && !site_dir.is_dir() {
        bail!("{} はディレクトリではありません", site_dir.display());
    }
    let existing: Vec<&str> = ["site.toml", "content/"]
        .into_iter()
        .filter(|name| site_dir.join(name.trim_end_matches('/')).exists())
        .collect();
    if !existing.is_empty() && !force {
        bail!(
            "{} にはすでに {} があります。作り直すときは --force を付けてください（content/ の記事や画像は消しません）",
            site_dir.display(),
            existing.join("、")
        );
    }

    fs::create_dir_all(site_dir).with_context(|| format!("{} を作れません", site_dir.display()))?;
    let title = title.map_or_else(|| default_title(site_dir), str::to_string);
    let files = [
        ("site.toml", site_toml(&title)),
        ("content/index.md", INDEX_MD.to_string()),
        ("content/header.md", HEADER_MD.to_string()),
        ("content/favicon.svg", crate::favicon::placeholder(&title)),
        ("content/robots.txt", crate::robots::DEFAULT.to_string()),
        ("content/pages/about.md", template(Kind::Page, "自己紹介", "")),
        ("content/posts/.gitkeep", String::new()),
        ("content/articles/.gitkeep", String::new()),
        ("content/media/.gitkeep", String::new()),
        (crate::dotenv::EXAMPLE_FILE_NAME, crate::dotenv::EXAMPLE.to_string()),
    ];

    let mut report = InitReport { written: Vec::new(), overwritten: Vec::new(), appended: Vec::new() };
    for (relative, body) in files {
        let path = site_dir.join(relative);
        if path.is_dir() {
            bail!("{} がディレクトリになっているので作れません", path.display());
        }
        if path.exists() {
            report.overwritten.push(path.clone());
        }
        fs::create_dir_all(path.parent().unwrap()).with_context(|| format!("{} を作れません", path.display()))?;
        fs::write(&path, body).with_context(|| format!("{} に書き込めません", path.display()))?;
        report.written.push(path);
    }
    write_gitignore(site_dir, &mut report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{parse_article, parse_page, parse_post};

    use crate::site::parse_site_config;

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
    fn init_creates_a_site_that_builds() {
        let tmp = crate::testutil::tempdir();
        let site = tmp.path().join("my-blog");
        let report = init(&site, None, false).unwrap();

        assert_eq!(
            files_under(&site),
            [
                ".env.example",
                ".gitignore",
                "content/articles/.gitkeep",
                "content/favicon.svg",
                "content/header.md",
                "content/index.md",
                "content/media/.gitkeep",
                "content/pages/about.md",
                "content/posts/.gitkeep",
                "content/robots.txt",
                "site.toml",
            ]
        );
        assert_eq!(report.written.len(), 11);
        assert!(report.overwritten.is_empty());
        assert!(report.appended.is_empty());
        assert_eq!(fs::read_to_string(site.join(".env.example")).unwrap(), crate::dotenv::EXAMPLE);
        assert!(fs::read_to_string(site.join(".gitignore")).unwrap().lines().any(|line| line == ".env"));

        let config = parse_site_config(&fs::read_to_string(site.join("site.toml")).unwrap()).unwrap();
        assert_eq!(config.title, "my-blog");
        assert_eq!(fs::read_to_string(site.join("content/favicon.svg")).unwrap(), crate::favicon::placeholder("my-blog"));
        assert_eq!(config.deploy, None);

        let output = crate::output::SiteOutput::data(&site, &crate::linkcard::Offline).unwrap();
        assert!(output.files.contains_key("/db/manifest.json"));
        // 作ったヘッダは、サイト名を埋め込んで DB に入る（ADR 0056）。
        let db = output.files.iter().find(|(path, _)| path.ends_with(".sqlite")).unwrap().1;
        let dir = crate::testutil::tempdir();
        fs::write(dir.path().join("db.sqlite"), db).unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("db.sqlite")).unwrap();
        let header: String = conn.query_row("SELECT header_md FROM site", [], |row| row.get(0)).unwrap();
        assert!(header.starts_with("[my-blog](/)\n"), "{header}");
        assert!(header.contains("<div class=\"partial-search\"></div>"), "{header}");
        assert!(!output.files.keys().any(|path| path.contains(".gitkeep")));
    }

    #[test]
    fn header_template_is_the_same_as_the_example() {
        // 見本のヘッダと init が作るヘッダをそろえておく（ADR 0056）。
        let example = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../example/content/header.md")).unwrap();
        assert_eq!(HEADER_MD, example);
    }

    #[test]
    fn init_quotes_any_title() {
        for title in ["tett23の記事置き場", "\"引用\" と \\ と # と [括弧]", "改行\nあり"] {
            let tmp = crate::testutil::tempdir();
            init(tmp.path(), Some(title), false).unwrap();
            let config = parse_site_config(&fs::read_to_string(tmp.path().join("site.toml")).unwrap()).unwrap();
            assert_eq!(config.title, title);
        }
    }

    #[test]
    fn init_refuses_existing_site_without_force() {
        let tmp = crate::testutil::tempdir();
        fs::write(tmp.path().join("site.toml"), "title = \"keep\"\n").unwrap();
        let err = init(tmp.path(), None, false).err().unwrap().to_string();
        assert!(err.contains("すでに site.toml があります"), "{err}");
        assert!(err.contains("--force"));
        assert_eq!(fs::read_to_string(tmp.path().join("site.toml")).unwrap(), "title = \"keep\"\n");
        assert!(!tmp.path().join("content").exists());
    }

    #[test]
    fn init_refuses_existing_content_without_force() {
        let tmp = crate::testutil::tempdir();
        fs::create_dir_all(tmp.path().join("content/posts")).unwrap();
        let err = init(tmp.path(), None, false).err().unwrap().to_string();
        assert!(err.contains("content/"), "{err}");
        assert!(!tmp.path().join("site.toml").exists());
    }

    #[test]
    fn init_with_force_recreates_init_files_but_keeps_articles() {
        let tmp = crate::testutil::tempdir();
        init(tmp.path(), Some("古い"), false).unwrap();
        fs::write(tmp.path().join("content/pages/about.md"), "---\ntitle: 書いた自己紹介\n---\n").unwrap();
        fs::write(tmp.path().join("content/posts/hello.md"), "---\ntitle: 記事\ndate: 2026-09-26\n---\n").unwrap();
        fs::write(tmp.path().join("content/media/photo.png"), b"png").unwrap();

        let report = init(tmp.path(), Some("新しい"), true).unwrap();

        let config = parse_site_config(&fs::read_to_string(tmp.path().join("site.toml")).unwrap()).unwrap();
        assert_eq!(config.title, "新しい");
        assert!(fs::read_to_string(tmp.path().join("content/pages/about.md")).unwrap().contains("\"自己紹介\""));
        assert!(tmp.path().join("content/posts/hello.md").is_file());
        assert!(tmp.path().join("content/media/photo.png").is_file());
        // .gitignore にはすでに .env があるので触らない。
        assert_eq!(report.overwritten.len(), 10);
        assert!(!report.written.iter().any(|path| path.ends_with(".gitignore")));
        assert!(report.appended.is_empty());
    }

    #[test]
    fn init_never_touches_dotenv_and_keeps_existing_gitignore() {
        let tmp = crate::testutil::tempdir();
        fs::write(tmp.path().join(".env"), "CLOUDFLARE_API_TOKEN=secret\n").unwrap();
        fs::write(tmp.path().join(".gitignore"), "dist/").unwrap();

        let report = init(tmp.path(), Some("サイト"), false).unwrap();

        assert_eq!(fs::read_to_string(tmp.path().join(".env")).unwrap(), "CLOUDFLARE_API_TOKEN=secret\n");
        let gitignore = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert!(gitignore.starts_with("dist/\n"), "{gitignore}");
        assert!(gitignore.lines().any(|line| line == ".env"), "{gitignore}");
        assert!(gitignore.lines().any(|line| line == ".sqlite-cms-cache/"), "{gitignore}");
        assert_eq!(report.appended, [tmp.path().join(".gitignore")]);

        // 二度目は足さない。
        init(tmp.path(), Some("サイト"), true).unwrap();
        assert_eq!(fs::read_to_string(tmp.path().join(".gitignore")).unwrap(), gitignore);
        assert_eq!(fs::read_to_string(tmp.path().join(".env")).unwrap(), "CLOUDFLARE_API_TOKEN=secret\n");
    }

    #[test]
    fn init_rejects_empty_title_and_non_directory() {
        let tmp = crate::testutil::tempdir();
        assert!(init(tmp.path(), Some(" "), false).err().unwrap().to_string().contains("タイトル"));
        let file = tmp.path().join("file");
        fs::write(&file, b"x").unwrap();
        assert!(init(&file, None, false).err().unwrap().to_string().contains("ディレクトリではありません"));
    }

    const TRICKY_TITLES: &[&str] = &[
        "ふつうのタイトル",
        "記事: その2",
        "\"引用\" と \\ と # と [括弧]",
        "[先頭が記号",
        "- ハイフン始まり",
        "null",
        "改行\nを含む",
    ];

    #[test]
    fn templates_round_trip_through_the_builder() {
        for title in TRICKY_TITLES {
            let post = parse_post("s", &template(Kind::Post, title, "2026-09-26")).unwrap();
            assert_eq!(post.title, *title);
            assert_eq!(post.published_at, "2026-09-26");

            let article = parse_article("s", &template(Kind::Article, title, "2026-09-26")).unwrap();
            assert_eq!(article.title, *title);
            assert_eq!(article.description, None);

            let page = parse_page("s", &template(Kind::Page, title, "2026-09-26")).unwrap();
            assert_eq!(page.title, *title);
        }
    }

    #[test]
    fn templates_have_empty_bodies() {
        assert_eq!(template(Kind::Post, "t", "2026-09-26"), "---\ntitle: \"t\"\ndate: 2026-09-26\ntags: []\n---\n\n");
        assert!(template(Kind::Article, "t", "2026-09-26").contains("\ndescription:\n"));
        // タグを書く場所がわかるよう、空のリストを入れておく（ADR 0048）。page には入れない。
        assert!(template(Kind::Post, "t", "2026-09-26").contains("\ntags: []\n"));
        assert!(template(Kind::Article, "t", "2026-09-26").contains("\ndescription:\ntags: []\n"));
        assert!(!template(Kind::Page, "t", "2026-09-26").contains("tags"));
        assert!(!template(Kind::Page, "t", "2026-09-26").contains("date:"));
    }

    #[test]
    fn validates_slugs() {
        for ok in ["hello", "hello-world", "2026_09", "日本語の記事"] {
            assert!(validate_slug(ok).is_ok(), "{ok}");
        }
        for ng in ["", "a/b", "../x", ".hidden", "a b", "a?b", "a#b", "a%20b", "a.md"] {
            assert!(validate_slug(ng).is_err(), "{ng}");
        }
    }

    fn site() -> crate::testutil::TempDir {
        let dir = crate::testutil::tempdir();
        fs::write(dir.path().join("site.toml"), "title = \"t\"\n").unwrap();
        dir
    }

    /// 作ったファイルの名前（拡張子を除く）と、その UUIDv7。
    fn stem_and_id(path: &Path) -> (String, String) {
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        let id = crate::uuid::leading_v7(&stem).unwrap_or_else(|| panic!("UUIDv7 で始まらない: {stem}")).to_string();
        (stem, id)
    }

    #[test]
    fn article_file_name_is_uuid_and_title() {
        let site = site();
        let path = create(site.path(), Kind::Article, None, Some("SQLite と CMS / 2026"), "2026-09-26").unwrap();
        assert_eq!(path.parent().unwrap(), site.path().join("content/articles"));
        let (stem, id) = stem_and_id(&path);
        assert_eq!(stem, format!("{id}-SQLite-と-CMS-2026"));
        let article = parse_article(&id, &fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(article.title, "SQLite と CMS / 2026");
        assert_eq!(article.published_at, "2026-09-26");
    }

    #[test]
    fn article_requires_a_title() {
        let site = site();
        for slug in [None, Some("long-read")] {
            let err = create(site.path(), Kind::Article, slug, None, "2026-09-26").unwrap_err();
            assert!(err.to_string().contains("article にはタイトルが要ります"), "{err}");
        }
        assert!(!site.path().join("content/articles").exists());
    }

    #[test]
    fn slug_follows_the_uuid() {
        let site = site();
        let article = create(site.path(), Kind::Article, Some("long-read"), Some("長い読み物"), "2026-09-26").unwrap();
        let (stem, id) = stem_and_id(&article);
        assert_eq!(stem, format!("{id}-long-read"));
        // article は、指定した slug を frontmatter に書き、URL にする（ADR 0059）。並べる順の鍵は UUIDv7。
        let written = fs::read_to_string(&article).unwrap();
        assert!(written.starts_with("---\ntitle: \"長い読み物\"\nslug: long-read\ndate: 2026-09-26\n"), "{written}");
        let parsed = parse_article(&id, &written).unwrap();
        assert_eq!((parsed.slug.as_str(), parsed.order_key.as_str()), ("long-read", id.as_str()));
        // 指定しなければ、frontmatter に slug を書かず、URL は UUIDv7。
        let untitled = create(site.path(), Kind::Article, None, Some("題"), "2026-09-26").unwrap();
        let (_, untitled_id) = stem_and_id(&untitled);
        let written = fs::read_to_string(&untitled).unwrap();
        assert!(!written.contains("\nslug:"), "{written}");
        assert_eq!(parse_article(&untitled_id, &written).unwrap().slug, untitled_id);
        let post = create(site.path(), Kind::Post, Some("hello"), None, "2026-09-26").unwrap();
        let (stem, id) = stem_and_id(&post);
        assert_eq!(stem, format!("{id}-hello"));
        // post の URL は、slug を指定しても UUIDv7（frontmatter に slug を書かない）。
        assert!(!fs::read_to_string(&post).unwrap().contains("\nslug:"));
    }

    #[test]
    fn post_without_slug_is_named_by_uuid_and_titled_by_date() {
        let site = site();
        let first = create(site.path(), Kind::Post, None, None, "2026-09-26").unwrap();
        let second = create(site.path(), Kind::Post, None, Some("二本目"), "2026-09-26").unwrap();
        let (first_stem, first_id) = stem_and_id(&first);
        let (second_stem, second_id) = stem_and_id(&second);
        // 名前は UUIDv7 だけ。同じ日付でも別の名前になり、作った順に並ぶ。
        assert_eq!((first_stem, second_stem.clone()), (first_id.clone(), second_id.clone()));
        assert!(first_id < second_id || first_id[..13] == second_id[..13], "{first_id} {second_id}");
        let post = parse_post(&first_id, &fs::read_to_string(&first).unwrap()).unwrap();
        assert_eq!(post.title, "2026-09-26");
        assert_eq!(post.published_at, "2026-09-26");
        assert_eq!(parse_post(&second_id, &fs::read_to_string(&second).unwrap()).unwrap().title, "二本目");
    }

    #[test]
    fn file_name_part_replaces_unsafe_characters() {
        assert_eq!(file_name_part("SQLite と CMS"), "SQLite-と-CMS");
        assert_eq!(file_name_part("  a / b\\c: d?*  "), "a-b-c-d");
        assert_eq!(file_name_part("a--b__c"), "a-b__c");
        assert_eq!(file_name_part("記事（その２）"), "記事-その２");
        assert_eq!(file_name_part("!!!"), "");
        assert_eq!(file_name_part(&"長".repeat(80)).chars().count(), 50);
        assert_eq!(file_name_part(&format!("{}-b", "a".repeat(49))), "a".repeat(49));
    }

    #[test]
    fn title_without_usable_characters_leaves_only_the_uuid() {
        let site = site();
        let path = create(site.path(), Kind::Article, None, Some("!!!"), "2026-09-26").unwrap();
        let (stem, id) = stem_and_id(&path);
        assert_eq!(stem, id);
    }

    #[test]
    fn page_requires_a_slug() {
        let site = site();
        let err = create(site.path(), Kind::Page, None, None, "").unwrap_err();
        assert!(err.to_string().contains("slug を省略できません"));
    }

    #[test]
    fn refuses_to_overwrite_existing_page() {
        let site = site();
        create(site.path(), Kind::Page, Some("about"), Some("一本目"), "").unwrap();
        let err = create(site.path(), Kind::Page, Some("about"), Some("二本目"), "").unwrap_err();
        assert!(err.to_string().contains("すでにあります"));
        let written = fs::read_to_string(site.path().join("content/pages/about.md")).unwrap();
        assert!(written.contains("一本目"));
    }

    #[test]
    fn same_slug_in_different_kinds_is_allowed() {
        let site = site();
        create(site.path(), Kind::Post, Some("about"), Some("t"), "2026-09-26").unwrap();
        create(site.path(), Kind::Page, Some("about"), Some("t"), "2026-09-26").unwrap();
    }

    #[test]
    fn requires_site_toml() {
        let dir = crate::testutil::tempdir();
        let err = create(dir.path(), Kind::Post, Some("hello"), Some("t"), "2026-09-26").unwrap_err();
        assert!(err.to_string().contains("site.toml がありません"));
        assert!(!dir.path().join("content").exists());
    }

    #[test]
    fn rejects_empty_title_and_bad_slug() {
        let site = site();
        assert!(create(site.path(), Kind::Post, Some("hello"), Some("  "), "2026-09-26").unwrap_err().to_string().contains("タイトル"));
        assert!(create(site.path(), Kind::Post, Some("../escape"), Some("t"), "2026-09-26").is_err());
        assert!(!site.path().join("content/escape.md").exists());
    }
}
