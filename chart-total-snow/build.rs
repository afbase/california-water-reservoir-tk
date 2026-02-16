use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Copy snow_stations.csv to OUT_DIR for include_str
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

    // Aggregate snow_observations.csv at build time into total_snow.csv.
    //
    // Problem: not all stations report every day. Naive SUM-by-date produces
    // wild spikes because the total only includes whichever stations happened
    // to report that day.
    //
    // Solution: forward-fill each station's last known SWE value across all
    // dates. For each date, every station contributes either its reported
    // value or its most recent prior value. This gives a consistent total.
    let obs_src = Path::new("../fixtures/snow_observations.csv");
    let total_dest = Path::new(&out_dir).join("total_snow.csv");

    if obs_src.exists() {
        // Step 1: Parse all observations into station -> date -> swe_value
        // Format (no headers): station_id,date(YYYYMMDD),swe,depth
        let mut station_obs: HashMap<String, BTreeMap<String, f64>> = HashMap::new();
        let mut all_dates: BTreeSet<String> = BTreeSet::new();

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(obs_src)
            .expect("Failed to open snow_observations.csv");

        for record in rdr.records().flatten() {
            let station_id = record.get(0).unwrap_or("").trim().to_string();
            let date = record.get(1).unwrap_or("").trim().to_string();
            // Column 2 is SWE (snow water equivalent) in inches
            let swe_str = record.get(2).unwrap_or("").trim();
            if let Ok(swe) = swe_str.parse::<f64>() {
                if !station_id.is_empty() && !date.is_empty() {
                    station_obs
                        .entry(station_id)
                        .or_default()
                        .insert(date.clone(), swe);
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

            // Only emit dates where at least 3 stations have started reporting
            if contributing_stations >= 3 {
                output.push_str(&format!("{},{:.1}\n", date, total));
            }
        }

        fs::write(&total_dest, output).unwrap();
    } else {
        fs::write(&total_dest, "").unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../fixtures/snow_stations.csv");
    println!("cargo:rerun-if-changed=../fixtures/snow_observations.csv");
}
