use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sqlite_cms(args: &[&str], current_dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sqlite-cms"))
        .args(args)
        .current_dir(current_dir)
        .output()
        .unwrap()
}

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../example")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn help_prints_to_stdout_and_exits_successfully() {
    let tmp = tempfile::tempdir().unwrap();
    let output = sqlite_cms(&["--help"], tmp.path());

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let help = stdout(&output);
    for section in ["usage: sqlite-cms", "引数:", "オプション:", "記事リポジトリの構成:", "出力:", "例:"] {
        assert!(help.contains(section), "ヘルプに {section} がありません");
    }
}

#[test]
fn short_help_is_the_same_as_long_help() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(
        sqlite_cms(&["-h"], tmp.path()).stdout,
        sqlite_cms(&["--help"], tmp.path()).stdout
    );
}

#[test]
fn help_does_not_touch_the_file_system() {
    let tmp = tempfile::tempdir().unwrap();
    sqlite_cms(&["--help"], tmp.path());
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
}

#[test]
fn unknown_option_fails_with_hint_on_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    let output = sqlite_cms(&["--verbose"], tmp.path());

    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    let err = stderr(&output);
    assert!(err.contains("不明なオプションです: --verbose"));
    assert!(err.contains("usage: sqlite-cms"));
    assert!(err.contains("sqlite-cms --help"));
}

#[test]
fn builds_db_and_copies_media_from_site_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let public = tmp.path().join("public");
    let output = sqlite_cms(
        &[example_dir().to_str().unwrap(), "--public", public.to_str().unwrap()],
        tmp.path(),
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let manifest = std::fs::read_to_string(public.join("db/manifest.json")).unwrap();
    let db_name = manifest.split("/db/").nth(1).unwrap().split('"').next().unwrap();
    assert!(public.join("db").join(db_name).is_file());
    assert!(public.join("media/sample.svg").is_file());
}

#[test]
fn site_dir_defaults_to_current_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let public = tmp.path().join("public");
    let output = sqlite_cms(&["--public", public.to_str().unwrap()], &example_dir());

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(public.join("db/manifest.json").is_file());
}
