use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;
use std::thread;

use crate::deploy::extension;
use crate::output::SiteOutput;

pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    /// gzip で圧縮したとき true。
    pub gzip: bool,
    /// 圧縮するかどうかを Accept-Encoding で決める種類なら true（Vary を付ける）。
    pub vary: bool,
    /// 転送先（302 のとき）。
    pub location: Option<String>,
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        302 => "Found",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "",
    }
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push(high * 16 + low);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn content_type(path: &str) -> String {
    crate::mime::by_extension(extension(path)).unwrap_or("application/octet-stream").to_string()
}

fn text(status: u16) -> Response {
    Response {
        status,
        content_type: "text/plain; charset=utf-8".into(),
        body: reason(status).as_bytes().to_vec(),
        gzip: false,
        vary: false,
        location: None,
    }
}

fn ok(content_type: String, body: Vec<u8>) -> Response {
    Response { status: 200, content_type, body, gzip: false, vary: false, location: None }
}

/// これより小さい応答は圧縮しない（gzip の枠の分だけかえって大きくなりうる）。
const MIN_COMPRESS_SIZE: usize = 256;

/// 圧縮して配信する種類。本番の Cloudflare と同じく、テキスト、JSON、JS、SVG、wasm などを圧縮する。
/// SVG 以外の画像、woff と woff2、音声、動画はすでに圧縮されているので除く。
fn compressible(content_type: &str) -> bool {
    let essence = content_type.split(';').next().unwrap_or("").trim();
    essence.starts_with("text/")
        || matches!(
            essence,
            "application/json"
                | "application/xml"
                | "application/wasm"
                | "application/vnd.sqlite3"
                | "image/svg+xml"
                | "font/ttf"
                | "font/otf"
        )
}

/// Accept-Encoding が gzip を受け付けるか。`gzip;q=0` は拒否として扱い、gzip の指定がなければ `*` に従う。
pub fn accepts_gzip(accept_encoding: &str) -> bool {
    let mut wildcard = None;
    for item in accept_encoding.split(',') {
        let mut parts = item.split(';');
        let coding = parts.next().unwrap_or("").trim();
        let quality = parts
            .find_map(|param| param.trim().strip_prefix("q="))
            .and_then(|q| q.trim().parse::<f32>().ok())
            .unwrap_or(1.0);
        if coding.eq_ignore_ascii_case("gzip") {
            return quality > 0.0;
        }
        if coding == "*" {
            wildcard = Some(quality > 0.0);
        }
    }
    wildcard.unwrap_or(false)
}

/// 圧縮した結果を中身ごとに覚えておく。配信する中身は起動後に変わらないので、同じものは一度だけ圧縮する。
#[derive(Default)]
pub struct GzipCache {
    entries: Mutex<HashMap<blake3::Hash, Arc<Vec<u8>>>>,
}

impl GzipCache {
    fn compress(&self, body: &[u8]) -> Arc<Vec<u8>> {
        let key = blake3::hash(body);
        if let Some(compressed) = self.entries.lock().unwrap().get(&key) {
            return Arc::clone(compressed);
        }
        let compressed = Arc::new(crate::gzip::compress(body));
        self.entries.lock().unwrap().insert(key, Arc::clone(&compressed));
        compressed
    }
}

/// 受け付けるなら、圧縮できる種類の応答を gzip で圧縮する。
pub fn encode(response: Response, accept_encoding: &str, cache: &GzipCache) -> Response {
    if response.status != 200 || !compressible(&response.content_type) {
        return response;
    }
    if response.body.len() < MIN_COMPRESS_SIZE || !accepts_gzip(accept_encoding) {
        return Response { vary: true, ..response };
    }
    let body = cache.compress(&response.body).as_ref().clone();
    Response { body, gzip: true, vary: true, ..response }
}

/// ブラウザに再読み込みを知らせる口（Server-Sent Events）のパス。サイトを置くパスの下に置く（ADR 0034）。
pub const EVENTS_PATH: &str = "__sqlite-cms/events";

/// 知らせがないときに、接続を保つために送る間隔。
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// 配信する中身と、自動反映の状態。記事の変更を反映するときは、中身を入れ替えて版を上げ、待っている接続に知らせる。
pub struct ServeState {
    output: RwLock<Arc<SiteOutput>>,
    version: Mutex<u64>,
    changed: Condvar,
    base_path: String,
    live_reload: bool,
}

impl ServeState {
    pub fn new(output: SiteOutput, base_path: String, live_reload: bool) -> Self {
        Self { output: RwLock::new(Arc::new(output)), version: Mutex::new(0), changed: Condvar::new(), base_path, live_reload }
    }

    fn current(&self) -> Arc<SiteOutput> {
        Arc::clone(&self.output.read().unwrap())
    }

    /// 中身を入れ替え、再読み込みを知らせる。
    pub fn replace(&self, output: SiteOutput) {
        *self.output.write().unwrap() = Arc::new(output);
        *self.version.lock().unwrap() += 1;
        self.changed.notify_all();
    }

    fn version(&self) -> u64 {
        *self.version.lock().unwrap()
    }

    /// 版が seen から変わるまで、最長 timeout 待つ。変わったら新しい版を返す。
    fn wait_for_change(&self, seen: u64, timeout: Duration) -> Option<u64> {
        let version = self.version.lock().unwrap();
        let (version, _) = self.changed.wait_timeout_while(version, timeout, |version| *version == seen).unwrap();
        (*version != seen).then_some(*version)
    }
}

/// 再読み込みの知らせを送り続ける。接続が切れたら終える。
fn stream_events(state: &ServeState, mut stream: TcpStream) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n: connected\n\n")?;
    stream.flush()?;
    let mut seen = state.version();
    loop {
        match state.wait_for_change(seen, KEEP_ALIVE) {
            Some(version) => {
                write!(stream, "event: reload\ndata: {version}\n\n")?;
                seen = version;
            }
            None => write!(stream, ": ping\n\n")?,
        }
        stream.flush()?;
    }
}

/// サイトを置くパス（ADR 0030）の下のパスを、サイトの中のパス（`/` から始まる）にする。外なら None。
fn strip_base_path(path: &str, base_path: &str) -> Option<String> {
    if base_path == "/" {
        return Some(path.to_string());
    }
    path.strip_prefix(base_path).map(|rest| format!("/{rest}")).or_else(|| (format!("{path}/") == base_path).then(|| "/".to_string()))
}

pub fn respond(output: &SiteOutput, base_path: &str, method: &str, target: &str) -> Response {
    if method != "GET" && method != "HEAD" {
        return text(405);
    }
    let path = percent_decode(target.split(['?', '#']).next().unwrap_or("/"));
    let Some(path) = strip_base_path(&path, base_path) else {
        // サイトの外のトップは、サイトのトップへ案内する。
        if path == "/" {
            return Response { location: Some(base_path.to_string()), ..text(302) };
        }
        return text(404);
    };
    let path = if path == "/" { "/index.html".to_string() } else { path };

    if let Some(bytes) = output.files.get(&path) {
        return ok(content_type(&path), bytes.clone());
    }
    if extension(&path).is_empty() {
        if let Some(index) = output.files.get("/index.html") {
            return ok(content_type("/index.html"), index.clone());
        }
    }
    text(404)
}

fn handle(state: &ServeState, cache: &GzipCache, stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut accept_encoding = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("accept-encoding") {
                accept_encoding = value.trim().to_string();
            }
        }
    }

    let mut parts = request_line.split_whitespace();
    let (method, target) = (parts.next(), parts.next());
    if state.live_reload
        && method == Some("GET")
        && target.and_then(|t| t.split('?').next()) == Some(format!("{}{EVENTS_PATH}", state.base_path).as_str())
    {
        return stream_events(state, stream);
    }
    let output = state.current();
    let mut head_length = 0;
    let response = match (method, target) {
        (Some(method), Some(target)) => {
            let response = encode(respond(&output, &state.base_path, method, target), &accept_encoding, cache);
            if method == "HEAD" {
                head_length = response.body.len();
                Response { body: Vec::new(), ..response }
            } else {
                response
            }
        }
        _ => text(400),
    };

    let mut stream = stream;
    // HEAD でも、GET と同じ（圧縮した）Content-Length を返す。
    let length = if response.body.is_empty() { head_length } else { response.body.len() };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\n{}{}{}Content-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        response.status,
        reason(response.status),
        response.content_type,
        response.location.as_ref().map_or_else(String::new, |location| format!("Location: {location}\r\n")),
        if response.gzip { "Content-Encoding: gzip\r\n" } else { "" },
        if response.vary { "Vary: Accept-Encoding\r\n" } else { "" },
        length
    )?;
    stream.write_all(&response.body)?;
    stream.flush()
}

pub fn serve(state: Arc<ServeState>, listener: TcpListener) {
    let cache = Arc::new(GzipCache::default());
    for stream in listener.incoming().flatten() {
        let state = Arc::clone(&state);
        let cache = Arc::clone(&cache);
        thread::spawn(move || {
            let _ = handle(&state, &cache, stream);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn output() -> SiteOutput {
        let mut files = BTreeMap::new();
        files.insert("/index.html".to_string(), b"<html>".to_vec());
        files.insert("/db/manifest.json".to_string(), b"{}".to_vec());
        files.insert("/media/画像.png".to_string(), b"png".to_vec());
        SiteOutput { files }
    }

    #[test]
    fn serves_files_with_content_type() {
        let response = respond(&output(), "/", "GET", "/db/manifest.json?v=1");
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json");
        assert_eq!(response.body, b"{}");
    }

    #[test]
    fn root_and_extensionless_paths_fall_back_to_index() {
        for target in ["/", "/posts/hello", "/about"] {
            let response = respond(&output(), "/", "GET", target);
            assert_eq!(response.status, 200, "{target}");
            assert_eq!(response.body, b"<html>", "{target}");
        }
    }

    #[test]
    fn decodes_percent_encoded_paths() {
        let response = respond(&output(), "/", "GET", "/media/%E7%94%BB%E5%83%8F.png");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"png");
    }

    #[test]
    fn missing_files_with_extension_are_not_found() {
        assert_eq!(respond(&output(), "/", "GET", "/assets/missing.js").status, 404);
        assert_eq!(respond(&output(), "/", "GET", "/../../etc/passwd.txt").status, 404);
    }

    #[test]
    fn only_get_and_head_are_allowed() {
        assert_eq!(respond(&output(), "/", "POST", "/").status, 405);
        assert_eq!(respond(&output(), "/", "HEAD", "/").status, 200);
    }

    fn large_js() -> Response {
        ok("text/javascript; charset=utf-8".into(), "console.log('こんにちは');\n".repeat(100).into_bytes())
    }

    #[test]
    fn compresses_compressible_responses_when_gzip_is_accepted() {
        let original = large_js().body;
        let response = encode(large_js(), "gzip, deflate, br", &GzipCache::default());
        assert!(response.gzip);
        assert!(response.vary);
        assert_eq!(response.body, crate::gzip::compress(&original));
        assert!(response.body.len() < original.len());
    }

    #[test]
    fn keeps_responses_as_is_when_gzip_is_not_accepted() {
        for accept_encoding in ["", "br", "gzip;q=0", "gzip;q=0, *", "identity"] {
            let response = encode(large_js(), accept_encoding, &GzipCache::default());
            assert!(!response.gzip, "{accept_encoding}");
            assert!(response.vary, "{accept_encoding}");
            assert_eq!(response.body, large_js().body, "{accept_encoding}");
        }
    }

    #[test]
    fn does_not_compress_small_or_already_compressed_or_error_responses() {
        let cache = GzipCache::default();
        let small = encode(ok("text/html; charset=utf-8".into(), b"<html>".to_vec()), "gzip", &cache);
        assert!(!small.gzip);
        let png = encode(ok("image/png".into(), vec![0; 1000]), "gzip", &cache);
        assert!(!png.gzip);
        assert!(!png.vary);
        let woff2 = encode(ok("font/woff2".into(), vec![0; 1000]), "gzip", &cache);
        assert!(!woff2.gzip);
        let not_found = encode(Response { body: vec![b'x'; 1000], ..text(404) }, "gzip", &cache);
        assert!(!not_found.gzip);
    }

    #[test]
    fn compressible_types() {
        for content_type in [
            "text/html; charset=utf-8",
            "text/css; charset=utf-8",
            "text/javascript; charset=utf-8",
            "application/json",
            "application/wasm",
            "application/vnd.sqlite3",
            "image/svg+xml",
        ] {
            assert!(compressible(content_type), "{content_type}");
        }
        for content_type in ["image/png", "image/jpeg", "font/woff2", "video/mp4", "application/zip"] {
            assert!(!compressible(content_type), "{content_type}");
        }
    }

    #[test]
    fn accept_encoding_is_parsed_case_insensitively_with_quality() {
        assert!(accepts_gzip("gzip"));
        assert!(accepts_gzip("GZIP"));
        assert!(accepts_gzip("br, gzip;q=0.5"));
        assert!(accepts_gzip("*"));
        assert!(!accepts_gzip("gzip;q=0"));
        assert!(!accepts_gzip("gzip; q=0.0, *"));
        assert!(!accepts_gzip("*;q=0"));
        assert!(!accepts_gzip("deflate"));
    }

    #[test]
    fn same_body_is_compressed_once() {
        let cache = GzipCache::default();
        encode(large_js(), "gzip", &cache);
        encode(large_js(), "gzip", &cache);
        assert_eq!(cache.entries.lock().unwrap().len(), 1);
    }

    #[test]
    fn serves_under_the_base_path() {
        let response = respond(&output(), "/blog/", "GET", "/blog/db/manifest.json");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"{}");
        for target in ["/blog/", "/blog", "/blog/posts/hello"] {
            assert_eq!(respond(&output(), "/blog/", "GET", target).body, b"<html>", "{target}");
        }
        let top = respond(&output(), "/blog/", "GET", "/");
        assert_eq!(top.status, 302);
        assert_eq!(top.location.as_deref(), Some("/blog/"));
        assert_eq!(respond(&output(), "/blog/", "GET", "/db/manifest.json").status, 404);
        assert_eq!(respond(&output(), "/blog/", "GET", "/blogger/").status, 404);
    }

    #[test]
    fn replacing_the_output_wakes_waiting_listeners() {
        let state = Arc::new(ServeState::new(output(), "/".into(), true));
        assert_eq!(state.wait_for_change(0, Duration::from_millis(10)), None);
        let waiter = {
            let state = Arc::clone(&state);
            thread::spawn(move || state.wait_for_change(0, Duration::from_secs(5)))
        };
        thread::sleep(Duration::from_millis(50));
        let mut next = output();
        next.files.insert("/db/manifest.json".into(), b"{\"db\":\"new\"}".to_vec());
        state.replace(next);
        assert_eq!(waiter.join().unwrap(), Some(1));
        assert_eq!(state.current().files["/db/manifest.json"], b"{\"db\":\"new\"}");
    }

    #[test]
    fn broken_percent_escapes_are_kept_as_is() {
        assert_eq!(percent_decode("/a%2"), "/a%2");
        assert_eq!(percent_decode("/a%zz"), "/a%zz");
        assert_eq!(percent_decode("/a%20b"), "/a b");
        assert_eq!(percent_decode("/a%画像"), "/a%画像");
    }
}
