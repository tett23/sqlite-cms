//! rsync による公開（ADR 0030）。組み立てた一式を、rsync で任意のサーバー（や手元のディレクトリ）に送る。
//!
//! 送り先にあって一式にないファイルは消す（--delete）。
//! 関係のないディレクトリを誤って消さないよう、送り先が空か、前に sqlite-cms が公開した場所（目印のファイルがある）のときだけ送る。

use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::output::{SiteOutput, MARKER};
use crate::scratch::ScratchDir;

/// rsync --list-only の 1 行から名前を取り出す（権限、大きさ、日付、時刻の後が名前）。
fn listed_name(line: &str) -> Option<String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    (fields.len() >= 5).then(|| fields[4..].join(" "))
}

/// 送り先の中身を確かめる。空か、なければ（rsync が作る）、目印のファイルがあれば送ってよい。
fn check_destination(destination: &str) -> Result<()> {
    let output = Command::new("rsync")
        .args(["--list-only", destination])
        .output()
        .context("rsync を実行できません。rsync をインストールしてください")?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        if output.status.code() == Some(23) && stderr.contains("No such file or directory") {
            return Ok(());
        }
        bail!("送り先 {destination} を確かめられません: {}", stderr.trim());
    }
    let names: Vec<String> =
        String::from_utf8_lossy(&output.stdout).lines().filter_map(listed_name).filter(|name| name != ".").collect();
    if names.is_empty() || names.iter().any(|name| name == MARKER) {
        return Ok(());
    }
    bail!(
        "送り先 {destination} は空ではなく、sqlite-cms の公開先でもありません（{MARKER} がありません）。\
         送り先を確かめてください。空のディレクトリを指定すれば公開できます"
    )
}

pub fn deploy(output: &SiteOutput, destination: &str) -> Result<()> {
    // 末尾の / がないと、rsync は送り先の中にディレクトリを作る。
    let destination = if destination.ends_with('/') { destination.to_string() } else { format!("{destination}/") };
    check_destination(&destination)?;

    let scratch = ScratchDir::new()?;
    let out_dir = scratch.path().join("site");
    output.write_site(&out_dir)?;
    let source = format!("{}/", out_dir.to_str().context("一時ディレクトリのパスが UTF-8 ではありません")?);

    let status = Command::new("rsync")
        .args(["-rlptz", "--delete", &source, &destination])
        .status()
        .context("rsync を実行できません")?;
    if !status.success() {
        bail!("rsync が失敗しました（{status}）");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_taken_from_list_only_output() {
        assert_eq!(listed_name("drwxr-xr-x          4,096 2026/09/26 15:36:54 ."), Some(".".into()));
        assert_eq!(listed_name("-rw-r--r--             46 2026/09/26 15:36:54 .sqlite-cms"), Some(".sqlite-cms".into()));
        assert_eq!(listed_name("-rw-r--r--  2 2026/09/26 15:36:54 file name.txt"), Some("file name.txt".into()));
        assert_eq!(listed_name(""), None);
    }
}
