use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

fn main() {
    // Determine a stable file in the workspace to persist the build number
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let mut counter_path = PathBuf::from(manifest_dir);
    counter_path.push(".build_number");

    // Ensure subsequent builds rerun this script when the file changes
    println!("cargo:rerun-if-changed={}", counter_path.display());

    // Read current counter (if present)
    let mut current: u64 = 0;
    if counter_path.exists() {
        if let Ok(mut f) = OpenOptions::new().read(true).open(&counter_path) {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                current = s.trim().parse().unwrap_or(0);
            }
        }
    }

    // Increment and persist
    let next = current.saturating_add(1);
    if let Some(parent) = counter_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // Write atomically-ish: write to temp then replace
    let mut tmp_path = counter_path.clone();
    tmp_path.set_extension("build_number.tmp");
    if let Ok(mut tmp) = OpenOptions::new().write(true).create(true).truncate(true).open(&tmp_path) {
        let _ = write!(tmp, "{}\n", next);
        let _ = tmp.flush();
        // Best-effort replace (fallback to direct write if rename fails)
        if fs::rename(&tmp_path, &counter_path).is_err() {
            let _ = fs::remove_file(&tmp_path);
            let _ = fs::write(&counter_path, format!("{}\n", next));
        }
    } else {
        // Fallback: try writing directly
        let _ = fs::write(&counter_path, format!("{}\n", next));
    }

    // Expose as an env var available at compile time via env!("BUILD_NUMBER")
    println!("cargo:rustc-env=BUILD_NUMBER={}", next);
}