mod content;
mod db;
mod media;
mod migrations;
mod site;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, bail, Result};

const USAGE: &str = "usage: sqlite-cms [SITE_DIR] [--public <DIR>]

  SITE_DIR       site.toml と content/ を持つ記事リポジトリのディレクトリ（既定: .）
  --public <DIR> 配信用ディレクトリ（既定: public）。DB を db/ に、content/media/ を media/ に書き出す";

#[derive(Debug, PartialEq)]
struct Args {
    site_dir: PathBuf,
    public_dir: PathBuf,
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Option<Args>> {
    let mut site_dir = None;
    let mut public_dir = PathBuf::from("public");

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "-h" || arg == "--help" {
            return Ok(None);
        } else if arg == "--public" {
            public_dir = args
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| anyhow!("--public に配信用ディレクトリを指定してください\n\n{USAGE}"))?;
        } else if site_dir.is_none() {
            site_dir = Some(PathBuf::from(arg));
        } else {
            bail!("余分な引数があります: {}\n\n{USAGE}", arg.to_string_lossy());
        }
    }

    let site_dir = site_dir.unwrap_or_else(|| PathBuf::from("."));
    Ok(Some(Args { site_dir, public_dir }))
}

fn run() -> Result<()> {
    let Some(args) = parse_args(std::env::args_os().skip(1))? else {
        println!("{USAGE}");
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
