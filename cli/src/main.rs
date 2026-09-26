mod base64;
mod cloudflare;
mod content;
mod db;
mod deploy;
mod frontmatter;
mod media;
mod migrations;
mod mime;
mod output;
mod serve;
mod site;
mod spa;
#[cfg(test)]
mod testutil;

use std::ffi::OsString;
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, bail, Context, Result};

use output::SiteOutput;

const USAGE: &str = "usage: sqlite-cms <build|serve|deploy> [SITE_DIR] [オプション]";

const HELP: &str = "\
sqlite-cms: 記事リポジトリの Markdown からサイトを組み立て、プレビューし、Cloudflare に公開する

usage: sqlite-cms <コマンド> [SITE_DIR] [オプション]

コマンド:
  serve   サイトを組み立てて手元でプレビューする
  build   サイトを組み立てて配信用のディレクトリに書き出す
  deploy  サイトを組み立てて Cloudflare Workers に公開する

引数:
  SITE_DIR  記事リポジトリのディレクトリ（既定: .）

オプション:
  serve --port <PORT>  待ち受けるポート（既定: 8080。0 なら空いているポート）
  build --out <DIR>    書き出し先（既定: dist）。前回の書き出しは消して作り直す
  -h, --help           このヘルプを表示して終了する

記事リポジトリの構成:
  site.toml       サイトのメタデータ（必須）
  content/
    index.md      トップページの本文（任意）
    posts/        post の Markdown
    articles/     article の Markdown
    pages/        固定ページの Markdown
    media/        画像など（任意。/media/ で配信する）

deploy に要るもの:
  site.toml の [deploy] worker    公開先の Worker 名
  環境変数 CLOUDFLARE_API_TOKEN   Workers を編集できる API トークン
  環境変数 CLOUDFLARE_ACCOUNT_ID  Cloudflare のアカウント ID

例:
  sqlite-cms serve                 カレントディレクトリの記事リポジトリをプレビューする
  sqlite-cms build --out public    public/ に書き出す
  sqlite-cms deploy ../blog        ../blog の記事リポジトリを公開する";

#[derive(Debug, PartialEq)]
enum Command {
    Help,
    Build { site_dir: PathBuf, out_dir: PathBuf, data_only: bool },
    Serve { site_dir: PathBuf, port: u16 },
    Deploy { site_dir: PathBuf },
}

fn usage_error(message: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{message}\n\n{USAGE}\n詳しくは sqlite-cms --help を参照してください")
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command> {
    let args: Vec<OsString> = args.into_iter().collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        return Ok(Command::Help);
    }

    let mut args = args.into_iter();
    let command = args.next().ok_or_else(|| usage_error("コマンドを指定してください"))?;
    let command = command.to_string_lossy().into_owned();
    if !["build", "serve", "deploy"].contains(&command.as_str()) {
        return Err(usage_error(format_args!("不明なコマンドです: {command}")));
    }

    let mut site_dir = None;
    let mut out_dir = PathBuf::from("dist");
    let mut data_only = false;
    let mut port: u16 = 8080;

    while let Some(arg) = args.next() {
        let flag = arg.to_string_lossy().into_owned();
        match (command.as_str(), flag.as_str()) {
            ("build", "--out") => {
                out_dir = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| usage_error("--out に書き出し先を指定してください"))?;
            }
            ("build", "--data-only") => data_only = true,
            ("serve", "--port") => {
                let value = args.next().ok_or_else(|| usage_error("--port にポート番号を指定してください"))?;
                port = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| usage_error(format_args!("ポート番号が不正です: {}", value.to_string_lossy())))?;
            }
            (_, f) if f.starts_with('-') => {
                return Err(usage_error(format_args!("{command} では使えないオプションです: {f}")));
            }
            _ if site_dir.is_none() => site_dir = Some(PathBuf::from(arg)),
            _ => return Err(usage_error(format_args!("余分な引数があります: {flag}"))),
        }
    }

    let site_dir = site_dir.unwrap_or_else(|| PathBuf::from("."));
    Ok(match command.as_str() {
        "build" => Command::Build { site_dir, out_dir, data_only },
        "serve" => Command::Serve { site_dir, port },
        _ => Command::Deploy { site_dir },
    })
}

fn build(site_dir: PathBuf, out_dir: PathBuf, data_only: bool) -> Result<()> {
    if data_only {
        let output = SiteOutput::data(&site_dir)?;
        output.write_data(&out_dir)?;
        println!("{}（{} ファイル、DB とメディアのみ）", out_dir.display(), output.files.len());
    } else {
        let output = SiteOutput::site(&site_dir, spa::embedded())?;
        output.write_site(&out_dir)?;
        println!("{}（{} ファイル、{} バイト）", out_dir.display(), output.files.len(), output.total_bytes());
    }
    Ok(())
}

fn serve(site_dir: PathBuf, port: u16) -> Result<()> {
    let output = SiteOutput::site(&site_dir, spa::embedded())?;
    let listener =
        TcpListener::bind(("127.0.0.1", port)).with_context(|| format!("ポート {port} で待ち受けられません"))?;
    let address = listener.local_addr()?;
    println!("http://{address}/ でプレビューしています（記事を変えたら再起動してください。Ctrl-C で終了）");
    std::io::stdout().flush()?;
    serve::serve(output, listener);
    Ok(())
}

fn required_env(name: &str) -> Result<String> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => bail!("環境変数 {name} を設定してください（詳しくは sqlite-cms --help）"),
    }
}

fn deploy(site_dir: PathBuf) -> Result<()> {
    let config = site::read_site_config(&site_dir)?;
    let worker = config
        .deploy
        .map(|d| d.worker)
        .ok_or_else(|| anyhow!("site.toml に公開先がありません。[deploy] に worker = \"Worker 名\" を書いてください"))?;
    let api_token = required_env("CLOUDFLARE_API_TOKEN")?;
    let account_id = required_env("CLOUDFLARE_ACCOUNT_ID")?;
    if !account_id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        bail!("環境変数 CLOUDFLARE_ACCOUNT_ID が不正です（英数字のアカウント ID を指定してください）");
    }
    let api_base =
        std::env::var("CLOUDFLARE_API_BASE_URL").unwrap_or_else(|_| cloudflare::DEFAULT_API_BASE.to_string());

    let output = SiteOutput::site(&site_dir, spa::embedded())?;
    println!("{} ファイル（{} バイト）を Worker {worker} に公開します", output.files.len(), output.total_bytes());

    let http = cloudflare::UreqHttp::new();
    let client = cloudflare::Client::new(&http, &api_base, &account_id, &api_token);
    let report = deploy::deploy(&client, &worker, &output)?;

    println!("公開しました（{} ファイル中 {} ファイルをアップロード）", report.total, report.uploaded);
    if let Some(url) = report.url {
        println!("{url}");
    }
    Ok(())
}

fn run() -> Result<()> {
    match parse_args(std::env::args_os().skip(1))? {
        Command::Help => {
            println!("{HELP}");
            Ok(())
        }
        Command::Build { site_dir, out_dir, data_only } => build(site_dir, out_dir, data_only),
        Command::Serve { site_dir, port } => serve(site_dir, port),
        Command::Deploy { site_dir } => deploy(site_dir),
    }
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

    fn parse(args: &[&str]) -> Result<Command> {
        parse_args(args.iter().map(OsString::from))
    }

    fn err(args: &[&str]) -> String {
        parse(args).unwrap_err().to_string()
    }

    #[test]
    fn build_defaults() {
        assert_eq!(
            parse(&["build"]).unwrap(),
            Command::Build { site_dir: ".".into(), out_dir: "dist".into(), data_only: false }
        );
    }

    #[test]
    fn build_with_site_dir_and_out() {
        assert_eq!(
            parse(&["build", "--out", "public", "../blog", "--data-only"]).unwrap(),
            Command::Build { site_dir: "../blog".into(), out_dir: "public".into(), data_only: true }
        );
    }

    #[test]
    fn serve_defaults_and_port() {
        assert_eq!(parse(&["serve"]).unwrap(), Command::Serve { site_dir: ".".into(), port: 8080 });
        assert_eq!(
            parse(&["serve", "blog", "--port", "0"]).unwrap(),
            Command::Serve { site_dir: "blog".into(), port: 0 }
        );
    }

    #[test]
    fn deploy_defaults() {
        assert_eq!(parse(&["deploy"]).unwrap(), Command::Deploy { site_dir: ".".into() });
    }

    #[test]
    fn help_wins_over_everything() {
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help);
        assert_eq!(parse(&["-h"]).unwrap(), Command::Help);
        assert_eq!(parse(&["bogus", "--nope", "--help"]).unwrap(), Command::Help);
    }

    #[test]
    fn missing_or_unknown_command_is_error() {
        assert!(err(&[]).contains("コマンドを指定してください"));
        assert!(err(&["publish"]).contains("不明なコマンドです: publish"));
    }

    #[test]
    fn options_are_checked_per_command() {
        assert!(err(&["deploy", "--out", "x"]).contains("deploy では使えないオプションです: --out"));
        assert!(err(&["build", "--port", "1"]).contains("build では使えないオプションです: --port"));
        assert!(err(&["serve", "--verbose"]).contains("--verbose"));
    }

    #[test]
    fn option_values_are_validated() {
        assert!(err(&["build", "--out"]).contains("--out"));
        assert!(err(&["serve", "--port"]).contains("--port"));
        assert!(err(&["serve", "--port", "http"]).contains("ポート番号が不正です"));
        assert!(err(&["serve", "--port", "70000"]).contains("ポート番号が不正です"));
    }

    #[test]
    fn extra_argument_is_error() {
        assert!(err(&["build", "a", "b"]).contains("余分な引数があります: b"));
    }

    #[test]
    fn errors_point_to_help() {
        assert!(err(&["build", "--bogus"]).contains("sqlite-cms --help"));
    }
}
