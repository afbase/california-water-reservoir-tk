use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Copy snow fixture CSVs to OUT_DIR for include_str
    let stations_src = Path::new("../fixtures/snow_stations.csv");
    if stations_src.exists() {
        fs::copy(stations_src, Path::new(&out_dir).join("snow_stations.csv")).unwrap();
    } else {
        fs::write(
            Path::new(&out_dir).join("snow_stations.csv"),
            "station_id,name,elevation,river_basin,county,latitude,longitude\n",
        )
        .unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../fixtures/snow_stations.csv");
}
