//! Build script for chart-cumulative-water.
//!
//! Copies the CA-only reservoir capacity CSV to OUT_DIR so it can be
//! embedded via `include_str!` at compile time. Observations are fetched
//! at runtime as a gzipped CSV. This app uses the CA-only data (excludes
//! Lake Mead and Lake Powell). Cumulative totals are derived on-the-fly
//! via SQL in the app.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // CA-only capacity excludes Mead/Powell
    let files = vec![("../fixtures/capacity-no-powell-no-mead.csv", "capacity.csv")];

    for (src_path, dest_name) in &files {
        let src = Path::new(src_path);
        let dest = Path::new(&out_dir).join(dest_name);
        if src.exists() {
            fs::copy(src, &dest).unwrap_or_else(|e| {
                panic!("Failed to copy {} to {}: {}", src_path, dest.display(), e);
            });
        } else {
            // Create empty placeholder so include_str! doesn't fail
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
