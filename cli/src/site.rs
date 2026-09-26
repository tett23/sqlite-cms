use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::date::{parse_timezone, TimeZone};

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteConfig {
    pub title: String,
    pub author: Option<String>,
    pub license: Option<License>,
    pub deploy: Option<DeployConfig>,
    pub timezone: Option<String>,
    pub description: Option<String>,
    /// サイトを置くパス（ADR 0030）。GitHub Pages のプロジェクトのページなら "/<リポジトリ名>/"。
    pub base_path: Option<String>,
    /// 公開したサイトの URL（ADR 0033）。書くと RSS のフィードを作る。
    pub url: Option<String>,
}

impl SiteConfig {
    /// meta description。省略したときはサイト名から組み立てる。
    pub fn description(&self) -> String {
        self.description
            .clone()
            .unwrap_or_else(|| format!("{}。記事とブログを置いているサイトです。", self.title))
    }

    /// `timezone` の指定。書式は読み込み時に検査済み。
    pub fn timezone(&self) -> Option<TimeZone> {
        self.timezone.as_deref().and_then(|s| parse_timezone(s).ok())
    }

    /// サイトを置くパス。`/` で始まり `/` で終わる形にそろえる。省略すると `/`。書式は読み込み時に検査済み。
    pub fn base_path(&self) -> String {
        self.base_path.as_deref().map_or_else(|| "/".to_string(), normalize_base_path)
    }

    /// 公開したサイトの URL。`/` で終わる形にそろえる。書式は読み込み時に検査済み。
    pub fn url(&self) -> Option<String> {
        self.url.as_deref().map(|url| if url.ends_with('/') { url.to_string() } else { format!("{url}/") })
    }

    /// 公開先。`[deploy]` がなければ None。書式は読み込み時に検査済み。
    pub fn deploy_target(&self) -> Option<DeployTarget> {
        self.deploy.as_ref().map(|deploy| deploy.target().expect("読み込み時に検査済み"))
    }
}

fn normalize_base_path(path: &str) -> String {
    if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    }
}

/// `base_path` の書式を確かめる。`/` で始まり、英数字と `-`、`_`、`.`、`~`、`/` だけを使え、`//`（別のホストと取り違える）や `.`、`..` の区切りを含まない。
fn validate_base_path(path: &str) -> Result<()> {
    let valid = path.starts_with('/')
        && !path.contains("//")
        && path.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.~/".contains(&b))
        && path.trim_matches('/').split('/').all(|segment| path == "/" || !matches!(segment, "" | "." | ".."));
    if !valid {
        bail!("site.toml の base_path は / で始まるパス（\"/my-blog/\" など）で指定してください: {path}");
    }
    Ok(())
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct License {
    pub name: String,
    pub url: Option<String>,
}

/// `[deploy]` の書かれたまま。公開先ごとの項目をまとめて持ち、`target()` で確かめて DeployTarget にする。
#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployConfig {
    /// "cloudflare"（省略したとき）、"github-pages"、"rsync"。
    pub target: Option<String>,
    pub worker: Option<String>,
    pub branch: Option<String>,
    pub remote: Option<String>,
    pub cname: Option<String>,
    pub destination: Option<String>,
}

/// 公開先（ADR 0030）。
#[derive(Debug, PartialEq)]
pub enum DeployTarget {
    Cloudflare { worker: String },
    GitHubPages { remote: String, branch: String, cname: Option<String> },
    Rsync { destination: String },
}

pub const DEFAULT_PAGES_BRANCH: &str = "gh-pages";
pub const DEFAULT_PAGES_REMOTE: &str = "origin";

impl DeployConfig {
    pub fn target(&self) -> Result<DeployTarget> {
        let target = self.target.as_deref().unwrap_or("cloudflare");
        let allowed: &[&str] = match target {
            "cloudflare" => &["worker"],
            "github-pages" => &["branch", "remote", "cname"],
            "rsync" => &["destination"],
            other => bail!("site.toml の deploy.target は \"cloudflare\"、\"github-pages\"、\"rsync\" のどれかにしてください: {other}"),
        };
        let written = [
            ("worker", self.worker.is_some()),
            ("branch", self.branch.is_some()),
            ("remote", self.remote.is_some()),
            ("cname", self.cname.is_some()),
            ("destination", self.destination.is_some()),
        ];
        if let Some((key, _)) = written.iter().find(|(key, present)| *present && !allowed.contains(key)) {
            bail!("site.toml の deploy.{key} は、deploy.target が {target} のときには使えません");
        }
        match target {
            "cloudflare" => {
                let worker = self.worker.clone().ok_or_else(|| {
                    anyhow::anyhow!("site.toml の [deploy] に worker = \"Worker 名\" を書いてください（Cloudflare に公開するとき）")
                })?;
                if !is_valid_worker_name(&worker) {
                    bail!("site.toml の deploy.worker は英小文字、数字、ハイフンの 63 文字以内で指定してください: {worker}");
                }
                Ok(DeployTarget::Cloudflare { worker })
            }
            "github-pages" => {
                let branch = self.branch.clone().unwrap_or_else(|| DEFAULT_PAGES_BRANCH.to_string());
                let remote = self.remote.clone().unwrap_or_else(|| DEFAULT_PAGES_REMOTE.to_string());
                if !is_safe_argument(&branch) || !branch.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_./".contains(&b)) {
                    bail!("site.toml の deploy.branch が不正です: {branch}");
                }
                if !is_safe_argument(&remote) {
                    bail!("site.toml の deploy.remote が不正です: {remote}");
                }
                if let Some(cname) = &self.cname {
                    if cname.is_empty() || !cname.bytes().all(|b| b.is_ascii_alphanumeric() || b"-.".contains(&b)) || cname.starts_with(['-', '.']) {
                        bail!("site.toml の deploy.cname はドメイン名（blog.example.com など）で指定してください: {cname}");
                    }
                }
                Ok(DeployTarget::GitHubPages { remote, branch, cname: self.cname.clone() })
            }
            _ => {
                let destination = self.destination.clone().ok_or_else(|| {
                    anyhow::anyhow!("site.toml の [deploy] に destination = \"user@host:/path/\" を書いてください（rsync で公開するとき）")
                })?;
                if !is_safe_argument(&destination) {
                    bail!("site.toml の deploy.destination が不正です（空、- で始まる、空白や改行を含むものは使えません）: {destination}");
                }
                Ok(DeployTarget::Rsync { destination })
            }
        }
    }
}

/// `url` の書式を確かめる。http か https で、ホストがあり、パスが `base_path` と同じであること。
fn validate_url(url: &str, base_path: &str) -> Result<()> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest.filter(|rest| !rest.starts_with('/') && !rest.contains(['?', '#', ' '])) else {
        bail!("site.toml の url は https:// で始まるサイトの URL（\"https://example.com/\" など）で指定してください: {url}");
    };
    let path = rest.find('/').map_or("/", |i| &rest[i..]);
    if path != base_path {
        bail!("site.toml の url のパス（{path}）が base_path（{base_path}）と違います。サイトを置く URL をそろえてください");
    }
    Ok(())
}

/// コマンドの引数として渡しても、オプションと取り違えられず、空白や改行を含まない値。
fn is_safe_argument(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('-') && !value.chars().any(char::is_whitespace)
}

fn is_valid_worker_name(name: &str) -> bool {
    (1..=63).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

pub fn read_site_config(site_dir: &Path) -> Result<SiteConfig> {
    let path = site_dir.join("site.toml");
    let raw = fs::read_to_string(&path).with_context(|| format!("{} を読めません", path.display()))?;
    parse_site_config(&raw)
}

pub fn parse_site_config(raw: &str) -> Result<SiteConfig> {
    let config: SiteConfig = toml::from_str(raw).context("site.toml を解釈できません")?;
    if config.title.trim().is_empty() {
        bail!("site.toml の title が空です");
    }
    if let Some(license) = &config.license {
        if license.name.trim().is_empty() {
            bail!("site.toml の license.name が空です");
        }
        if let Some(url) = &license.url {
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                bail!("site.toml の license.url は http:// か https:// で始めてください: {url}");
            }
        }
    }
    if config.description.as_ref().is_some_and(|d| d.trim().is_empty()) {
        bail!("site.toml の description が空です。書かないか、説明を書いてください");
    }
    if let Some(timezone) = &config.timezone {
        parse_timezone(timezone).map_err(|e| anyhow::anyhow!("site.toml の timezone が不正です: {e}"))?;
    }
    if let Some(deploy) = &config.deploy {
        deploy.target()?;
    }
    if let Some(base_path) = &config.base_path {
        validate_base_path(base_path)?;
    }
    if let Some(url) = config.url() {
        validate_url(&url, &config.base_path())?;
    }
    if matches!(config.deploy_target(), Some(DeployTarget::Cloudflare { .. })) && config.base_path() != "/" {
        bail!("Cloudflare に公開するときは base_path を使えません（Worker はドメインの直下で配信します）");
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_title_and_author() {
        let config = parse_site_config("title = \"記事置き場\"\nauthor = \"tett23\"\n").unwrap();
        assert_eq!(
            config,
            SiteConfig {
                title: "記事置き場".into(),
                author: Some("tett23".into()),
                license: None,
                deploy: None,
                timezone: None,
                description: None,
                base_path: None,
                url: None,
            }
        );
    }

    #[test]
    fn author_is_optional() {
        assert_eq!(parse_site_config("title = \"t\"").unwrap().author, None);
    }

    #[test]
    fn reads_license_with_url() {
        let config = parse_site_config(
            "title = \"t\"\n[license]\nname = \"CC0 1.0\"\nurl = \"https://creativecommons.org/publicdomain/zero/1.0/\"\n",
        )
        .unwrap();
        assert_eq!(
            config.license,
            Some(License {
                name: "CC0 1.0".into(),
                url: Some("https://creativecommons.org/publicdomain/zero/1.0/".into()),
            })
        );
    }

    #[test]
    fn license_url_is_optional() {
        let config = parse_site_config("title = \"t\"\n[license]\nname = \"CC0\"\n").unwrap();
        assert_eq!(config.license.unwrap().url, None);
    }

    #[test]
    fn license_without_name_is_error() {
        let err = parse_site_config("title = \"t\"\n[license]\nurl = \"https://example.com\"\n").unwrap_err();
        assert!(format!("{err:#}").contains("name"));
    }

    #[test]
    fn license_url_must_be_http() {
        let err =
            parse_site_config("title = \"t\"\n[license]\nname = \"x\"\nurl = \"javascript:alert(1)\"\n").unwrap_err();
        assert!(err.to_string().contains("license.url"));
    }

    #[test]
    fn description_defaults_to_a_sentence_with_the_title() {
        let config = parse_site_config("title = \"記事置き場\"").unwrap();
        assert_eq!(config.description(), "記事置き場。記事とブログを置いているサイトです。");
        let config = parse_site_config("title = \"t\"\ndescription = \"組版の記事\"").unwrap();
        assert_eq!(config.description(), "組版の記事");
    }

    #[test]
    fn empty_description_is_error() {
        let err = parse_site_config("title = \"t\"\ndescription = \" \"").unwrap_err();
        assert!(err.to_string().contains("description が空"));
    }

    #[test]
    fn reads_timezone() {
        let config = parse_site_config("title = \"t\"\ntimezone = \"Asia/Tokyo\"\n").unwrap();
        assert_eq!(config.timezone(), Some(TimeZone::Named("Asia/Tokyo".into())));
        let config = parse_site_config("title = \"t\"\ntimezone = \"+09:00\"\n").unwrap();
        assert_eq!(config.timezone(), Some(TimeZone::Offset(540)));
        assert_eq!(parse_site_config("title = \"t\"").unwrap().timezone(), None);
    }

    #[test]
    fn invalid_timezone_is_error() {
        let err = parse_site_config("title = \"t\"\ntimezone = \"+9\"\n").unwrap_err();
        assert!(err.to_string().contains("timezone が不正です"), "{err}");
    }

    #[test]
    fn reads_deploy_worker() {
        let config = parse_site_config("title = \"t\"\n[deploy]\nworker = \"tett23-blog\"\n").unwrap();
        assert_eq!(config.deploy_target(), Some(DeployTarget::Cloudflare { worker: "tett23-blog".into() }));
    }

    #[test]
    fn invalid_worker_name_is_error() {
        for name in ["Blog", "blog/../x", "-blog", "blog-", ""] {
            let raw = format!("title = \"t\"\n[deploy]\nworker = \"{name}\"\n");
            let err = parse_site_config(&raw).unwrap_err();
            assert!(err.to_string().contains("deploy.worker") || err.to_string().contains("worker"), "{name}");
        }
        let long = "a".repeat(64);
        assert!(parse_site_config(&format!("title = \"t\"\n[deploy]\nworker = \"{long}\"\n")).is_err());
    }

    fn target(deploy: &str) -> Result<Option<DeployTarget>> {
        parse_site_config(&format!("title = \"t\"\n[deploy]\n{deploy}")).map(|c| c.deploy_target())
    }

    #[test]
    fn reads_each_deploy_target() {
        assert_eq!(
            target("target = \"cloudflare\"\nworker = \"blog\"\n").unwrap(),
            Some(DeployTarget::Cloudflare { worker: "blog".into() })
        );
        assert_eq!(
            target("target = \"github-pages\"\n").unwrap(),
            Some(DeployTarget::GitHubPages { remote: "origin".into(), branch: "gh-pages".into(), cname: None })
        );
        assert_eq!(
            target("target = \"github-pages\"\nremote = \"upstream\"\nbranch = \"pages\"\ncname = \"blog.example.com\"\n").unwrap(),
            Some(DeployTarget::GitHubPages { remote: "upstream".into(), branch: "pages".into(), cname: Some("blog.example.com".into()) })
        );
        assert_eq!(
            target("target = \"rsync\"\ndestination = \"user@example.com:/var/www/blog/\"\n").unwrap(),
            Some(DeployTarget::Rsync { destination: "user@example.com:/var/www/blog/".into() })
        );
        assert_eq!(parse_site_config("title = \"t\"\n").unwrap().deploy_target(), None);
    }

    #[test]
    fn deploy_target_errors_name_the_problem() {
        let message = |deploy: &str| target(deploy).unwrap_err().to_string();
        assert!(message("target = \"netlify\"\n").contains("deploy.target"));
        assert!(message("\n").contains("worker"));
        assert!(message("target = \"rsync\"\n").contains("destination"));
        assert!(message("target = \"rsync\"\ndestination = \"x:/a\"\nworker = \"w\"\n").contains("deploy.worker は、deploy.target が rsync"));
        assert!(message("target = \"github-pages\"\nworker = \"w\"\n").contains("deploy.worker"));
        assert!(message("target = \"rsync\"\ndestination = \"--rsh=evil\"\n").contains("deploy.destination"));
        assert!(message("target = \"rsync\"\ndestination = \"a b:/c\"\n").contains("deploy.destination"));
        assert!(message("target = \"github-pages\"\nbranch = \"-f\"\n").contains("deploy.branch"));
        assert!(message("target = \"github-pages\"\nbranch = \"a b\"\n").contains("deploy.branch"));
        assert!(message("target = \"github-pages\"\nremote = \"--upload-pack=x\"\n").contains("deploy.remote"));
        assert!(message("target = \"github-pages\"\ncname = \"https://x\"\n").contains("deploy.cname"));
    }

    #[test]
    fn url_is_normalized_and_must_match_the_base_path() {
        let url = |extra: &str| parse_site_config(&format!("title = \"t\"\n{extra}")).map(|c| c.url());
        assert_eq!(url("").unwrap(), None);
        assert_eq!(url("url = \"https://example.com\"\n").unwrap().as_deref(), Some("https://example.com/"));
        assert_eq!(url("url = \"http://example.com/\"\n").unwrap().as_deref(), Some("http://example.com/"));
        assert_eq!(
            url("base_path = \"/blog/\"\nurl = \"https://user.github.io/blog\"\n").unwrap().as_deref(),
            Some("https://user.github.io/blog/")
        );
        assert!(url("url = \"https://example.com/blog/\"\n").unwrap_err().to_string().contains("base_path"));
        for invalid in ["example.com", "ftp://example.com/", "https:///path", "https://example.com/?q=1", "https://a b/"] {
            assert!(url(&format!("url = \"{invalid}\"\n")).is_err(), "{invalid}");
        }
    }

    #[test]
    fn cloudflare_does_not_accept_a_base_path() {
        let err = parse_site_config("title = \"t\"\nbase_path = \"/blog/\"\n[deploy]\nworker = \"w\"\n").unwrap_err();
        assert!(err.to_string().contains("base_path"));
        assert!(parse_site_config("title = \"t\"\nbase_path = \"/\"\n[deploy]\nworker = \"w\"\n").is_ok());
    }

    #[test]
    fn base_path_is_normalized_and_validated() {
        let base = |raw: &str| parse_site_config(&format!("title = \"t\"\nbase_path = \"{raw}\"\n")).map(|c| c.base_path());
        assert_eq!(parse_site_config("title = \"t\"\n").unwrap().base_path(), "/");
        assert_eq!(base("/").unwrap(), "/");
        assert_eq!(base("/my-blog").unwrap(), "/my-blog/");
        assert_eq!(base("/my-blog/").unwrap(), "/my-blog/");
        assert_eq!(base("/a/b.c_d~e/").unwrap(), "/a/b.c_d~e/");
        for invalid in ["my-blog", "", "/a//b/", "/a/../b/", "/./", "/a b/", "/日本語/", "/a?b/", "//cdn"] {
            assert!(base(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn missing_title_is_error() {
        let err = parse_site_config("author = \"a\"").unwrap_err();
        assert!(format!("{err:#}").contains("title"));
    }

    #[test]
    fn empty_title_is_error() {
        let err = parse_site_config("title = \"  \"").unwrap_err();
        assert!(err.to_string().contains("title が空"));
    }

    #[test]
    fn unknown_key_is_error() {
        let err = parse_site_config("title = \"t\"\ntitel = \"typo\"").unwrap_err();
        assert!(format!("{err:#}").contains("titel"));
    }

    #[test]
    fn malformed_toml_is_error() {
        let err = parse_site_config("title = ").unwrap_err();
        assert!(err.to_string().contains("site.toml を解釈できません"));
    }
}
