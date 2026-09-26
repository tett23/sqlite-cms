mod base64;
mod cloudflare;
mod content;
mod date;
mod db;
mod deploy;
mod frontmatter;
mod media;
mod migrations;
mod mime;
mod output;
mod scaffold;
mod serve;
mod site;
mod spa;
#[cfg(test)]
mod testutil;

use std::ffi::OsString;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{anyhow, bail, Context, Result};

use output::SiteOutput;
use scaffold::Kind;

const USAGE: &str = "usage: sqlite-cms <init|new|serve|build|deploy> [引数] [オプション]";

const HELP: &str = "\
sqlite-cms: 記事リポジトリの Markdown からサイトを組み立て、プレビューし、Cloudflare に公開する

usage: sqlite-cms <コマンド> [引数] [オプション]

コマンド:
  init [SITE_DIR]               記事リポジトリに必要な site.toml と content/ を作る
  new <種別> <SLUG> [SITE_DIR]  記事の雛形を作る（種別は post、article、page）
  serve [SITE_DIR]              サイトを組み立てて手元でプレビューする
  build [SITE_DIR]              サイトを組み立てて配信用のディレクトリに書き出す
  deploy [SITE_DIR]             サイトを組み立てて Cloudflare Workers に公開する

引数:
  SITE_DIR  記事リポジトリのディレクトリ（既定: .）
  SLUG      ファイル名と URL になる名前（文字、数字、-、_）

オプション:
  init --title <TITLE>     サイト名（既定: ディレクトリ名）
  init --force             site.toml や content/ があっても作り直す。init が作るファイル
                           （site.toml、index.md、pages/about.md）は上書きし、記事や画像は消さない
  new --title <TITLE>      タイトル（既定: SLUG）
  new --date <YYYY-MM-DD>  日付（既定: site.toml の timezone での今日。page では使えない）
  serve --port <PORT>      待ち受けるポート（既定: 8080。0 なら空いているポート）
  build --out <DIR>        書き出し先（既定: dist）。前回の書き出しは消して作り直す
  -h, --help               このヘルプを表示して終了する

記事リポジトリの構成:
  site.toml       サイトのメタデータ（必須）。timezone で new の日付のタイムゾーンを
                  指定できる（Asia/Tokyo や +09:00。既定は環境のタイムゾーン）
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
  sqlite-cms init my-blog                        my-blog/ に記事リポジトリを作る
  sqlite-cms new post hello --title はじめまして  content/posts/hello.md を作る
  sqlite-cms serve                               カレントディレクトリの記事リポジトリをプレビューする
  sqlite-cms build --out public                  public/ に書き出す
  sqlite-cms deploy ../blog                      ../blog の記事リポジトリを公開する";

#[derive(Debug, PartialEq)]
enum Command {
    Help,
    Init { site_dir: PathBuf, title: Option<String>, force: bool },
    New { kind: Kind, slug: String, site_dir: PathBuf, title: Option<String>, date: Option<String> },
    Build { site_dir: PathBuf, out_dir: PathBuf, data_only: bool },
    Serve { site_dir: PathBuf, port: u16 },
    Deploy { site_dir: PathBuf },
}

fn usage_error(message: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{message}\n\n{USAGE}\n詳しくは sqlite-cms --help を参照してください")
}

fn option_value(args: &mut impl Iterator<Item = OsString>, flag: &str, what: &str) -> Result<String> {
    args.next()
        .map(|v| v.to_string_lossy().into_owned())
        .ok_or_else(|| usage_error(format_args!("{flag} に{what}を指定してください")))
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command> {
    let args: Vec<OsString> = args.into_iter().collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        return Ok(Command::Help);
    }

    let mut args = args.into_iter();
    let command = args.next().ok_or_else(|| usage_error("コマンドを指定してください"))?;
    let command = command.to_string_lossy().into_owned();
    if !["init", "new", "build", "serve", "deploy"].contains(&command.as_str()) {
        return Err(usage_error(format_args!("不明なコマンドです: {command}")));
    }

    let mut positionals: Vec<OsString> = Vec::new();
    let mut out_dir = PathBuf::from("dist");
    let mut data_only = false;
    let mut port: u16 = 8080;
    let mut title = None;
    let mut date = None;
    let mut force = false;

    while let Some(arg) = args.next() {
        let flag = arg.to_string_lossy().into_owned();
        match (command.as_str(), flag.as_str()) {
            ("build", "--out") => out_dir = PathBuf::from(option_value(&mut args, "--out", "書き出し先")?),
            ("build", "--data-only") => data_only = true,
            ("serve", "--port") => {
                let value = option_value(&mut args, "--port", "ポート番号")?;
                port = value.parse().map_err(|_| usage_error(format_args!("ポート番号が不正です: {value}")))?;
            }
            ("new" | "init", "--title") => title = Some(option_value(&mut args, "--title", "タイトル")?),
            ("init", "--force") => force = true,
            ("new", "--date") => {
                let value = option_value(&mut args, "--date", "日付")?;
                if !date::is_valid_date(&value) {
                    return Err(usage_error(format_args!("日付は実在する YYYY-MM-DD で指定してください: {value}")));
                }
                date = Some(value);
            }
            (_, f) if f.starts_with('-') => {
                return Err(usage_error(format_args!("{command} では使えないオプションです: {f}")));
            }
            _ => positionals.push(arg),
        }
    }

    let max_positionals = if command == "new" { 3 } else { 1 };
    if let Some(extra) = positionals.get(max_positionals) {
        return Err(usage_error(format_args!("余分な引数があります: {}", extra.to_string_lossy())));
    }
    let mut positionals = positionals.into_iter();

    if command == "new" {
        let kind = positionals
            .next()
            .ok_or_else(|| usage_error("記事の種別（post、article、page）を指定してください"))?
            .to_string_lossy()
            .into_owned();
        let kind = Kind::parse(&kind)
            .ok_or_else(|| usage_error(format_args!("不明な種別です: {kind}（post、article、page のどれか）")))?;
        let slug = positionals
            .next()
            .ok_or_else(|| usage_error("slug（ファイル名と URL になる名前）を指定してください"))?
            .to_string_lossy()
            .into_owned();
        if kind == Kind::Page && date.is_some() {
            return Err(usage_error("page は日付を持たないので --date は使えません"));
        }
        let site_dir = positionals.next().map_or_else(|| PathBuf::from("."), PathBuf::from);
        return Ok(Command::New { kind, slug, site_dir, title, date });
    }

    let site_dir = positionals.next().map_or_else(|| PathBuf::from("."), PathBuf::from);
    Ok(match command.as_str() {
        "init" => Command::Init { site_dir, title, force },
        "build" => Command::Build { site_dir, out_dir, data_only },
        "serve" => Command::Serve { site_dir, port },
        _ => Command::Deploy { site_dir },
    })
}

fn init(site_dir: PathBuf, title: Option<String>, force: bool) -> Result<()> {
    let report = scaffold::init(&site_dir, title.as_deref(), force)?;
    for path in &report.written {
        let verb = if report.overwritten.contains(path) { "上書きしました" } else { "作りました" };
        println!("{} を{verb}", path.display());
    }
    let target = if site_dir == Path::new(".") { String::new() } else { format!(" {}", site_dir.display()) };
    println!();
    println!("次は記事を書いてプレビューする:");
    println!("  sqlite-cms new post <SLUG>{target}");
    println!("  sqlite-cms serve{target}");
    Ok(())
}

fn new_document(kind: Kind, slug: String, site_dir: PathBuf, title: Option<String>, date: Option<String>) -> Result<()> {
    let title = title.unwrap_or_else(|| slug.clone());
    let date = match date {
        Some(date) => date,
        None => {
            let timezone = if site_dir.join("site.toml").is_file() {
                site::read_site_config(&site_dir)?.timezone()
            } else {
                None
            };
            date::today_in(timezone.as_ref())?
        }
    };
    let path = scaffold::create(&site_dir, kind, &slug, &title, &date)?;
    println!("{} を作りました", path.display());
    Ok(())
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
        Command::Init { site_dir, title, force } => init(site_dir, title, force),
        Command::New { kind, slug, site_dir, title, date } => new_document(kind, slug, site_dir, title, date),
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
    fn init_defaults_and_options() {
        assert_eq!(parse(&["init"]).unwrap(), Command::Init { site_dir: ".".into(), title: None, force: false });
        assert_eq!(
            parse(&["init", "my-blog", "--title", "記事置き場", "--force"]).unwrap(),
            Command::Init { site_dir: "my-blog".into(), title: Some("記事置き場".into()), force: true }
        );
        assert!(err(&["init", "a", "b"]).contains("余分な引数があります: b"));
        assert!(err(&["init", "--date", "2026-09-26"]).contains("init では使えないオプションです: --date"));
        assert!(err(&["build", "--force"]).contains("build では使えないオプションです: --force"));
    }

    #[test]
    fn new_with_kind_slug_and_defaults() {
        assert_eq!(
            parse(&["new", "post", "hello"]).unwrap(),
            Command::New { kind: Kind::Post, slug: "hello".into(), site_dir: ".".into(), title: None, date: None }
        );
    }

    #[test]
    fn new_with_all_options() {
        assert_eq!(
            parse(&["new", "article", "long", "../blog", "--title", "長い読み物", "--date", "2026-09-26"]).unwrap(),
            Command::New {
                kind: Kind::Article,
                slug: "long".into(),
                site_dir: "../blog".into(),
                title: Some("長い読み物".into()),
                date: Some("2026-09-26".into()),
            }
        );
    }

    #[test]
    fn new_argument_errors() {
        assert!(err(&["new"]).contains("記事の種別"));
        assert!(err(&["new", "blog", "x"]).contains("不明な種別です: blog"));
        assert!(err(&["new", "post"]).contains("slug"));
        assert!(err(&["new", "post", "x", "site", "extra"]).contains("余分な引数があります: extra"));
        assert!(err(&["new", "post", "x", "--date", "2026-02-30"]).contains("実在する YYYY-MM-DD"));
        assert!(err(&["new", "page", "about", "--date", "2026-09-26"]).contains("page は日付を持たない"));
        assert!(err(&["new", "post", "x", "--title"]).contains("--title にタイトル"));
        assert!(err(&["build", "--title", "x"]).contains("build では使えないオプションです: --title"));
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
