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

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let path = entry.unwrap().path();
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
    for section in ["usage: sqlite-cms", "コマンド:", "serve", "build", "deploy", "記事リポジトリの構成:", "deploy に要るもの:", "例:"] {
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
    for file in ["index.html", "_headers", "db/manifest.json", "media/sample.svg"] {
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
    let mut stream = TcpStream::connect(address).unwrap();
    write!(stream, "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n").unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(raw[..split].to_vec()).unwrap();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    let content_type = head
        .lines()
        .find_map(|l| l.strip_prefix("Content-Type: "))
        .unwrap_or_default()
        .to_string();
    (status, content_type, raw[split + 4..].to_vec())
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

    assert_eq!(http_get(&server.address, "/assets/missing.js").0, 404);
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
fn deploy_requires_deploy_section_and_credentials() {
    let tmp = testutil::tempdir();

    let output = run(&["deploy", example_dir().to_str().unwrap()], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("[deploy]"));

    let site = site_with_deploy(tmp.path());
    let output = run(&["deploy", site.to_str().unwrap()], tmp.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("CLOUDFLARE_API_TOKEN"));
}
