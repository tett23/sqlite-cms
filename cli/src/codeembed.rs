//! コードの埋め込み（ADR 0065）。
//!
//! 本文の URL だけの行のうち、GitHub のファイルと Gist の URL について、その中身を取得する。
//! 中身は DB の `code_embed_files` に入れ、SPA がリンクカードの代わりにコードブロックとして描く。
//! 閲覧のときに GitHub と通信しないよう、取得はサイトを組み立てるとき（リンクカードの画像と同じとき）に行う。
//! 取得した結果は記事リポジトリの `.sqlite-cms-cache/code-embeds/` に保存し、次のビルドからはそれを使う。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::linkcard::{self, Fetcher};

const EMBED_CACHE: &str = "code-embeds";

/// 取得の User-Agent に書く用途。
const PURPOSE: &str = "code embed";

/// 一つの埋め込みの中身の合計の上限（行の範囲があれば、切り出した後の大きさで比べる）。
pub const MAX_EMBED_BYTES: usize = 100 * 1024;
/// 取得する元のファイルと、Gist の API の応答の上限。行の範囲で切り出す前の大きさなので、埋め込みの上限より大きくする。
const MAX_SOURCE_BYTES: u64 = 10 * 1024 * 1024;

const GITHUB_PREFIX: &str = "https://github.com/";
const GIST_PREFIX: &str = "https://gist.github.com/";
const GIST_RAW_PREFIX: &str = "https://gist.githubusercontent.com/";

/// 埋め込みの対象の URL の種類。
#[derive(Debug, PartialEq)]
pub enum Target {
    /// GitHub のファイル（`/blob/`）。
    File {
        /// 中身を取得する URL（raw.githubusercontent.com）。
        raw_url: String,
        file_name: String,
        /// 行の範囲（`#L10` は (10, None)、`#L10-L20` は (10, Some(20))）。
        lines: Option<(usize, Option<usize>)>,
    },
    /// Gist。
    Gist {
        /// フラグメントを除いた Gist の URL。
        page_url: String,
        id: String,
        /// `#file-<名前>` の名前。
        file: Option<String>,
    },
}

/// 表示する一つのファイル。
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct EmbedFile {
    pub file_name: String,
    /// 見出しのリンク先。
    pub source_url: String,
    /// 最初の行の番号。
    pub start_line: usize,
    pub content: String,
}

/// 本文の URL と、表示するファイル。
#[derive(Debug, PartialEq)]
pub struct CodeEmbed {
    pub url: String,
    pub files: Vec<EmbedFile>,
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `%XX` を戻す。戻した結果が UTF-8 でなければ、元のまま返す。
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(byte) = hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// `L10` と `L10-L20` を読む。
fn line_range(fragment: &str) -> Option<(usize, Option<usize>)> {
    let number = |s: &str| s.strip_prefix('L').filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))?.parse::<usize>().ok();
    match fragment.split_once('-') {
        None => Some((number(fragment).filter(|&n| n >= 1)?, None)),
        Some((start, end)) => {
            let (start, end) = (number(start)?, number(end)?);
            (start >= 1 && end >= start).then_some((start, Some(end)))
        }
    }
}

/// URL が埋め込みの対象なら、その種類を返す。対象でなければ None（今までどおりリンクカードにする）。
pub fn target(url: &str) -> Option<Target> {
    if url.contains('?') {
        return None;
    }
    let (without_fragment, fragment) = match url.split_once('#') {
        Some((head, fragment)) => (head, Some(fragment)),
        None => (url, None),
    };
    if let Some(path) = without_fragment.strip_prefix(GITHUB_PREFIX) {
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() < 5 || segments[2] != "blob" || segments.iter().any(|s| s.is_empty()) {
            return None;
        }
        let lines = match fragment {
            None => None,
            Some(fragment) => Some(line_range(fragment)?),
        };
        return Some(Target::File {
            raw_url: format!("https://raw.githubusercontent.com/{}/{}/{}", segments[0], segments[1], segments[3..].join("/")),
            file_name: percent_decode(segments[segments.len() - 1]),
            lines,
        });
    }
    if let Some(path) = without_fragment.strip_prefix(GIST_PREFIX) {
        let segments: Vec<&str> = path.split('/').collect();
        // ユーザー名だけの URL（ユーザーのページ）と区別するため、ユーザー名のない形では、今の Gist の ID の長さ（20 桁以上）を求める。
        let id = match segments.as_slice() {
            [id] if is_hex(id) && id.len() >= 20 => *id,
            [user, id] if !user.is_empty() && is_hex(id) => *id,
            _ => return None,
        };
        let file = match fragment {
            None => None,
            Some(fragment) => Some(fragment.strip_prefix("file-").filter(|name| !name.is_empty())?.to_string()),
        };
        return Some(Target::Gist { page_url: without_fragment.to_string(), id: id.to_string(), file });
    }
    None
}

/// Gist のファイル名を、`#file-` の後の名前と比べる形にする（小文字にし、英数字以外を `-` にする）。
pub fn gist_anchor(file_name: &str) -> String {
    file_name.chars().map(|c| if c.is_alphanumeric() { c.to_lowercase().next().unwrap_or(c) } else { '-' }).collect()
}

/// 取得した中身を文字列にする。UTF-8 でないもの、NUL を含むもの（バイナリ）は使わない。改行は LF にそろえる。
fn decode_text(bytes: Vec<u8>, what: &str) -> Result<String> {
    let text = String::from_utf8(bytes).map_err(|_| anyhow!("{what} は UTF-8 のテキストではありません"))?;
    if text.contains('\0') {
        bail!("{what} はバイナリのファイルです");
    }
    Ok(text.replace("\r\n", "\n"))
}

/// 中身から行の範囲を切り出す。範囲がファイルの終わりを超えるところは切り詰め、まったくファイルにないときは失敗にする。
/// 返す中身は、行を `\n` でつないだもの（最後の改行は含まない）。
pub fn slice_lines(text: &str, lines: Option<(usize, Option<usize>)>) -> Result<(usize, String)> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    let Some((start, end)) = lines else { return Ok((1, body.to_string())) };
    let all: Vec<&str> = if body.is_empty() { Vec::new() } else { body.split('\n').collect() };
    if start > all.len() {
        bail!("行 {start} はファイルにありません（{} 行）", all.len());
    }
    let end = end.unwrap_or(start).min(all.len());
    Ok((start, all[start - 1..end].join("\n")))
}

fn fetch_text(fetcher: &dyn Fetcher, url: &str) -> Result<String> {
    let fetched = fetcher.fetch(url, MAX_SOURCE_BYTES, PURPOSE)?;
    if fetched.body.len() as u64 > MAX_SOURCE_BYTES {
        bail!("{url} が {MAX_SOURCE_BYTES} バイトを超えています");
    }
    decode_text(fetched.body, url)
}

#[derive(Deserialize)]
struct GistFile {
    filename: String,
    content: Option<String>,
    #[serde(default)]
    truncated: bool,
    raw_url: Option<String>,
}

#[derive(Deserialize)]
struct Gist {
    // serde_json の Map は名前の順に並ぶ。Gist のページも、ファイルを名前の順に並べる。
    files: serde_json::Map<String, serde_json::Value>,
}

/// Gist の API の応答から、表示するファイルを作る。切り詰められたファイルは raw_url から取得し直す。
pub fn gist_files(fetcher: &dyn Fetcher, page_url: &str, json: &[u8], only: Option<&str>) -> Result<Vec<EmbedFile>> {
    let gist: Gist = serde_json::from_slice(json).context("Gist の API の応答を読めません")?;
    let mut files = Vec::new();
    for value in gist.files.into_values() {
        let file: GistFile = serde_json::from_value(value).context("Gist のファイルを読めません")?;
        let anchor = gist_anchor(&file.filename);
        if only.is_some_and(|only| gist_anchor(only) != anchor) {
            continue;
        }
        let content = match (file.truncated, file.content) {
            (false, Some(content)) => decode_text(content.into_bytes(), &file.filename)?,
            _ => {
                let raw_url = file.raw_url.filter(|url| url.starts_with(GIST_RAW_PREFIX));
                let raw_url = raw_url.ok_or_else(|| anyhow!("{} の raw_url が {GIST_RAW_PREFIX} で始まりません", file.filename))?;
                fetch_text(fetcher, &raw_url)?
            }
        };
        let (start_line, content) = slice_lines(&content, None)?;
        files.push(EmbedFile { source_url: format!("{page_url}#file-{anchor}"), file_name: file.filename, start_line, content });
    }
    if files.is_empty() {
        bail!("表示するファイルがありません");
    }
    Ok(files)
}

/// 埋め込みの中身を取得する。
pub fn fetch_embed(fetcher: &dyn Fetcher, url: &str, target: &Target) -> Result<Vec<EmbedFile>> {
    let files = match target {
        Target::File { raw_url, file_name, lines } => {
            let text = fetch_text(fetcher, raw_url)?;
            let (start_line, content) = slice_lines(&text, *lines)?;
            vec![EmbedFile { file_name: file_name.clone(), source_url: url.to_string(), start_line, content }]
        }
        Target::Gist { page_url, id, file } => {
            let api_url = format!("https://api.github.com/gists/{id}");
            let fetched = fetcher.fetch(&api_url, MAX_SOURCE_BYTES, PURPOSE)?;
            gist_files(fetcher, page_url, &fetched.body, file.as_deref())?
        }
    };
    let total: usize = files.iter().map(|file| file.content.len()).sum();
    if total > MAX_EMBED_BYTES {
        bail!("中身が {MAX_EMBED_BYTES} バイトを超えています（{total} バイト）");
    }
    Ok(files)
}

/// URL から作る、ファイル名に使う名前。
fn key(url: &str) -> String {
    blake3::hash(url.as_bytes()).to_hex()[..16].to_string()
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    url: String,
    files: Vec<EmbedFile>,
}

fn cache_path(site_dir: &Path, url: &str) -> PathBuf {
    site_dir.join(linkcard::CACHE_DIR).join(EMBED_CACHE).join(format!("{}.json", key(url)))
}

fn read_cache(site_dir: &Path, url: &str) -> Option<Vec<EmbedFile>> {
    let entry: CacheEntry = serde_json::from_slice(&fs::read(cache_path(site_dir, url)).ok()?).ok()?;
    (entry.url == url).then_some(entry.files)
}

/// 取得した結果を保存してあるか。見本のキャッシュのテスト（ADR 0066）で使う。
#[cfg(test)]
pub fn is_cached(site_dir: &Path, url: &str) -> bool {
    read_cache(site_dir, url).is_some()
}

fn write_cache(site_dir: &Path, url: &str, files: &[EmbedFile]) -> Result<()> {
    let path = cache_path(site_dir, url);
    let dir = path.parent().expect("キャッシュのディレクトリ");
    fs::create_dir_all(dir).with_context(|| format!("{} を作れません", dir.display()))?;
    let entry = CacheEntry { url: url.to_string(), files: files.to_vec() };
    fs::write(&path, serde_json::to_vec_pretty(&entry)?)?;
    Ok(())
}

/// 本文のコードの埋め込みを集める。保存済みのものはそれを使い、なければ取得して保存する。
/// 取得できなかったものは警告を出して飛ばす（その URL は今までどおりリンクカードになる。ビルドは止めない）。
pub fn collect(site_dir: &Path, sources: &[String], fetcher: &dyn Fetcher) -> Vec<CodeEmbed> {
    let urls: BTreeSet<String> = sources.iter().flat_map(|source| linkcard::card_urls(source)).collect();
    let mut embeds = Vec::new();
    for url in urls {
        let Some(target) = target(&url) else { continue };
        if let Some(files) = read_cache(site_dir, &url) {
            embeds.push(CodeEmbed { url, files });
            continue;
        }
        if fetcher.offline() {
            continue;
        }
        eprintln!("コードの埋め込みを取得しています: {url}");
        match fetch_embed(fetcher, &url, &target) {
            Ok(files) => {
                if let Err(error) = write_cache(site_dir, &url, &files) {
                    eprintln!("警告: コードの埋め込みを保存できませんでした: {error:#}");
                }
                embeds.push(CodeEmbed { url, files });
            }
            Err(error) => eprintln!("警告: コードの埋め込みを取得できませんでした（リンクカードにします）: {url}: {error:#}"),
        }
    }
    embeds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linkcard::{Fetched, Offline};
    use std::cell::RefCell;
    use std::collections::HashMap;

    struct FakeFetcher {
        responses: HashMap<String, Vec<u8>>,
        requested: RefCell<Vec<(String, String)>>,
    }

    impl FakeFetcher {
        fn new(responses: &[(&str, &[u8])]) -> Self {
            FakeFetcher {
                responses: responses.iter().map(|(url, body)| (url.to_string(), body.to_vec())).collect(),
                requested: RefCell::new(Vec::new()),
            }
        }
        fn urls(&self) -> Vec<String> {
            self.requested.borrow().iter().map(|(url, _)| url.clone()).collect()
        }
    }

    impl Fetcher for FakeFetcher {
        fn fetch(&self, url: &str, _max_bytes: u64, purpose: &str) -> Result<Fetched> {
            self.requested.borrow_mut().push((url.to_string(), purpose.to_string()));
            let body = self.responses.get(url).ok_or_else(|| anyhow!("404"))?;
            Ok(Fetched { url: url.to_string(), body: body.clone() })
        }
    }

    const FILE_URL: &str = "https://github.com/tett23/sqlite-cms/blob/main/cli/src/main.rs";
    const RAW_URL: &str = "https://raw.githubusercontent.com/tett23/sqlite-cms/main/cli/src/main.rs";
    const GIST_ID: &str = "0123456789abcdef0123";

    fn file(lines: Option<(usize, Option<usize>)>) -> Option<Target> {
        Some(Target::File { raw_url: RAW_URL.into(), file_name: "main.rs".into(), lines })
    }

    #[test]
    fn github_file_urls_are_targets() {
        assert_eq!(target(FILE_URL), file(None));
        assert_eq!(target(&format!("{FILE_URL}#L10")), file(Some((10, None))));
        assert_eq!(target(&format!("{FILE_URL}#L10-L20")), file(Some((10, Some(20)))));
        // `/` を含むブランチ名は、そのまま raw の URL に写す。
        assert_eq!(
            target("https://github.com/o/r/blob/feature/x/src/a%20b.rs"),
            Some(Target::File { raw_url: "https://raw.githubusercontent.com/o/r/feature/x/src/a%20b.rs".into(), file_name: "a b.rs".into(), lines: None })
        );
    }

    #[test]
    fn other_urls_are_not_targets() {
        for url in [
            "http://github.com/tett23/sqlite-cms/blob/main/cli/src/main.rs",
            "https://github.com/tett23/sqlite-cms/blob/main/cli/src/main.rs?plain=1",
            "https://github.com/tett23/sqlite-cms/raw/main/cli/src/main.rs",
            "https://github.com/tett23/sqlite-cms/tree/main/cli/src",
            "https://github.com/tett23/sqlite-cms/blame/main/cli/src/main.rs",
            "https://github.com/tett23/sqlite-cms/blob/main",
            "https://github.com/tett23/sqlite-cms",
            "https://github.com/tett23/sqlite-cms/blob/main/cli/src/main.rs#readme",
            "https://github.com/tett23/sqlite-cms/blob/main/cli/src/main.rs#L0",
            "https://github.com/tett23/sqlite-cms/blob/main/cli/src/main.rs#L20-L10",
            "https://github.com/tett23/sqlite-cms/blob/main/cli/src/main.rs#L10-20",
            "https://raw.githubusercontent.com/tett23/sqlite-cms/main/cli/src/main.rs",
            "https://gitlab.com/tett23/sqlite-cms/blob/main/cli/src/main.rs",
            "https://gist.github.com/tett23",
            "https://gist.github.com/abcdef",
            "https://gist.github.com/tett23/not-hex",
            "https://gist.github.com/tett23/0123456789abcdef0123#comments",
            "https://gist.github.com/tett23/0123456789abcdef0123/revisions",
            "https://example.com/",
        ] {
            assert_eq!(target(url), None, "{url}");
        }
    }

    #[test]
    fn gist_urls_are_targets() {
        let gist = |page_url: &str, file: Option<&str>| Some(Target::Gist { page_url: page_url.into(), id: GIST_ID.into(), file: file.map(str::to_string) });
        let with_user = format!("https://gist.github.com/tett23/{GIST_ID}");
        let without_user = format!("https://gist.github.com/{GIST_ID}");
        assert_eq!(target(&with_user), gist(&with_user, None));
        assert_eq!(target(&without_user), gist(&without_user, None));
        assert_eq!(target(&format!("{with_user}#file-hello-rb")), gist(&with_user, Some("hello-rb")));
    }

    #[test]
    fn file_names_are_percent_decoded_only_when_valid() {
        assert_eq!(percent_decode("%E3%81%82.rs"), "あ.rs");
        assert_eq!(percent_decode("a%20b.rs"), "a b.rs");
        // 16 進でないもの、末尾の `%`、戻すと UTF-8 にならないものは、書かれたまま。
        assert_eq!(percent_decode("100%ZZ.rs"), "100%ZZ.rs");
        assert_eq!(percent_decode("a%"), "a%");
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("%FF.rs"), "%FF.rs");
    }

    #[test]
    fn gist_files_without_content_are_fetched_from_raw() {
        let json = r#"{"files":{"a.rs":{"filename":"a.rs","content":null,"truncated":false,"raw_url":"https://gist.githubusercontent.com/u/id/raw/a.rs"}}}"#;
        let fetcher = FakeFetcher::new(&[("https://gist.githubusercontent.com/u/id/raw/a.rs", b"fn a() {}\n")]);
        let files = gist_files(&fetcher, "https://gist.github.com/x", json.as_bytes(), None).unwrap();
        assert_eq!(files[0].content, "fn a() {}");
    }

    #[test]
    fn gist_anchors_lowercase_and_replace_non_alphanumerics() {
        assert_eq!(gist_anchor("Hello_World.RB"), "hello-world-rb");
        assert_eq!(gist_anchor("gistfile1.txt"), "gistfile1-txt");
    }

    #[test]
    fn slices_line_ranges() {
        let text = "a\nb\nc\nd\n";
        assert_eq!(slice_lines(text, None).unwrap(), (1, "a\nb\nc\nd".into()));
        assert_eq!(slice_lines(text, Some((2, None))).unwrap(), (2, "b".into()));
        assert_eq!(slice_lines(text, Some((2, Some(3)))).unwrap(), (2, "b\nc".into()));
        // 終わりを超えるところは切り詰める。
        assert_eq!(slice_lines(text, Some((3, Some(10)))).unwrap(), (3, "c\nd".into()));
        assert_eq!(slice_lines(text, Some((4, None))).unwrap(), (4, "d".into()));
        // まったくファイルにないときは失敗。
        assert!(slice_lines(text, Some((5, Some(6)))).is_err());
        assert!(slice_lines("", Some((1, None))).is_err());
    }

    #[test]
    fn fetches_files_from_raw_and_normalizes_crlf() {
        let fetcher = FakeFetcher::new(&[(RAW_URL, b"fn a() {}\r\nfn b() {}\r\nfn c() {}\r\n")]);
        let url = format!("{FILE_URL}#L2-L3");
        let files = fetch_embed(&fetcher, &url, &target(&url).unwrap()).unwrap();
        assert_eq!(files, [EmbedFile { file_name: "main.rs".into(), source_url: url, start_line: 2, content: "fn b() {}\nfn c() {}".into() }]);
        assert_eq!(fetcher.requested.borrow()[0], (RAW_URL.to_string(), "code embed".to_string()));
    }

    #[test]
    fn rejects_binary_and_non_utf8_and_too_large_contents() {
        let url = FILE_URL;
        let target = target(url).unwrap();
        for body in [&b"\x89PNG\r\n\x1a\n\0\0"[..], &b"a\xffb"[..], &b"a\0b"[..]] {
            assert!(fetch_embed(&FakeFetcher::new(&[(RAW_URL, body)]), url, &target).is_err());
        }
        let large = "a\n".repeat(MAX_EMBED_BYTES / 2 + 1);
        assert!(fetch_embed(&FakeFetcher::new(&[(RAW_URL, large.as_bytes())]), url, &target).is_err());
        // 行の範囲があれば、切り出した後の大きさで比べる。
        let ranged = format!("{url}#L1-L3");
        let files = fetch_embed(&FakeFetcher::new(&[(RAW_URL, large.as_bytes())]), &ranged, &self::target(&ranged).unwrap()).unwrap();
        assert_eq!(files[0].content, "a\na\na");
    }

    const GIST_JSON: &str = r#"{"files":{
        "b.py":{"filename":"b.py","content":"print(1)\n","truncated":false,"raw_url":"https://gist.githubusercontent.com/u/id/raw/b.py"},
        "A_File.RS":{"filename":"A_File.RS","content":"fn","truncated":true,"raw_url":"https://gist.githubusercontent.com/u/id/raw/A_File.RS"}
    }}"#;

    #[test]
    fn gist_files_are_ordered_by_name_and_truncated_ones_are_refetched() {
        let page = "https://gist.github.com/u/0123456789abcdef0123";
        let fetcher = FakeFetcher::new(&[("https://gist.githubusercontent.com/u/id/raw/A_File.RS", b"fn main() {}\n")]);
        let files = gist_files(&fetcher, page, GIST_JSON.as_bytes(), None).unwrap();
        assert_eq!(
            files,
            [
                EmbedFile { file_name: "A_File.RS".into(), source_url: format!("{page}#file-a-file-rs"), start_line: 1, content: "fn main() {}".into() },
                EmbedFile { file_name: "b.py".into(), source_url: format!("{page}#file-b-py"), start_line: 1, content: "print(1)".into() },
            ]
        );
        // `#file-` の名前で一つに絞る。GitHub の書き方（`_` を残すもの）でも合う。
        let only = gist_files(&fetcher, page, GIST_JSON.as_bytes(), Some("a_file-rs")).unwrap();
        assert_eq!(only.iter().map(|f| f.file_name.as_str()).collect::<Vec<_>>(), ["A_File.RS"]);
        assert!(gist_files(&fetcher, page, GIST_JSON.as_bytes(), Some("missing-rs")).is_err());
    }

    #[test]
    fn gist_raw_urls_on_other_hosts_are_not_used() {
        let json = r#"{"files":{"a.rs":{"filename":"a.rs","content":"","truncated":true,"raw_url":"https://example.com/a.rs"}}}"#;
        let fetcher = FakeFetcher::new(&[("https://example.com/a.rs", b"fn")]);
        assert!(gist_files(&fetcher, "https://gist.github.com/x", json.as_bytes(), None).is_err());
        assert!(fetcher.urls().is_empty());
    }

    #[test]
    fn fetches_gists_from_the_api() {
        let url = format!("https://gist.github.com/u/{GIST_ID}#file-b-py");
        let fetcher = FakeFetcher::new(&[(&format!("https://api.github.com/gists/{GIST_ID}"), GIST_JSON.as_bytes())]);
        let files = fetch_embed(&fetcher, &url, &target(&url).unwrap()).unwrap();
        assert_eq!(files.iter().map(|f| f.content.as_str()).collect::<Vec<_>>(), ["print(1)"]);
    }

    #[test]
    fn collect_caches_successes_and_skips_failures() {
        let site = crate::testutil::tempdir();
        let missing = "https://github.com/o/r/blob/main/missing.rs";
        let sources = vec![format!("{FILE_URL}\n\n{missing}\n\nhttps://example.com/\n")];
        let fetcher = FakeFetcher::new(&[(RAW_URL, b"fn main() {}\n")]);
        let embeds = collect(site.path(), &sources, &fetcher);
        assert_eq!(embeds.iter().map(|e| e.url.as_str()).collect::<Vec<_>>(), [FILE_URL]);
        // 対象でない URL は取得しない。
        assert_eq!(fetcher.urls().len(), 2);

        // 保存した結果を使い、取得し直さない。失敗したものは保存しないので、取得し直す。
        let again = FakeFetcher::new(&[]);
        assert_eq!(collect(site.path(), &sources, &again), embeds);
        assert_eq!(again.urls(), ["https://raw.githubusercontent.com/o/r/main/missing.rs"]);

        // オフラインでは取得せず、保存した結果だけを使う。
        assert_eq!(collect(site.path(), &sources, &Offline), embeds);
        let other = crate::testutil::tempdir();
        assert!(collect(other.path(), &sources, &Offline).is_empty());
    }
}
