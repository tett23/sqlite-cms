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
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct License {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployConfig {
    pub worker: String,
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
        if !is_valid_worker_name(&deploy.worker) {
            bail!(
                "site.toml の deploy.worker は英小文字、数字、ハイフンの 63 文字以内で指定してください: {}",
                deploy.worker
            );
        }
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
        assert_eq!(config.deploy, Some(DeployConfig { worker: "tett23-blog".into() }));
    }

    #[test]
    fn invalid_worker_name_is_error() {
        for name in ["Blog", "blog/../x", "-blog", "blog-", ""] {
            let raw = format!("title = \"t\"\n[deploy]\nworker = \"{name}\"\n");
            let err = parse_site_config(&raw).unwrap_err();
            assert!(err.to_string().contains("deploy.worker"), "{name}");
        }
        let long = "a".repeat(64);
        assert!(parse_site_config(&format!("title = \"t\"\n[deploy]\nworker = \"{long}\"\n")).is_err());
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
