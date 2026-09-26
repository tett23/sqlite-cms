//! GitHub Pages への公開（ADR 0030）。
//!
//! 組み立てた一式を、記事リポジトリの gh-pages ブランチ（設定で変えられる）に push する。
//! 作業ツリー、インデックス、手元のブランチには触れない。Git の低レベルのコマンドで、
//! 一時的なインデックスから公開用のコミットを作り、そのコミットをリモートのブランチに強制的に push する。
//! 記事リポジトリの Git の設定（リモートと認証情報）をそのまま使うので、GitHub Actions でも手元でも同じように動く。

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::output::SiteOutput;
use crate::scratch::ScratchDir;

/// git を実行し、標準出力を返す。失敗したら標準エラー出力を添えてエラーにする。
fn git(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .context("git を実行できません。Git をインストールしてください")?;
    if !output.status.success() {
        bail!("git {} が失敗しました: {}", args.join(" "), String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_config(dir: &Path, key: &str) -> Option<String> {
    git(dir, &["config", key], &[]).ok().filter(|value| !value.is_empty())
}

/// GitHub Pages が配信するための、公開用のファイル。
/// `.nojekyll` がないと、GitHub Pages は Jekyll でサイトを組み立て直し、`_` で始まるファイルなどを配信しない。
/// 独自ドメインを使うときは、CNAME を毎回置く（強制的に push するので、GitHub の画面で設定した CNAME は消える）。
fn write_pages_files(out_dir: &Path, cname: Option<&str>) -> Result<()> {
    std::fs::write(out_dir.join(".nojekyll"), "")?;
    if let Some(cname) = cname {
        std::fs::write(out_dir.join("CNAME"), format!("{cname}\n"))?;
    }
    Ok(())
}

pub struct PagesReport {
    pub commit: String,
    pub branch: String,
}

pub fn deploy(site_dir: &Path, output: &SiteOutput, remote: &str, branch: &str, cname: Option<&str>) -> Result<PagesReport> {
    let git_dir = git(site_dir, &["rev-parse", "--absolute-git-dir"], &[])
        .map_err(|_| anyhow::anyhow!("{} は Git のリポジトリの中にありません。GitHub Pages には、記事リポジトリから公開します", site_dir.display()))?;
    git(site_dir, &["remote", "get-url", remote], &[])
        .map_err(|_| anyhow::anyhow!("Git のリモート {remote} がありません（site.toml の deploy.remote で変えられます）"))?;

    let scratch = ScratchDir::new()?;
    let out_dir = scratch.path().join("site");
    output.write_site(&out_dir)?;
    write_pages_files(&out_dir, cname)?;

    let index = scratch.path().join("index");
    let index = index.to_str().context("一時ディレクトリのパスが UTF-8 ではありません")?;
    let out = out_dir.to_str().context("一時ディレクトリのパスが UTF-8 ではありません")?;
    let with_index = [("GIT_INDEX_FILE", index)];
    // 作業ツリーを公開用のディレクトリにし、一時的なインデックスにすべてのファイルを入れる。
    // --force を付け、利用者の全体の除外設定（core.excludesFile）で公開用のファイルが漏れないようにする。
    git(&out_dir, &["--git-dir", &git_dir, "--work-tree", out, "add", "--all", "--force", "."], &with_index)?;
    let tree = git(&out_dir, &["--git-dir", &git_dir, "write-tree"], &with_index)?;

    let source = git(site_dir, &["rev-parse", "--short", "HEAD"], &[]).ok();
    let message = match &source {
        Some(commit) => format!("sqlite-cms で公開（{commit} から）"),
        None => "sqlite-cms で公開".to_string(),
    };
    // 名前とメールアドレスが設定されていない環境（CI など）でもコミットを作れるようにする。
    let mut identity = Vec::new();
    if git_config(site_dir, "user.name").is_none() {
        identity.extend([("GIT_AUTHOR_NAME", "sqlite-cms"), ("GIT_COMMITTER_NAME", "sqlite-cms")]);
    }
    if git_config(site_dir, "user.email").is_none() {
        identity.extend([("GIT_AUTHOR_EMAIL", "sqlite-cms@localhost"), ("GIT_COMMITTER_EMAIL", "sqlite-cms@localhost")]);
    }
    let commit = git(site_dir, &["commit-tree", &tree, "-m", &message], &identity)?;

    // push の進み具合と、認証の問い合わせは、そのまま利用者に見せる。
    let status = Command::new("git")
        .current_dir(site_dir)
        .args(["push", "--force", remote, &format!("{commit}:refs/heads/{branch}")])
        .stdin(Stdio::inherit())
        .status()
        .context("git を実行できません")?;
    if !status.success() {
        bail!("{remote} の {branch} ブランチに push できませんでした");
    }
    Ok(PagesReport { commit, branch: branch.to_string() })
}
