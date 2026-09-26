use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use crate::deploy::{content_type, extension};
use crate::output::SiteOutput;

pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
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

fn text(status: u16) -> Response {
    Response { status, content_type: "text/plain; charset=utf-8".into(), body: reason(status).as_bytes().to_vec() }
}

pub fn respond(output: &SiteOutput, method: &str, target: &str) -> Response {
    if method != "GET" && method != "HEAD" {
        return text(405);
    }
    let path = percent_decode(target.split(['?', '#']).next().unwrap_or("/"));
    let path = if path == "/" { "/index.html".to_string() } else { path };

    if let Some(bytes) = output.files.get(&path) {
        return Response { status: 200, content_type: content_type(&path), body: bytes.clone() };
    }
    if extension(&path).is_empty() {
        if let Some(index) = output.files.get("/index.html") {
            return Response { status: 200, content_type: content_type("/index.html"), body: index.clone() };
        }
    }
    text(404)
}

fn handle(output: &SiteOutput, stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }

    let mut parts = request_line.split_whitespace();
    let response = match (parts.next(), parts.next()) {
        (Some(method), Some(target)) => {
            let response = respond(output, method, target);
            if method == "HEAD" {
                Response { body: Vec::new(), ..response }
            } else {
                response
            }
        }
        _ => text(400),
    };

    let mut stream = stream;
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        response.status,
        reason(response.status),
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)?;
    stream.flush()
}

pub fn serve(output: SiteOutput, listener: TcpListener) {
    let output = Arc::new(output);
    for stream in listener.incoming().flatten() {
        let output = Arc::clone(&output);
        thread::spawn(move || {
            let _ = handle(&output, stream);
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
        let response = respond(&output(), "GET", "/db/manifest.json?v=1");
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json");
        assert_eq!(response.body, b"{}");
    }

    #[test]
    fn root_and_extensionless_paths_fall_back_to_index() {
        for target in ["/", "/posts/hello", "/about"] {
            let response = respond(&output(), "GET", target);
            assert_eq!(response.status, 200, "{target}");
            assert_eq!(response.body, b"<html>", "{target}");
        }
    }

    #[test]
    fn decodes_percent_encoded_paths() {
        let response = respond(&output(), "GET", "/media/%E7%94%BB%E5%83%8F.png");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"png");
    }

    #[test]
    fn missing_files_with_extension_are_not_found() {
        assert_eq!(respond(&output(), "GET", "/assets/missing.js").status, 404);
        assert_eq!(respond(&output(), "GET", "/../../etc/passwd.txt").status, 404);
    }

    #[test]
    fn only_get_and_head_are_allowed() {
        assert_eq!(respond(&output(), "POST", "/").status, 405);
        assert_eq!(respond(&output(), "HEAD", "/").status, 200);
    }

    #[test]
    fn broken_percent_escapes_are_kept_as_is() {
        assert_eq!(percent_decode("/a%2"), "/a%2");
        assert_eq!(percent_decode("/a%zz"), "/a%zz");
        assert_eq!(percent_decode("/a%20b"), "/a b");
        assert_eq!(percent_decode("/a%画像"), "/a%画像");
    }
}
