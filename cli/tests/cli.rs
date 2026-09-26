use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

#[path = "../src/testutil.rs"]
mod testutil;

fn sqlite_cms() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sqlite-cms"));
    for name in ["CLOUDFLARE_API_TOKEN", "CLOUDFLARE_ACCOUNT_ID", "CLOUDFLARE_API_BASE_URL"] {
        command.env_remove(name);
    }
    // 見本の記事のリンクカードの画像を、テストのたびに外部から取得しない。
    command.env("SQLITE_CMS_OFFLINE", "1");
    command
}

fn run(args: &[&str], current_dir: &Path) -> Output {
    sqlite_cms().args(args).current_dir(current_dir).output().unwrap()
}

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../example")
}

fn require_spa() {
    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/dist/index.html");
    assert!(
        index.is_file(),
        "SPA が埋め込まれていません。先に npm --prefix web run build を実行してください"
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// ディレクトリを写す。`.env` は写さない。
/// 開発者が example/.env に本物の認証情報を書いていても、テストが本物の Cloudflare に公開しないようにするためである。
fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|name| name == ".env") {
            continue;
        }
        let target = dst.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap();
        }
    }
}

#[test]
fn help_prints_to_stdout_and_exits_successfully() {
    let tmp = testutil::tempdir();
    let output = run(&["--help"], tmp.path());

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let help = stdout(&output);
    for section in ["usage: sqlite-cms", "コマンド:", "serve", "build", "deploy", "記事リポジトリの構成:", "deploy の公開先", "github-pages", "rsync", "例:"] {
        assert!(help.contains(section), "ヘルプに {section} がありません");
    }
    assert!(!help.contains("--data-only"), "開発者向けのオプションは利用者向けのヘルプに出さない");
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
}

#[test]
fn short_help_is_the_same_as_long_help() {
    let tmp = testutil::tempdir();
    assert_eq!(run(&["-h"], tmp.path()).stdout, run(&["--help"], tmp.path()).stdout);
}

#[test]
fn missing_command_fails_with_hint_on_stderr() {
    let tmp = testutil::tempdir();
    let output = run(&[], tmp.path());

    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    let err = stderr(&output);
    assert!(err.contains("コマンドを指定してください"));
    assert!(err.contains("sqlite-cms --help"));
}

#[test]
fn build_writes_complete_site() {
    require_spa();
    let tmp = testutil::tempdir();
    let out = tmp.path().join("dist");
    let output = run(&["build", example_dir().to_str().unwrap(), "--out", out.to_str().unwrap()], tmp.path());

    assert!(output.status.success(), "{}", stderr(&output));
    for file in ["index.html", "_headers", "db/manifest.json", "media/sample.svg", "favicon.svg", "rss.xml"] {
        assert!(out.join(file).is_file(), "{file} がありません");
    }
    assert!(fs::read_dir(out.join("assets")).unwrap().count() > 0);
}

#[test]
fn build_data_only_writes_db_and_media() {
    let tmp = testutil::tempdir();
    let out = tmp.path().join("public");
    let output = run(
        &["build", example_dir().to_str().unwrap(), "--out", out.to_str().unwrap(), "--data-only"],
        tmp.path(),
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(out.join("db/manifest.json").is_file());
    assert!(out.join("media/sample.svg").is_file());
    assert!(!out.join("index.html").exists());
}

#[test]
fn build_uses_current_directory_by_default() {
    let tmp = testutil::tempdir();
    let out = tmp.path().join("public");
    let output = sqlite_cms()
        .args(["build", "--data-only", "--out", out.to_str().unwrap()])
        .current_dir(example_dir())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(out.join("db/manifest.json").is_file());
}

struct ServeProcess {
    child: Child,
    address: String,
}

impl Drop for ServeProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn http_get(address: &str, path: &str) -> (u16, String, Vec<u8>) {
    let (status, head, body) = http_request(address, path, "");
    let content_type = head
        .lines()
        .find_map(|l| l.strip_prefix("Content-Type: "))
        .unwrap_or_default()
        .to_string();
    (status, content_type, body)
}

/// 追加のヘッダ（`名前: 値\r\n` の並び）を付けて GET し、状態、ヘッダ、本文を返す。
fn http_request(address: &str, path: &str, headers: &str) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(address).unwrap();
    write!(stream, "GET {path} HTTP/1.1\r\nHost: {address}\r\n{headers}Connection: close\r\n\r\n").unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(raw[..split].to_vec()).unwrap();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, head, raw[split + 4..].to_vec())
}

fn gunzip(data: &[u8]) -> Vec<u8> {
    let mut child = Command::new("gzip")
        .arg("-dc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("gzip を起動できません");
    child.stdin.take().unwrap().write_all(data).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    output.stdout
}

#[test]
fn serve_previews_the_site() {
    require_spa();
    let mut child = sqlite_cms()
        .args(["serve", example_dir().to_str().unwrap(), "--port", "0"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    let address = line
        .strip_prefix("http://")
        .and_then(|rest| rest.split('/').next())
        .unwrap_or_else(|| panic!("URL が出力されていません: {line}"))
        .to_string();
    let server = ServeProcess { child, address };

    let (status, content_type, body) = http_get(&server.address, "/");
    assert_eq!(status, 200);
    assert!(content_type.starts_with("text/html"));
    assert!(String::from_utf8_lossy(&body).contains("id=\"root\""));

    let (status, _, body) = http_get(&server.address, "/posts/hello");
    assert_eq!(status, 200);
    assert!(String::from_utf8_lossy(&body).contains("id=\"root\""));

    let (status, content_type, body) = http_get(&server.address, "/db/manifest.json");
    assert_eq!(status, 200);
    assert_eq!(content_type, "application/json");
    assert!(String::from_utf8_lossy(&body).contains("/db/articles-"));

    let (status, content_type, _) = http_get(&server.address, "/media/sample.svg");
    assert_eq!(status, 200);
    assert_eq!(content_type, "image/svg+xml");

    let (status, content_type, body) = http_get(&server.address, "/favicon.svg");
    assert_eq!(status, 200);
    assert_eq!(content_type, "image/svg+xml");
    assert!(String::from_utf8_lossy(&body).starts_with("<svg"));

    assert_eq!(http_get(&server.address, "/assets/missing.js").0, 404);

    // Accept-Encoding に gzip があれば、圧縮して返す（ADR 0026）。
    let (_, plain_head, plain) = http_request(&server.address, "/", "");
    assert!(!plain_head.contains("Content-Encoding"));
    let (status, head, body) = http_request(&server.address, "/", "Accept-Encoding: gzip, deflate, br\r\n");
    assert_eq!(status, 200);
    assert!(head.contains("\r\nContent-Encoding: gzip\r\n"), "{head}");
    assert!(head.contains("\r\nVary: Accept-Encoding\r\n"), "{head}");
    assert!(head.contains(&format!("\r\nContent-Length: {}\r\n", body.len())), "{head}");
    assert_eq!(gunzip(&body), plain);
}

#[derive(Clone, Debug)]
struct Recorded {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn ok(result: &str) -> String {
    format!(r#"{{"success":true,"errors":[],"messages":[],"result":{result}}}"#)
}

fn mock_response(request: &Recorded) -> (u16, String) {
    let path = request.path.as_str();
    match request.method.as_str() {
        "POST" if path.ends_with("/workers/scripts/blog-test/assets-upload-session") => {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            let mut hashes: Vec<String> = body["manifest"]
                .as_object()
                .unwrap()
                .values()
                .map(|entry| entry["hash"].as_str().unwrap().to_string())
                .collect();
            hashes.sort();
            hashes.dedup();
            (200, ok(&serde_json::json!({ "jwt": "upload-jwt", "buckets": [hashes] }).to_string()))
        }
        "POST" if path.ends_with("/workers/assets/upload?base64=true") => (201, ok(r#"{"jwt":"completion-jwt"}"#)),
        "PUT" if path.ends_with("/workers/scripts/blog-test") => (200, ok("{}")),
        "POST" if path.ends_with("/workers/scripts/blog-test/subdomain") => (200, ok(r#"{"enabled":true}"#)),
        "GET" if path.ends_with("/workers/subdomain") => (200, ok(r#"{"subdomain":"example"}"#)),
        _ => (404, r#"{"success":false,"errors":[{"code":7003,"message":"not mocked"}],"result":null}"#.to_string()),
    }
}

fn handle_mock_connection(stream: TcpStream, recorded: Arc<Mutex<Vec<Recorded>>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;
    loop {
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
            return;
        }
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap().to_string();
        let path = parts.next().unwrap().to_string();

        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
        assert!(!headers.contains_key("transfer-encoding"), "chunked は想定していません");
        let length: usize = headers.get("content-length").map_or(0, |v| v.parse().unwrap());
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();

        let request = Recorded { method, path, headers, body };
        let (status, response) = mock_response(&request);
        recorded.lock().unwrap().push(request);
        write!(
            writer,
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{response}",
            response.len()
        )
        .unwrap();
        writer.flush().unwrap();
    }
}

fn start_mock_cloudflare() -> (String, Arc<Mutex<Vec<Recorded>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/client/v4", listener.local_addr().unwrap());
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let shared = Arc::clone(&recorded);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let shared = Arc::clone(&shared);
            thread::spawn(move || handle_mock_connection(stream, shared));
        }
    });
    (base, recorded)
}

fn site_with_deploy(tmp: &Path) -> PathBuf {
    let site = tmp.join("site");
    copy_dir(&example_dir(), &site);
    let mut toml = fs::read_to_string(site.join("site.toml")).unwrap();
    toml.push_str("\n[deploy]\nworker = \"blog-test\"\n");
    fs::write(site.join("site.toml"), toml).unwrap();
    site
}

#[test]
fn deploy_uploads_site_through_cloudflare_api() {
    require_spa();
    let tmp = testutil::tempdir();
    let site = site_with_deploy(tmp.path());
    let (base, recorded) = start_mock_cloudflare();

    let output = sqlite_cms()
        .args(["deploy", site.to_str().unwrap()])
        .env("CLOUDFLARE_API_BASE_URL", &base)
        .env("CLOUDFLARE_API_TOKEN", "test-token")
        .env("CLOUDFLARE_ACCOUNT_ID", "abc123")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("https://blog-test.example.workers.dev"));
    assert!(!stdout(&output).contains("test-token") && !stderr(&output).contains("test-token"));

    let requests = recorded.lock().unwrap().clone();
    let lines: Vec<String> = requests.iter().map(|r| format!("{} {}", r.method, r.path)).collect();
    assert_eq!(
        lines,
        [
            "POST /client/v4/accounts/abc123/workers/scripts/blog-test/assets-upload-session",
            "POST /client/v4/accounts/abc123/workers/assets/upload?base64=true",
            "PUT /client/v4/accounts/abc123/workers/scripts/blog-test",
            "POST /client/v4/accounts/abc123/workers/scripts/blog-test/subdomain",
            "GET /client/v4/accounts/abc123/workers/subdomain",
        ]
    );

    let session: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    let manifest = session["manifest"].as_object().unwrap();
    for path in ["/index.html", "/db/manifest.json", "/media/sample.svg"] {
        assert!(manifest.contains_key(path), "マニフェストに {path} がありません");
    }
    assert!(!manifest.contains_key("/_headers"));
    assert_eq!(requests[0].headers["authorization"], "Bearer test-token");

    let upload = String::from_utf8(requests[1].body.clone()).unwrap();
    assert_eq!(requests[1].headers["authorization"], "Bearer upload-jwt");
    assert!(requests[1].headers["content-type"].starts_with("multipart/form-data; boundary="));
    for entry in manifest.values() {
        assert!(upload.contains(&format!("name=\"{}\"", entry["hash"].as_str().unwrap())));
    }

    let put = String::from_utf8(requests[2].body.clone()).unwrap();
    assert!(put.contains("completion-jwt"));
    assert!(put.contains("single-page-application"));
    assert!(put.contains("Cache-Control: no-cache"));
}

#[test]
fn deploy_reads_credentials_from_dotenv_and_prefers_the_environment() {
    require_spa();
    let tmp = testutil::tempdir();
    let site = site_with_deploy(tmp.path());
    let (base, recorded) = start_mock_cloudflare();
    fs::write(
        site.join(".env"),
        format!("# 認証情報\nCLOUDFLARE_API_TOKEN=dotenv-token\nCLOUDFLARE_ACCOUNT_ID='abc123'\nCLOUDFLARE_API_BASE_URL=\"{base}\"\n"),
    )
    .unwrap();

    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    let requests = recorded.lock().unwrap().clone();
    assert_eq!(requests[0].path, "/client/v4/accounts/abc123/workers/scripts/blog-test/assets-upload-session");
    assert_eq!(requests[0].headers["authorization"], "Bearer dotenv-token");
    assert!(!stdout(&output).contains("dotenv-token") && !stderr(&output).contains("dotenv-token"));

    recorded.lock().unwrap().clear();
    let output = sqlite_cms()
        .args(["deploy", site.to_str().unwrap()])
        .env("CLOUDFLARE_API_TOKEN", "process-token")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(recorded.lock().unwrap()[0].headers["authorization"], "Bearer process-token");
}

#[test]
fn deploy_reports_broken_dotenv_lines() {
    let tmp = testutil::tempdir();
    let site = site_with_deploy(tmp.path());
    fs::write(site.join(".env"), "CLOUDFLARE_API_TOKEN=x\nこれは代入ではない\n").unwrap();
    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains(".env の 2 行目"), "{}", stderr(&output));
}

#[test]
fn deploy_requires_deploy_section_and_credentials() {
    let tmp = testutil::tempdir();

    let output = run(&["deploy", example_dir().to_str().unwrap()], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("[deploy]"));

    let site = site_with_deploy(tmp.path());
    assert!(!site.join(".env").exists(), "テストの記事リポジトリに .env を写してはいけない");
    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("CLOUDFLARE_API_TOKEN"));
}

fn empty_site(tmp: &Path) -> PathBuf {
    let site = tmp.join("site");
    fs::create_dir_all(&site).unwrap();
    fs::write(site.join("site.toml"), "title = \"t\"\n").unwrap();
    site
}

#[test]
fn new_creates_a_document_that_builds() {
    let tmp = testutil::tempdir();
    let site = empty_site(tmp.path());

    let output = run(
        &["new", "post", "hello", "--title", "[はじめまして] \"引用\"", "--date", "2026-09-26"],
        &site,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("hello.md を作りました"));
    let written = fs::read_to_string(site.join("content/posts/hello.md")).unwrap();
    assert!(written.starts_with("---\ntitle: \"[はじめまして] \\\"引用\\\"\"\ndate: 2026-09-26\n---\n"));

    for (kind, slug) in [("article", "long"), ("page", "about")] {
        let output = run(&["new", kind, slug], &site);
        assert!(output.status.success(), "{}", stderr(&output));
    }

    let out = tmp.path().join("public");
    let output = run(&["build", "--data-only", "--out", out.to_str().unwrap()], &site);
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn new_refuses_to_overwrite() {
    let tmp = testutil::tempdir();
    let site = empty_site(tmp.path());
    assert!(run(&["new", "post", "hello", "--title", "一本目"], &site).status.success());

    let output = run(&["new", "post", "hello", "--title", "二本目"], &site);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("すでにあります"));
    assert!(fs::read_to_string(site.join("content/posts/hello.md")).unwrap().contains("一本目"));
}

#[test]
fn new_requires_a_site() {
    let tmp = testutil::tempdir();
    let output = run(&["new", "post", "hello"], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("site.toml がありません"));
    assert!(!tmp.path().join("content").exists());
}

/// 記事リポジトリの timezone（任意）と環境変数 TZ を与えて new を実行し、既定の日付を返す。
fn default_date(site_timezone: Option<&str>, env_tz: &str) -> String {
    let tmp = testutil::tempdir();
    let site = empty_site(tmp.path());
    if let Some(timezone) = site_timezone {
        fs::write(site.join("site.toml"), format!("title = \"t\"\ntimezone = \"{timezone}\"\n")).unwrap();
    }
    let output = sqlite_cms().args(["new", "post", "x"]).env("TZ", env_tz).current_dir(&site).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let written = fs::read_to_string(site.join("content/posts/x.md")).unwrap();
    written.lines().find_map(|l| l.strip_prefix("date: ")).unwrap().to_string()
}

// UTC+14（Pacific/Kiritimati）と UTC-12（Etc/GMT+12）は 26 時間ずれているので、いつ実行しても日付が異なる。
const EAST: &str = "Pacific/Kiritimati";
const WEST: &str = "Etc/GMT+12";

#[cfg(unix)]
#[test]
fn new_uses_the_environment_time_zone_by_default() {
    assert_ne!(default_date(None, EAST), default_date(None, WEST));
}

#[cfg(unix)]
#[test]
fn site_timezone_overrides_the_environment() {
    let east = default_date(None, EAST);
    let west = default_date(None, WEST);
    assert_eq!(default_date(Some(EAST), WEST), east);
    assert_eq!(default_date(Some(WEST), EAST), west);
}

#[test]
fn site_timezone_accepts_fixed_offsets() {
    #[cfg(unix)]
    {
        assert_eq!(default_date(Some("+14:00"), WEST), default_date(None, EAST));
        assert_eq!(default_date(Some("-12:00"), EAST), default_date(None, WEST));
    }
    let _ = default_date(Some("UTC"), "UTC");
}

#[test]
fn unknown_timezone_fails_new_but_not_build() {
    let tmp = testutil::tempdir();
    let site = empty_site(tmp.path());
    fs::write(site.join("site.toml"), "title = \"t\"\ntimezone = \"Mars/Olympus_Mons\"\n").unwrap();

    let output = run(&["new", "post", "x"], &site);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Mars/Olympus_Mons"), "{}", stderr(&output));
    assert!(!site.join("content/posts/x.md").exists());

    let output = run(&["new", "post", "x", "--date", "2026-09-26"], &site);
    assert!(output.status.success(), "{}", stderr(&output));

    let out = tmp.path().join("public");
    let output = run(&["build", "--data-only", "--out", out.to_str().unwrap()], &site);
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn init_new_build_is_the_first_run_flow() {
    require_spa();
    let tmp = testutil::tempdir();

    let output = run(&["init", "my-blog", "--title", "はじめてのサイト"], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("site.toml を作りました"), "{out}");
    assert!(out.contains("sqlite-cms new post <SLUG> my-blog"), "{out}");

    let site = tmp.path().join("my-blog");
    assert!(run(&["new", "post", "hello", "--date", "2026-09-26"], &site).status.success());

    let dist = tmp.path().join("dist");
    let output = run(&["build", "--out", dist.to_str().unwrap()], &site);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(dist.join("index.html").is_file());
    assert!(site.join(".env.example").is_file());
    assert!(fs::read_to_string(site.join(".gitignore")).unwrap().lines().any(|line| line == ".env"));
    assert!(!dist.join(".env.example").exists());
    assert!(fs::read_to_string(dist.join("index.html")).unwrap().contains(r#"<link rel="icon" type="image/svg+xml" href="/favicon.svg" />"#));
    assert_eq!(fs::read(dist.join("favicon.svg")).unwrap(), fs::read(site.join("content/favicon.svg")).unwrap());
    assert!(!dist.join("media/.gitkeep").exists());
}

#[test]
fn init_refuses_existing_site_unless_forced() {
    let tmp = testutil::tempdir();
    assert!(run(&["init"], tmp.path()).status.success());
    fs::write(tmp.path().join("content/posts/hello.md"), "---\ntitle: 記事\ndate: 2026-09-26\n---\n").unwrap();
    fs::write(tmp.path().join("site.toml"), "title = \"編集済み\"\n").unwrap();

    let output = run(&["init"], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--force"), "{}", stderr(&output));
    assert_eq!(fs::read_to_string(tmp.path().join("site.toml")).unwrap(), "title = \"編集済み\"\n");

    let output = run(&["init", "--force"], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("site.toml を上書きしました"));
    assert!(!fs::read_to_string(tmp.path().join("site.toml")).unwrap().contains("編集済み"));
    assert!(tmp.path().join("content/posts/hello.md").is_file());
}

#[test]
fn new_without_slug_uses_the_date_and_suffixes() {
    let tmp = testutil::tempdir();
    let site = empty_site(tmp.path());

    let first = run(&["new", "post", "--date", "2026-09-26"], &site);
    assert!(first.status.success(), "{}", stderr(&first));
    assert!(stdout(&first).contains("2026-09-26.md を作りました"));
    let second = run(&["new", "post", "--date", "2026-09-26", "--title", "二本目"], &site);
    assert!(second.status.success(), "{}", stderr(&second));
    assert!(stdout(&second).contains("2026-09-26-2.md を作りました"));

    let out = tmp.path().join("public");
    let output = run(&["build", "--data-only", "--out", out.to_str().unwrap()], &site);
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn new_without_slug_names_the_file_after_the_date_it_writes() {
    let tmp = testutil::tempdir();
    let site = empty_site(tmp.path());
    fs::write(site.join("site.toml"), "title = \"t\"\ntimezone = \"+14:00\"\n").unwrap();

    let output = sqlite_cms().args(["new", "article"]).env("TZ", "Etc/GMT+12").current_dir(&site).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    let entries: Vec<PathBuf> = fs::read_dir(site.join("content/articles")).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(entries.len(), 1);
    let stem = entries[0].file_stem().unwrap().to_str().unwrap().to_string();
    let written = fs::read_to_string(&entries[0]).unwrap();
    assert!(written.contains(&format!("\ndate: {stem}\n")), "{stem}: {written}");
}

#[test]
fn new_without_slug_accepts_a_site_path() {
    let tmp = testutil::tempdir();
    let site = empty_site(tmp.path());
    let output = run(&["new", "post", "./site", "--date", "2026-09-26"], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(site.join("content/posts/2026-09-26.md").is_file());
}

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n fake png";

/// リンクカードの取得先の偽のサイト。/page は og:image を持つページ、/og.png はその画像を返す。
fn start_mock_site() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let requested = Arc::new(Mutex::new(Vec::new()));
    let shared = Arc::clone(&requested);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let path = request_line.split_whitespace().nth(1).unwrap_or("/").to_string();
            shared.lock().unwrap().push(path.clone());
            let (status, content_type, body): (&str, &str, Vec<u8>) = match path.as_str() {
                "/page" => ("200 OK", "text/html", br#"<html><head><meta property="og:image" content="/og.png"></head></html>"#.to_vec()),
                "/og.png" => ("200 OK", "image/png", PNG.to_vec()),
                _ => ("404 Not Found", "text/plain", b"not found".to_vec()),
            };
            let mut stream = stream;
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    (origin, requested)
}

#[test]
fn build_fetches_link_card_images_and_reuses_the_cache() {
    require_spa();
    let tmp = testutil::tempdir();
    let site = tmp.path().join("site");
    assert!(run(&["init", site.to_str().unwrap()], tmp.path()).status.success());
    let (origin, requested) = start_mock_site();
    fs::write(
        site.join("content/posts/cards.md"),
        format!("---\ntitle: カード\ndate: 2026-09-26\n---\n\n{origin}/page\n\n{origin}/missing\n"),
    )
    .unwrap();

    let dist = tmp.path().join("dist");
    let output = sqlite_cms()
        .args(["build", site.to_str().unwrap(), "--out", dist.to_str().unwrap()])
        .env_remove("SQLITE_CMS_OFFLINE")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains(&format!("警告: リンクカードの画像を取得できませんでした: {origin}/missing")), "{}", stderr(&output));

    let images: Vec<PathBuf> = fs::read_dir(dist.join("link-cards")).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].extension().unwrap(), "png");
    assert_eq!(fs::read(&images[0]).unwrap(), PNG);
    let db = fs::read_dir(dist.join("db")).unwrap().map(|e| e.unwrap().path()).find(|p| p.extension().is_some_and(|e| e == "sqlite")).unwrap();
    let conn = rusqlite::Connection::open(&db).unwrap();
    let (url, image_path): (String, String) =
        conn.query_row("SELECT url, image_path FROM link_cards", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!(url, format!("{origin}/page"));
    assert_eq!(image_path, format!("/link-cards/{}", images[0].file_name().unwrap().to_string_lossy()));
    assert!(fs::read_to_string(site.join(".gitignore")).unwrap().lines().any(|line| line == ".sqlite-cms-cache/"));

    // 二度目は保存した結果を使い、オフラインでも画像が入る。
    requested.lock().unwrap().clear();
    let output = run(&["build", site.to_str().unwrap(), "--out", dist.to_str().unwrap()], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(requested.lock().unwrap().is_empty());
    assert_eq!(fs::read_dir(dist.join("link-cards")).unwrap().count(), 1);
}

/// 見本の site.toml から、公開したサイトの url の行を外す（base_path を変えるテストで、パスが食い違わないように）。
fn without_site_url(toml: &str) -> String {
    toml.lines().filter(|line| !line.starts_with("url = \"https://gentle")).map(|line| format!("{line}\n")).collect()
}

/// 見本を写し、site.toml の末尾に設定を足した記事リポジトリ。
fn site_with_config(tmp: &Path, extra: &str) -> PathBuf {
    let site = tmp.join("site");
    copy_dir(&example_dir(), &site);
    let mut toml = fs::read_to_string(site.join("site.toml")).unwrap();
    toml.push_str(extra);
    fs::write(site.join("site.toml"), toml).unwrap();
    site
}

#[test]
fn deploy_with_rsync_syncs_into_an_empty_or_own_directory_only() {
    require_spa();
    let tmp = testutil::tempdir();
    let dest = tmp.path().join("www");
    let site = site_with_config(tmp.path(), &format!("\n[deploy]\ntarget = \"rsync\"\ndestination = \"{}\"\n", dest.display()));

    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    for file in ["index.html", "404.html", ".sqlite-cms", "db/manifest.json", "media/sample.svg"] {
        assert!(dest.join(file).is_file(), "{file} がありません");
    }

    // 二度目は、一式にないファイルを消す。
    fs::write(dest.join("stale.txt"), "old").unwrap();
    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!dest.join("stale.txt").exists());

    // 関係のないディレクトリには送らない。
    let foreign = tmp.path().join("foreign");
    fs::create_dir(&foreign).unwrap();
    fs::write(foreign.join("important.txt"), "keep").unwrap();
    let site = site_with_config(&tmp.path().join("other"), &format!("\n[deploy]\ntarget = \"rsync\"\ndestination = \"{}\"\n", foreign.display()));
    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("sqlite-cms の公開先でもありません"), "{}", stderr(&output));
    assert_eq!(fs::read_to_string(foreign.join("important.txt")).unwrap(), "keep");
    assert!(!foreign.join("index.html").exists());
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn deploy_to_github_pages_pushes_a_branch_without_touching_the_work_tree() {
    require_spa();
    let tmp = testutil::tempdir();
    let remote = tmp.path().join("remote.git");
    git(tmp.path(), &["init", "--quiet", "--bare", remote.to_str().unwrap()]);
    let site = site_with_config(
        tmp.path(),
        "base_path = \"/my-blog/\"\n\n[deploy]\ntarget = \"github-pages\"\ncname = \"blog.example.com\"\n",
    );
    // base_path は site.toml の先頭の表の中に置く必要があるので、[license] より前に書き直す。
    let toml = without_site_url(&fs::read_to_string(site.join("site.toml")).unwrap().replace("base_path = \"/my-blog/\"\n", ""));
    fs::write(site.join("site.toml"), format!("base_path = \"/my-blog/\"\n{toml}")).unwrap();
    git(&site, &["init", "--quiet", "-b", "main"]);
    git(&site, &["add", "."]);
    git(&site, &["commit", "--quiet", "-m", "記事"]);
    git(&site, &["remote", "add", "origin", remote.to_str().unwrap()]);
    let head = git(&site, &["rev-parse", "HEAD"]);

    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("Settings → Pages"), "{}", stdout(&output));

    let files = git(&remote, &["ls-tree", "-r", "--name-only", "gh-pages"]);
    for file in ["index.html", "404.html", ".nojekyll", "CNAME", "db/manifest.json", "media/sample.svg"] {
        assert!(files.lines().any(|line| line == file), "{file} がありません: {files}");
    }
    let index = git(&remote, &["show", "gh-pages:index.html"]);
    assert!(index.contains("src=\"/my-blog/assets/"), "{index}");
    assert!(index.contains("<meta name=\"sqlite-cms-base\" content=\"/my-blog/\""), "{index}");
    assert_eq!(git(&remote, &["show", "gh-pages:CNAME"]), "blog.example.com");
    assert!(git(&remote, &["log", "-1", "--format=%s", "gh-pages"]).contains(&head[..7]));

    // 記事リポジトリの作業ツリー、インデックス、ブランチは変わらない。
    assert_eq!(git(&site, &["status", "--porcelain"]), "");
    assert_eq!(git(&site, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&site, &["branch", "--format=%(refname:short)"]), "main");

    // 二度目も同じように上書きできる。
    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn deploy_to_github_pages_requires_a_git_repository() {
    let tmp = testutil::tempdir();
    let site = site_with_config(tmp.path(), "\n[deploy]\ntarget = \"github-pages\"\n");
    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Git のリポジトリ"), "{}", stderr(&output));
}

#[test]
fn serve_and_build_use_the_base_path() {
    require_spa();
    let tmp = testutil::tempdir();
    let site = tmp.path().join("site");
    copy_dir(&example_dir(), &site);
    let toml = without_site_url(&fs::read_to_string(site.join("site.toml")).unwrap());
    fs::write(site.join("site.toml"), format!("base_path = \"/blog\"\n{toml}")).unwrap();

    let dist = tmp.path().join("dist");
    let output = run(&["build", site.to_str().unwrap(), "--out", dist.to_str().unwrap()], tmp.path());
    assert!(output.status.success(), "{}", stderr(&output));
    let index = fs::read_to_string(dist.join("index.html")).unwrap();
    assert!(index.contains("href=\"/blog/favicon.svg\""), "{index}");
    assert_eq!(fs::read(dist.join("404.html")).unwrap(), index.as_bytes());

    let mut child = sqlite_cms()
        .args(["serve", site.to_str().unwrap(), "--port", "0"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    assert!(line.contains("/blog/ でプレビュー"), "{line}");
    let address = line.strip_prefix("http://").and_then(|rest| rest.split('/').next()).unwrap().to_string();
    let server = ServeProcess { child, address };

    let (status, _, body) = http_get(&server.address, "/blog/posts/hello");
    assert_eq!(status, 200);
    assert!(String::from_utf8_lossy(&body).contains("content=\"/blog/\""));
    assert_eq!(http_get(&server.address, "/blog/db/manifest.json").0, 200);
    let (status, head, _) = http_request(&server.address, "/", "");
    assert_eq!(status, 302);
    assert!(head.contains("\r\nLocation: /blog/\r\n"), "{head}");
}

/// serve を起動し、アドレスを返す。
fn start_serve(site: &Path, extra: &[&str]) -> ServeProcess {
    let mut child = sqlite_cms()
        .args(["serve", site.to_str().unwrap(), "--port", "0"])
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    let address = line.strip_prefix("http://").and_then(|rest| rest.split('/').next()).unwrap().to_string();
    ServeProcess { child, address }
}

fn manifest(address: &str) -> String {
    String::from_utf8(http_get(address, "/db/manifest.json").2).unwrap()
}

/// 再読み込みの知らせ（event: reload）が届くまで読む。届かなければ失敗する。
fn wait_for_reload(reader: &mut BufReader<TcpStream>) {
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).expect("再読み込みの知らせが届きません");
        assert!(read > 0, "接続が切れました");
        if line.trim() == "event: reload" {
            return;
        }
    }
}

#[test]
fn serve_rebuilds_on_changes_and_tells_the_browser_to_reload() {
    require_spa();
    let tmp = testutil::tempdir();
    let site = tmp.path().join("site");
    copy_dir(&example_dir(), &site);
    let server = start_serve(&site, &[]);

    let (_, _, index) = http_get(&server.address, "/");
    assert!(String::from_utf8_lossy(&index).contains("EventSource(\"/__sqlite-cms/events\")"));

    let mut events = TcpStream::connect(&server.address).unwrap();
    events.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
    write!(events, "GET /__sqlite-cms/events HTTP/1.1\r\nHost: {}\r\n\r\n", server.address).unwrap();
    let mut reader = BufReader::new(events);
    let mut status = String::new();
    reader.read_line(&mut status).unwrap();
    assert!(status.starts_with("HTTP/1.1 200"), "{status}");

    let before = manifest(&server.address);
    fs::write(site.join("content/posts/new-post.md"), "---\ntitle: 足した記事\ndate: 2026-09-27\n---\n\n本文。\n").unwrap();
    wait_for_reload(&mut reader);
    let after = manifest(&server.address);
    assert_ne!(before, after, "DB が変わっていません");

    // 書き誤りがあれば前の内容のまま配信し、直せば反映する。
    fs::write(site.join("content/posts/broken.md"), "---\ndate: 2026-09-27\n---\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1500));
    assert_eq!(manifest(&server.address), after);
    fs::write(site.join("content/posts/broken.md"), "---\ntitle: 直した\ndate: 2026-09-27\n---\n").unwrap();
    wait_for_reload(&mut reader);
    assert_ne!(manifest(&server.address), after);
}

#[test]
fn serve_without_reload_does_not_inject_the_script() {
    require_spa();
    let tmp = testutil::tempdir();
    let server = start_serve(&example_dir(), &["--no-reload"]);
    let (_, _, index) = http_get(&server.address, "/");
    assert!(!String::from_utf8_lossy(&index).contains("EventSource"));
    drop(tmp);
}
