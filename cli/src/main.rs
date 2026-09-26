mod content;
mod db;
mod media;
mod migrations;
mod site;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Result};

const USAGE: &str = "usage: sqlite-cms [SITE_DIR] [--public <DIR>]";

const HELP: &str = "\
sqlite-cms: 記事リポジトリの Markdown を検証して SQLite に格納し、配信用ディレクトリに書き出す

usage: sqlite-cms [SITE_DIR] [--public <DIR>]

引数:
  SITE_DIR        記事リポジトリのディレクトリ（既定: .）

オプション:
  --public <DIR>  配信用ディレクトリ（既定: public）
  -h, --help      このヘルプを表示して終了する

記事リポジトリの構成:
  site.toml       サイトのメタデータ（必須）
  content/
    index.md      トップページの本文（任意）
    posts/        post の Markdown
    articles/     article の Markdown
    pages/        固定ページの Markdown
    media/        画像など（任意）

出力:
  <DIR>/db/articles-<hash>.sqlite  全コンテンツを格納した DB（書き出しのたびに作り直す）
  <DIR>/db/manifest.json           現行の DB を指すマニフェスト
  <DIR>/media/                     content/media/ のコピー

例:
  sqlite-cms                       カレントディレクトリの記事リポジトリから ./public に書き出す
  sqlite-cms ../blog               ../blog の記事リポジトリから ./public に書き出す
  sqlite-cms ../blog --public out  書き出し先を ./out にする";

#[derive(Debug, PartialEq)]
struct Args {
    site_dir: PathBuf,
    public_dir: PathBuf,
}

fn usage_error(message: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{message}\n\n{USAGE}\n詳しくは sqlite-cms --help を参照してください")
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Option<Args>> {
    let args: Vec<OsString> = args.into_iter().collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        return Ok(None);
    }

    let mut site_dir = None;
    let mut public_dir = PathBuf::from("public");

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--public" {
            public_dir = args
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| usage_error("--public に配信用ディレクトリを指定してください"))?;
        } else if arg.to_string_lossy().starts_with('-') {
            return Err(usage_error(format_args!("不明なオプションです: {}", arg.to_string_lossy())));
        } else if site_dir.is_none() {
            site_dir = Some(PathBuf::from(arg));
        } else {
            return Err(usage_error(format_args!("余分な引数があります: {}", arg.to_string_lossy())));
        }
    }

    let site_dir = site_dir.unwrap_or_else(|| PathBuf::from("."));
    Ok(Some(Args { site_dir, public_dir }))
}

fn run() -> Result<()> {
    let Some(args) = parse_args(std::env::args_os().skip(1))? else {
        println!("{HELP}");
        return Ok(());
    };

    let bytes = db::build_db_bytes(&args.site_dir)?;
    let db_dir = args.public_dir.join("db");
    let name = db::write_output(&db_dir, &bytes)?;
    println!("{} ({} bytes)", db_dir.join(name).display(), bytes.len());

    let media_dir = args.public_dir.join("media");
    let copied = media::copy_media(&args.site_dir.join("content").join("media"), &media_dir)?;
    println!("{} ({copied} files)", media_dir.display());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sqlite-cms: {e:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Args>> {
        parse_args(args.iter().map(OsString::from))
    }

    #[test]
    fn site_dir_with_default_public_dir() {
        assert_eq!(
            parse(&["../blog-content"]).unwrap(),
            Some(Args {
                site_dir: "../blog-content".into(),
                public_dir: "public".into(),
            })
        );
    }

    #[test]
    fn public_dir_can_be_overridden() {
        assert_eq!(
            parse(&["--public", "tmp/public", "site"]).unwrap(),
            Some(Args {
                site_dir: "site".into(),
                public_dir: "tmp/public".into(),
            })
        );
    }

    #[test]
    fn help_returns_none() {
        assert_eq!(parse(&["--help"]).unwrap(), None);
        assert_eq!(parse(&["-h"]).unwrap(), None);
    }

    #[test]
    fn help_wins_over_other_arguments() {
        assert_eq!(parse(&["site", "--bogus", "--help"]).unwrap(), None);
    }

    #[test]
    fn unknown_option_is_error() {
        let err = parse(&["--verbose"]).unwrap_err().to_string();
        assert!(err.contains("不明なオプションです: --verbose"));
        assert!(err.contains("sqlite-cms --help"));
    }

    #[test]
    fn site_dir_defaults_to_current_directory() {
        assert_eq!(
            parse(&[]).unwrap(),
            Some(Args {
                site_dir: ".".into(),
                public_dir: "public".into(),
            })
        );
        assert_eq!(parse(&["--public", "out"]).unwrap().unwrap().site_dir, PathBuf::from("."));
    }

    #[test]
    fn missing_public_value_is_error() {
        assert!(parse(&["site", "--public"]).unwrap_err().to_string().contains("--public"));
    }

    #[test]
    fn extra_argument_is_error() {
        assert!(parse(&["a", "b"]).unwrap_err().to_string().contains("余分な引数"));
    }
}
