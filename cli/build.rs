use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    embed_migrations(&manifest_dir.join("../migrations"), &out_dir.join("migrations.rs"));
    embed_spa(&manifest_dir.join("../web/dist"), &out_dir.join("spa.rs"));
}

fn embed_migrations(dir: &Path, out: &Path) {
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

    fs::write(out, format!("static EMBEDDED: &[(&str, &str)] = &[\n{entries}];\n")).unwrap();
}

fn embed_spa(dir: &Path, out: &Path) {
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut files = Vec::new();
    if dir.is_dir() {
        collect_files(&dir.canonicalize().unwrap(), &mut files);
    } else {
        println!(
            "cargo:warning=web/dist がないため SPA を埋め込まずにビルドします（npm --prefix web run build のあとで再ビルドしてください）"
        );
    }
    files.sort();

    let root = dir.canonicalize().unwrap_or_default();
    let entries: String = files
        .iter()
        .map(|path| {
            let relative = path.strip_prefix(&root).unwrap();
            let url_path: String = relative
                .components()
                .map(|c| format!("/{}", c.as_os_str().to_str().expect("ファイル名が UTF-8 ではありません")))
                .collect();
            format!("    ({url_path:?}, include_bytes!({:?})),\n", path.display().to_string())
        })
        .collect();

    fs::write(out, format!("static SPA: &[(&str, &[u8])] = &[\n{entries}];\n")).unwrap();
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}
