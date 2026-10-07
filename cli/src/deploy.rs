use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, bail, Result};

use crate::cloudflare::{self, AssetConfig, Client, ManifestEntry, UploadFile};
use crate::output::{SiteOutput, HEADERS};

pub const COMPATIBILITY_DATE: &str = "2026-09-01";

const MAX_ASSET_SIZE: usize = 25 * 1024 * 1024;

pub struct DeployReport {
    pub total: usize,
    pub uploaded: usize,
    pub url: Option<String>,
}

pub(crate) fn extension(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rfind('.') {
        Some(0) | None => "",
        Some(i) => &name[i + 1..],
    }
}

pub fn asset_hash(path: &str, bytes: &[u8]) -> String {
    let encoded = crate::base64::encode(bytes);
    let digest = blake3::hash(format!("{encoded}{}", extension(path)).as_bytes());
    digest.to_hex()[..32].to_string()
}

pub fn content_type(path: &str) -> String {
    crate::mime::by_extension(extension(path)).unwrap_or("application/null").to_string()
}

pub fn deploy(client: &Client, worker: &str, output: &SiteOutput) -> Result<DeployReport> {
    let mut manifest = BTreeMap::new();
    let mut path_by_hash: HashMap<String, &str> = HashMap::new();
    for (path, bytes) in &output.files {
        if bytes.len() > MAX_ASSET_SIZE {
            bail!("{path} は {} バイトあり、Cloudflare の上限（25 MiB）を超えています", bytes.len());
        }
        let hash = asset_hash(path, bytes);
        path_by_hash.entry(hash.clone()).or_insert(path);
        manifest.insert(path.clone(), ManifestEntry { hash, size: bytes.len() });
    }

    let session = client.start_upload_session(worker, &manifest)?;
    let buckets = session
        .buckets
        .iter()
        .map(|bucket| {
            bucket
                .iter()
                .map(|hash| {
                    let path = path_by_hash
                        .get(hash)
                        .ok_or_else(|| anyhow!("Cloudflare がマニフェストにないファイル（{hash}）を要求しました"))?;
                    Ok(UploadFile { hash, content_type: content_type(path), bytes: &output.files[*path] })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let uploaded = buckets.iter().map(Vec::len).sum();

    let mut completion_jwt = None;
    if buckets.is_empty() {
        completion_jwt = Some(session.jwt.clone());
    } else if cloudflare::is_single_asset_upload_mode(&session.jwt) {
        for file in buckets.iter().flatten() {
            completion_jwt = client.upload_single(&session.jwt, file)?.or(completion_jwt);
        }
    } else {
        for bucket in &buckets {
            completion_jwt = client.upload_bucket(&session.jwt, bucket)?.or(completion_jwt);
        }
    }
    let completion_jwt =
        completion_jwt.ok_or_else(|| anyhow!("アセットのアップロードが完了しませんでした（完了トークンが返されませんでした）"))?;

    let config = AssetConfig { not_found_handling: "single-page-application", headers: Some(HEADERS.to_string()) };
    client.put_assets_only_worker(worker, &completion_jwt, &config, COMPATIBILITY_DATE)?;

    if let Err(e) = client.enable_workers_dev(worker) {
        eprintln!("警告: workers.dev の URL を有効にできませんでした: {e:#}");
    }
    let url = match client.account_subdomain() {
        Ok(subdomain) => Some(format!("https://{worker}.{subdomain}.workers.dev")),
        Err(e) => {
            eprintln!("警告: workers.dev のサブドメインを取得できませんでした: {e:#}");
            None
        }
    };

    Ok(DeployReport { total: manifest.len(), uploaded, url })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloudflare::tests::FakeHttp;

    fn output() -> SiteOutput {
        let mut files = BTreeMap::new();
        files.insert("/index.html".to_string(), b"<html>".to_vec());
        files.insert("/db/manifest.json".to_string(), b"{}".to_vec());
        SiteOutput { files }
    }

    fn client(http: &FakeHttp) -> Client<'_> {
        Client::new(http, "https://api.test/client/v4", "acct", "token").without_retry_delay()
    }

    fn ok(result: &str) -> String {
        format!(r#"{{"success":true,"errors":[],"result":{result}}}"#)
    }

    fn jwt_with(payload: &str) -> String {
        format!("h.{}.s", crate::base64::encode_url_safe_no_pad(payload.as_bytes()))
    }

    fn body_of(http: &FakeHttp, index: usize) -> String {
        String::from_utf8(http.requests.borrow()[index].3.clone()).unwrap()
    }

    #[test]
    fn hash_follows_wrangler_scheme() {
        let expected = blake3::hash(b"PGh0bWw+html").to_hex()[..32].to_string();
        assert_eq!(asset_hash("/index.html", b"<html>"), expected);
        assert_eq!(asset_hash("/index.html", b"<html>").len(), 32);
    }

    #[test]
    fn extension_matches_node_path_extname() {
        assert_eq!(extension("/assets/app.min.js"), "js");
        assert_eq!(extension("/.hidden"), "");
        assert_eq!(extension("/LICENSE"), "");
        assert_eq!(extension("/dir.d/file"), "");
    }

    #[test]
    fn content_types() {
        assert_eq!(content_type("/index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("/assets/sql-wasm.wasm"), "application/wasm");
        assert_eq!(content_type("/media/a.svg"), "image/svg+xml");
        assert_eq!(content_type("/db/articles-0.sqlite"), "application/vnd.sqlite3");
        assert_eq!(content_type("/unknown.zzz"), "application/null");
    }

    #[test]
    fn uploads_requested_buckets_and_updates_worker() {
        let out = output();
        let index_hash = asset_hash("/index.html", b"<html>");
        let manifest_hash = asset_hash("/db/manifest.json", b"{}");
        let session = ok(&format!(r#"{{"jwt":"upload-jwt","buckets":[["{index_hash}"],["{manifest_hash}"]]}}"#));
        let http = FakeHttp::new(vec![
            (200, &session),
            (202, &ok("{}")),
            (201, &ok(r#"{"jwt":"completion-jwt"}"#)),
            (200, &ok("{}")),
            (200, &ok("{}")),
            (200, &ok(r#"{"subdomain":"tett23"}"#)),
        ]);

        let report = deploy(&client(&http), "blog", &out).unwrap();

        assert_eq!(
            http.request_lines(),
            [
                "POST https://api.test/client/v4/accounts/acct/workers/scripts/blog/assets-upload-session",
                "POST https://api.test/client/v4/accounts/acct/workers/assets/upload?base64=true",
                "POST https://api.test/client/v4/accounts/acct/workers/assets/upload?base64=true",
                "PUT https://api.test/client/v4/accounts/acct/workers/scripts/blog",
                "POST https://api.test/client/v4/accounts/acct/workers/scripts/blog/subdomain",
                "GET https://api.test/client/v4/accounts/acct/workers/subdomain",
            ]
        );

        let session_body: serde_json::Value = serde_json::from_str(&body_of(&http, 0)).unwrap();
        assert_eq!(session_body["manifest"]["/index.html"]["hash"], index_hash.as_str());
        assert_eq!(session_body["manifest"]["/index.html"]["size"], 6);

        let upload = body_of(&http, 1);
        assert!(upload.contains(&format!("name=\"{index_hash}\"; filename=\"{index_hash}\"")));
        assert!(upload.contains("Content-Type: text/html; charset=utf-8"));
        assert!(upload.contains("PGh0bWw+"));
        assert!(http.requests.borrow()[1].2.contains(&("Authorization".into(), "Bearer upload-jwt".into())));

        let put = body_of(&http, 3);
        assert!(put.contains(r#""jwt":"completion-jwt""#));
        assert!(put.contains(r#""not_found_handling":"single-page-application""#));
        assert!(put.contains(r#""_headers":"/assets/*\n  Access-Control-Allow-Origin: *\n\n/db/*.sqlite\n"#), "{put}");
        assert!(put.contains(&format!(r#""compatibility_date":"{COMPATIBILITY_DATE}""#)));

        assert_eq!(report.total, 2);
        assert_eq!(report.uploaded, 2);
        assert_eq!(report.url.as_deref(), Some("https://blog.tett23.workers.dev"));
    }

    #[test]
    fn skips_upload_when_everything_is_already_uploaded() {
        let http = FakeHttp::new(vec![
            (200, &ok(r#"{"jwt":"already-complete","buckets":[]}"#)),
            (200, &ok("{}")),
            (200, &ok("{}")),
            (200, &ok(r#"{"subdomain":"s"}"#)),
        ]);
        let report = deploy(&client(&http), "blog", &output()).unwrap();
        assert_eq!(report.uploaded, 0);
        assert!(http.request_lines()[1].starts_with("PUT "));
        assert!(body_of(&http, 1).contains(r#""jwt":"already-complete""#));
    }

    #[test]
    fn uploads_files_one_by_one_in_single_asset_mode() {
        let out = output();
        let index_hash = asset_hash("/index.html", b"<html>");
        let upload_jwt = jwt_with(r#"{"wrangler_single_asset_uploads":true}"#);
        let session = ok(&format!(r#"{{"jwt":"{upload_jwt}","buckets":[["{index_hash}"]]}}"#));
        let http = FakeHttp::new(vec![
            (200, &session),
            (201, &ok(r#"{"jwt":"completion-jwt"}"#)),
            (200, &ok("{}")),
            (200, &ok("{}")),
            (200, &ok(r#"{"subdomain":"s"}"#)),
        ]);

        deploy(&client(&http), "blog", &out).unwrap();

        assert_eq!(
            http.request_lines()[1],
            format!("POST https://api.test/client/v4/accounts/acct/workers/assets/upload/{index_hash}")
        );
        assert_eq!(http.requests.borrow()[1].3, b"<html>");
    }

    #[test]
    fn unknown_hash_in_bucket_is_error() {
        let http = FakeHttp::new(vec![(200, &ok(r#"{"jwt":"j","buckets":[["ffff"]]}"#))]);
        let err = deploy(&client(&http), "blog", &output()).err().unwrap();
        assert!(err.to_string().contains("マニフェストにないファイル"));
    }

    #[test]
    fn missing_completion_token_is_error() {
        let index_hash = asset_hash("/index.html", b"<html>");
        let session = ok(&format!(r#"{{"jwt":"j","buckets":[["{index_hash}"]]}}"#));
        let http = FakeHttp::new(vec![(200, &session), (202, &ok("{}"))]);
        let err = deploy(&client(&http), "blog", &output()).err().unwrap();
        assert!(err.to_string().contains("完了トークン"));
    }

    #[test]
    fn oversized_file_is_rejected_before_any_request() {
        let mut out = output();
        out.files.insert("/media/huge.bin".to_string(), vec![0; MAX_ASSET_SIZE + 1]);
        let http = FakeHttp::new(vec![]);
        let err = deploy(&client(&http), "blog", &out).err().unwrap();
        assert!(err.to_string().contains("/media/huge.bin"));
        assert!(http.requests.borrow().is_empty());
    }

    #[test]
    fn workers_dev_failures_are_warnings() {
        let http = FakeHttp::new(vec![
            (200, &ok(r#"{"jwt":"done","buckets":[]}"#)),
            (200, &ok("{}")),
            (400, r#"{"success":false,"errors":[{"code":1,"message":"no subdomain"}],"result":null}"#),
            (404, r#"{"success":false,"errors":[{"code":2,"message":"not found"}],"result":null}"#),
        ]);
        let report = deploy(&client(&http), "blog", &output()).unwrap();
        assert_eq!(report.url, None);
    }
}
