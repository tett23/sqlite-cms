use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../migrations");
    println!("cargo:rerun-if-changed={}", dir.display());

    let dir = dir.canonicalize().expect("migrations ディレクトリがありません");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("migrations ディレクトリを読めません")
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    files.sort();

    let entries: String = files
        .iter()
        .map(|path| {
            let file_name = path.file_name().unwrap().to_str().expect("ファイル名が UTF-8 ではありません");
            format!("    ({file_name:?}, include_str!({:?})),\n", path.display().to_string())
        })
        .collect();

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("migrations.rs");
    fs::write(out, format!("static EMBEDDED: &[(&str, &str)] = &[\n{entries}];\n")).unwrap();
}
