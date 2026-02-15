//! Build script for chart-water-years.
//!
//! Copies reservoir capacity and observation CSV files to OUT_DIR
//! so they can be embedded via `include_str!` at compile time.
//! This app overlays water years for a selected reservoir to compare
//! driest, wettest, and most recent years.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let files = vec![
        ("../fixtures/capacity.csv", "capacity.csv"),
        ("../fixtures/observations.csv", "observations.csv"),
    ];

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
