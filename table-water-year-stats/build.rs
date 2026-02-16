//! Build script for table-water-year-stats.
//!
//! Copies the reservoir capacity CSV to OUT_DIR so it can be embedded
//! via `include_str!` at compile time. Observations are fetched at
//! runtime as a gzipped CSV.
//! This app displays a sortable table of water year statistics per reservoir.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let files = vec![("../fixtures/capacity.csv", "capacity.csv")];

    for (src_path, dest_name) in &files {
        let src = Path::new(src_path);
        let dest = Path::new(&out_dir).join(dest_name);
        if src.exists() {
            fs::copy(src, &dest).unwrap_or_else(|e| {
                panic!("Failed to copy {} to {}: {}", src_path, dest.display(), e);
            });
        } else {
            fs::write(&dest, "").unwrap();
            println!(
                "cargo:warning=Fixture file {} not found, using empty placeholder",
                src_path
            );
        }
        println!("cargo:rerun-if-changed={}", src_path);
    }

    println!("cargo:rerun-if-changed=build.rs");
}
