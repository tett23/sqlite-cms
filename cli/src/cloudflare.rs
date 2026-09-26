use std::collections::BTreeMap;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const DEFAULT_API_BASE: &str = "https://api.cloudflare.com/client/v4";

const MAX_ATTEMPTS: u32 = 3;

pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

pub trait Http {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse>;
}

pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(120)))
            .build()
            .into();
        Self { agent }
    }
}

impl Http for UreqHttp {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse> {
        let mut builder = ureq::http::Request::builder().method(request.method).uri(&request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        let mut response = if request.body.is_empty() {
            self.agent.run(builder.body(())?)?
        } else {
            self.agent.run(builder.body(request.body.clone())?)?
        };
        let status = response.status().as_u16();
        let body = response.body_mut().read_to_vec()?;
        Ok(HttpResponse { status, body })
    }
}

#[derive(Debug, Serialize)]
pub struct ManifestEntry {
    pub hash: String,
    pub size: usize,
}

#[derive(Debug, Deserialize)]
pub struct UploadSession {
    pub jwt: String,
    #[serde(default)]
    pub buckets: Vec<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct UploadResult {
    jwt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AccountSubdomain {
    subdomain: String,
}

#[derive(Debug, Serialize)]
pub struct AssetConfig {
    pub not_found_handling: &'static str,
    #[serde(rename = "_headers", skip_serializing_if = "Option::is_none")]
    pub headers: Option<String>,
}

pub struct UploadFile<'a> {
    pub hash: &'a str,
    pub content_type: String,
    pub bytes: &'a [u8],
}

#[derive(Deserialize)]
struct Envelope<T> {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    errors: Vec<ApiMessage>,
    result: Option<T>,
}

#[derive(Deserialize)]
struct ApiMessage {
    code: Option<i64>,
    message: String,
}

pub struct Client<'a> {
    http: &'a dyn Http,
    api_base: String,
    account_id: String,
    api_token: String,
    retry_delay: Duration,
}

impl<'a> Client<'a> {
    pub fn new(http: &'a dyn Http, api_base: &str, account_id: &str, api_token: &str) -> Self {
        Self {
            http,
            api_base: api_base.trim_end_matches('/').to_string(),
            account_id: account_id.to_string(),
            api_token: api_token.to_string(),
            retry_delay: Duration::from_secs(1),
        }
    }

    #[cfg(test)]
    pub fn without_retry_delay(mut self) -> Self {
        self.retry_delay = Duration::ZERO;
        self
    }

    fn account_url(&self, path: &str) -> String {
        format!("{}/accounts/{}{path}", self.api_base, self.account_id)
    }

    fn api_auth(&self) -> (String, String) {
        ("Authorization".to_string(), format!("Bearer {}", self.api_token))
    }

    fn send_with_retry(&self, what: &str, request: &HttpRequest) -> Result<HttpResponse> {
        let mut attempt = 1;
        loop {
            let retryable = match self.http.send(request) {
                Ok(response) if response.status == 429 || response.status >= 500 => {
                    if attempt >= MAX_ATTEMPTS {
                        return Ok(response);
                    }
                    format!("HTTP {}", response.status)
                }
                Ok(response) => return Ok(response),
                Err(e) if attempt >= MAX_ATTEMPTS => return Err(e).with_context(|| format!("{what}: 通信に失敗しました")),
                Err(e) => format!("{e:#}"),
            };
            eprintln!("{what}: 失敗したので再試行します（{attempt}/{MAX_ATTEMPTS}、{retryable}）");
            thread::sleep(self.retry_delay * 2u32.pow(attempt - 1));
            attempt += 1;
        }
    }

    fn call<T: DeserializeOwned>(&self, what: &str, request: HttpRequest) -> Result<Option<T>> {
        let response = self.send_with_retry(what, &request)?;
        let envelope: Envelope<T> = serde_json::from_slice(&response.body)
            .with_context(|| format!("{what}: Cloudflare API の応答を解釈できません（HTTP {}）", response.status))?;
        if !envelope.success || !(200..300).contains(&response.status) {
            let detail = envelope
                .errors
                .iter()
                .map(|e| match e.code {
                    Some(code) => format!("[{code}] {}", e.message),
                    None => e.message.clone(),
                })
                .collect::<Vec<_>>()
                .join("; ");
            bail!("{what}: Cloudflare API がエラーを返しました（HTTP {}）: {detail}", response.status);
        }
        Ok(envelope.result)
    }

    fn call_required<T: DeserializeOwned>(&self, what: &str, request: HttpRequest) -> Result<T> {
        self.call(what, request)?.ok_or_else(|| anyhow!("{what}: Cloudflare API の応答に result がありません"))
    }

    pub fn start_upload_session(&self, worker: &str, manifest: &BTreeMap<String, ManifestEntry>) -> Result<UploadSession> {
        let body = serde_json::to_vec(&serde_json::json!({ "manifest": manifest }))?;
        self.call_required(
            "アップロードの開始",
            HttpRequest {
                method: "POST",
                url: self.account_url(&format!("/workers/scripts/{worker}/assets-upload-session")),
                headers: vec![self.api_auth(), ("Content-Type".into(), "application/json".into())],
                body,
            },
        )
    }

    pub fn upload_bucket(&self, upload_jwt: &str, files: &[UploadFile]) -> Result<Option<String>> {
        let parts: Vec<Part> = files
            .iter()
            .map(|f| Part {
                name: f.hash.to_string(),
                filename: Some(f.hash.to_string()),
                content_type: Some(f.content_type.clone()),
                data: crate::base64::encode(f.bytes).into_bytes(),
            })
            .collect();
        let (content_type, body) = multipart(&parts);
        let result: Option<UploadResult> = self.call(
            "アセットのアップロード",
            HttpRequest {
                method: "POST",
                url: self.account_url("/workers/assets/upload?base64=true"),
                headers: vec![
                    ("Authorization".into(), format!("Bearer {upload_jwt}")),
                    ("Content-Type".into(), content_type),
                ],
                body,
            },
        )?;
        Ok(result.unwrap_or_default().jwt)
    }

    pub fn upload_single(&self, upload_jwt: &str, file: &UploadFile) -> Result<Option<String>> {
        let result: Option<UploadResult> = self.call(
            "アセットのアップロード",
            HttpRequest {
                method: "POST",
                url: self.account_url(&format!("/workers/assets/upload/{}", file.hash)),
                headers: vec![
                    ("Authorization".into(), format!("Bearer {upload_jwt}")),
                    ("Content-Type".into(), file.content_type.clone()),
                ],
                body: file.bytes.to_vec(),
            },
        )?;
        Ok(result.unwrap_or_default().jwt)
    }

    pub fn put_assets_only_worker(
        &self,
        worker: &str,
        completion_jwt: &str,
        config: &AssetConfig,
        compatibility_date: &str,
    ) -> Result<()> {
        let metadata = serde_json::json!({
            "assets": { "jwt": completion_jwt, "config": config },
            "compatibility_date": compatibility_date,
        });
        let (content_type, body) = multipart(&[Part {
            name: "metadata".into(),
            filename: None,
            content_type: None,
            data: serde_json::to_vec(&metadata)?,
        }]);
        let _: Option<serde_json::Value> = self.call(
            "Worker の更新",
            HttpRequest {
                method: "PUT",
                url: self.account_url(&format!("/workers/scripts/{worker}")),
                headers: vec![self.api_auth(), ("Content-Type".into(), content_type)],
                body,
            },
        )?;
        Ok(())
    }

    pub fn enable_workers_dev(&self, worker: &str) -> Result<()> {
        let _: Option<serde_json::Value> = self.call(
            "workers.dev の有効化",
            HttpRequest {
                method: "POST",
                url: self.account_url(&format!("/workers/scripts/{worker}/subdomain")),
                headers: vec![
                    self.api_auth(),
                    ("Content-Type".into(), "application/json".into()),
                    ("Cloudflare-Workers-Script-Api-Date".into(), "2025-08-01".into()),
                ],
                body: br#"{"enabled":true,"previews_enabled":false}"#.to_vec(),
            },
        )?;
        Ok(())
    }

    pub fn account_subdomain(&self) -> Result<String> {
        let result: AccountSubdomain = self.call_required(
            "workers.dev サブドメインの取得",
            HttpRequest {
                method: "GET",
                url: self.account_url("/workers/subdomain"),
                headers: vec![self.api_auth()],
                body: Vec::new(),
            },
        )?;
        Ok(result.subdomain)
    }
}

pub fn is_single_asset_upload_mode(jwt: &str) -> bool {
    let Some(payload) = jwt.split('.').nth(1) else {
        return false;
    };
    let Some(bytes) = crate::base64::decode_url_safe(payload) else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|v| v.get("wrangler_single_asset_uploads").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}

struct Part {
    name: String,
    filename: Option<String>,
    content_type: Option<String>,
    data: Vec<u8>,
}

fn multipart(parts: &[Part]) -> (String, Vec<u8>) {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(&part.data);
    }
    let boundary = format!("sqlite-cms-{}", &hasher.finalize().to_hex()[..32]);

    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match &part.filename {
            Some(filename) => body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{}\"; filename=\"{filename}\"\r\n", part.name).as_bytes(),
            ),
            None => body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{}\"\r\n", part.name).as_bytes()),
        }
        if let Some(content_type) = &part.content_type {
            body.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
        }
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(&part.data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// (メソッド, URL, ヘッダ, 本文)
    pub type RecordedRequest = (String, String, Vec<(String, String)>, Vec<u8>);

    pub struct FakeHttp {
        pub requests: RefCell<Vec<RecordedRequest>>,
        responses: RefCell<VecDeque<Result<HttpResponse>>>,
    }

    impl FakeHttp {
        pub fn new(responses: Vec<(u16, &str)>) -> Self {
            Self {
                requests: RefCell::new(Vec::new()),
                responses: RefCell::new(
                    responses
                        .into_iter()
                        .map(|(status, body)| Ok(HttpResponse { status, body: body.as_bytes().to_vec() }))
                        .collect(),
                ),
            }
        }

        pub fn push_error(&self, message: &str) {
            self.responses.borrow_mut().push_front(Err(anyhow!(message.to_string())));
        }

        pub fn request_lines(&self) -> Vec<String> {
            self.requests.borrow().iter().map(|(m, u, _, _)| format!("{m} {u}")).collect()
        }
    }

    impl Http for FakeHttp {
        fn send(&self, request: &HttpRequest) -> Result<HttpResponse> {
            self.requests.borrow_mut().push((
                request.method.to_string(),
                request.url.clone(),
                request.headers.clone(),
                request.body.clone(),
            ));
            self.responses.borrow_mut().pop_front().expect("想定外のリクエスト")
        }
    }

    fn client(http: &FakeHttp) -> Client<'_> {
        Client::new(http, "https://api.test/client/v4/", "acct", "secret-token").without_retry_delay()
    }

    #[test]
    fn api_errors_are_reported_with_cloudflare_messages() {
        let http = FakeHttp::new(vec![(
            403,
            r#"{"success":false,"errors":[{"code":10000,"message":"Authentication error"}],"result":null}"#,
        )]);
        let err = client(&http).account_subdomain().unwrap_err().to_string();
        assert!(err.contains("HTTP 403"), "{err}");
        assert!(err.contains("[10000] Authentication error"), "{err}");
        assert!(!err.contains("secret-token"));
    }

    #[test]
    fn server_errors_are_retried() {
        let http = FakeHttp::new(vec![
            (502, "bad gateway"),
            (200, r#"{"success":true,"errors":[],"result":{"subdomain":"tett23"}}"#),
        ]);
        http.push_error("connection reset");
        assert_eq!(client(&http).account_subdomain().unwrap(), "tett23");
        assert_eq!(http.requests.borrow().len(), 3);
    }

    #[test]
    fn gives_up_after_max_attempts() {
        let http = FakeHttp::new(vec![(500, "x"), (500, "x"), (500, "x")]);
        assert!(client(&http).account_subdomain().is_err());
        assert_eq!(http.requests.borrow().len(), 3);
    }

    #[test]
    fn api_requests_use_token_and_account_url() {
        let http = FakeHttp::new(vec![(200, r#"{"success":true,"result":{"subdomain":"s"}}"#)]);
        client(&http).account_subdomain().unwrap();
        let requests = http.requests.borrow();
        assert_eq!(requests[0].1, "https://api.test/client/v4/accounts/acct/workers/subdomain");
        assert!(requests[0].2.contains(&("Authorization".into(), "Bearer secret-token".into())));
    }

    #[test]
    fn multipart_body_has_named_parts() {
        let (content_type, body) = multipart(&[
            Part { name: "abc".into(), filename: Some("abc".into()), content_type: Some("text/html".into()), data: b"PGh0bWw+".to_vec() },
            Part { name: "metadata".into(), filename: None, content_type: None, data: b"{}".to_vec() },
        ]);
        let boundary = content_type.strip_prefix("multipart/form-data; boundary=").unwrap();
        let body = String::from_utf8(body).unwrap();
        assert_eq!(
            body,
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"abc\"; filename=\"abc\"\r\nContent-Type: text/html\r\n\r\nPGh0bWw+\r\n\
                 --{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"\r\n\r\n{{}}\r\n--{boundary}--\r\n"
            )
        );
    }

    #[test]
    fn detects_single_asset_upload_mode_from_jwt() {
        let encode = |json: &str| crate::base64::encode_url_safe_no_pad(json.as_bytes());
        let single = format!("h.{}.s", encode(r#"{"wrangler_single_asset_uploads":true}"#));
        let bulk = format!("h.{}.s", encode(r#"{"exp":1}"#));
        assert!(is_single_asset_upload_mode(&single));
        assert!(!is_single_asset_upload_mode(&bulk));
        assert!(!is_single_asset_upload_mode("not-a-jwt"));
    }
}
