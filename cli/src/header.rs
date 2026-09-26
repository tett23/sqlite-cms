//! サイトのヘッダ（ADR 0043）。
//!
//! 記事リポジトリの `content/header.md` に、site.toml の値を Mustache のような書き方で埋め込み、DB に入れる。
//! `content/header.md` がなければ、SPA は既定のヘッダ（サイト名、案内、検索ボックス）を出す。

use std::collections::BTreeMap;

use crate::mustache::{self, Value};
use crate::site::SiteConfig;

/// `{{> search}}` を置き換える目印。SPA がこれを検索ボックスにする。
/// HTML のブロックが前後の段落とつながらないよう、前後に空行を置く。
pub const SEARCH_PARTIAL: &str = "\n<div class=\"partial-search\"></div>\n\n";

/// ヘッダに埋め込める値。
pub fn site_values(config: &SiteConfig) -> Value {
    let license = match &config.license {
        Some(license) => Value::Map(BTreeMap::from([
            ("name".to_string(), Value::from(license.name.as_str())),
            ("url".to_string(), Value::from(license.url.clone())),
        ])),
        None => Value::Null,
    };
    Value::Map(BTreeMap::from([
        ("title".to_string(), Value::from(config.title.as_str())),
        ("author".to_string(), Value::from(config.author.clone())),
        ("description".to_string(), Value::from(config.description().as_str())),
        ("url".to_string(), Value::from(config.url())),
        ("license".to_string(), license),
    ]))
}

/// `content/header.md` に値を埋め込む。
pub fn render(template: &str, config: &SiteConfig) -> Result<String, mustache::Error> {
    let partials = BTreeMap::from([("search".to_string(), SEARCH_PARTIAL.to_string())]);
    mustache::render(template, &site_values(config), &partials)
}
