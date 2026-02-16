use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Copy capacity.csv to OUT_DIR for include_str
    let capacity_src = Path::new("../fixtures/capacity.csv");
    if capacity_src.exists() {
        fs::copy(capacity_src, Path::new(&out_dir).join("capacity.csv")).unwrap();
    } else {
        fs::write(
            Path::new(&out_dir).join("capacity.csv"),
            "ID,DAM,LAKE,STREAM,CAPACITY (AF),YEAR FILL\nSHA,Shasta,Lake Shasta,Sacramento River,4552000,1954\n",
        )
        .unwrap();
    }

    // Aggregate observations.csv at build time into total_water.csv.
    //
    // Problem: not all stations report every day. Some report daily, others
    // monthly (typically on the 1st). Naive SUM-by-date produces wild spikes
    // because the total only includes whichever stations happened to report.
    //
    // Solution: forward-fill each station's last known value across all dates.
    // For each date, every station contributes either its reported value or
    // its most recent prior value. This gives a consistent total.
    let obs_src = Path::new("../fixtures/observations.csv");
    let total_dest = Path::new(&out_dir).join("total_water.csv");

    if obs_src.exists() {
        // Step 1: Parse all observations into station -> date -> value
        let mut station_obs: HashMap<String, BTreeMap<String, f64>> = HashMap::new();
        let mut all_dates: BTreeSet<String> = BTreeSet::new();

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(obs_src)
            .expect("Failed to open observations.csv");

        for record in rdr.records().flatten() {
            let station_id = record.get(0).unwrap_or("").trim().to_string();
            let date = record.get(2).unwrap_or("").trim().to_string();
            let value_str = record.get(3).unwrap_or("").trim();
            if let Ok(value) = value_str.parse::<f64>() {
                if !station_id.is_empty() && !date.is_empty() {
                    station_obs
                        .entry(station_id)
                        .or_default()
                        .insert(date.clone(), value);
                    all_dates.insert(date);
                }
            }
        }

        let dates: Vec<String> = all_dates.into_iter().collect();

        // Step 2: For each date, compute the total by forward-filling each station.
        // A station contributes to the total starting from its first observation.
        let mut output = String::new();
        let mut last_values: HashMap<String, f64> = HashMap::new();

        for date in &dates {
            let mut total = 0.0;
            let mut contributing_stations = 0;

            for (station_id, obs) in &station_obs {
                // Update last known value if this station reported today
                if let Some(&value) = obs.get(date) {
                    last_values.insert(station_id.clone(), value);
                }

                // Use the forward-filled value (if the station has ever reported)
                if let Some(&value) = last_values.get(station_id) {
                    total += value;
                    contributing_stations += 1;
                }
            }

            // Only emit dates where at least a few stations have started reporting
            if contributing_stations >= 5 {
                output.push_str(&format!("{},{:.0}\n", date, total));
            }
        }

        fs::write(&total_dest, output).unwrap();
    } else {
        fs::write(&total_dest, "").unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../fixtures/capacity.csv");
    println!("cargo:rerun-if-changed=../fixtures/observations.csv");
}
