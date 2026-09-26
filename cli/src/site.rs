use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteConfig {
    pub title: String,
    pub author: Option<String>,
    pub license: Option<License>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct License {
    pub name: String,
    pub url: Option<String>,
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
