mod build;
mod content;
mod migrations;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;

fn run() -> Result<()> {
    let root = std::env::args_os().nth(1).map_or_else(|| PathBuf::from("."), PathBuf::from);

    let bytes = build::build_db_bytes(&root.join("content"), &root.join("migrations"))?;
    let name = build::write_output(&root.join("public").join("db"), &bytes)?;

    println!("public/db/{name} ({} bytes)", bytes.len());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("build-db: {e:#}");
            ExitCode::FAILURE
        }
    }
}
