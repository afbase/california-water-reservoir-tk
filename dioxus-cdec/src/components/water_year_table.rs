use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use crate::database::Database;

#[derive(Clone, Debug)]
struct WaterYearStats {
    water_year: i32,
    min_level: u32,
    max_level: u32,
    avg_level: u32,
    start_level: u32,
    end_level: u32,
}

fn parse_date(date: &str) -> Option<(i32, i32, i32)> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 {
        let year = parts[0].parse().ok()?;
        let month = parts[1].parse().ok()?;
        let day = parts[2].parse().ok()?;
        Some((year, month, day))
    } else {
        None
    }
}

fn get_water_year(date: &str) -> Option<i32> {
    let (year, month, _) = parse_date(date)?;
    // Water year starts October 1
    if month >= 10 {
        Some(year + 1)
    } else {
        Some(year)
    }
}

fn calculate_water_year_stats(data: &[(String, u32)]) -> Vec<WaterYearStats> {
    use std::collections::HashMap;

    let mut by_year: HashMap<i32, Vec<u32>> = HashMap::new();
    let mut first_value: HashMap<i32, u32> = HashMap::new();
    let mut last_value: HashMap<i32, u32> = HashMap::new();

    for (date, value) in data {
        if let Some(wy) = get_water_year(date) {
            by_year.entry(wy).or_insert_with(Vec::new).push(*value);
            first_value.entry(wy).or_insert(*value);
            last_value.insert(wy, *value);
        }
    }

    let mut stats: Vec<WaterYearStats> = by_year
        .iter()
        .map(|(year, values)| {
            let min_level = *values.iter().min().unwrap();
            let max_level = *values.iter().max().unwrap();
            let avg_level = (values.iter().map(|v| *v as u64).sum::<u64>() / values.len() as u64) as u32;
            let start_level = *first_value.get(year).unwrap();
            let end_level = *last_value.get(year).unwrap();

            WaterYearStats {
                water_year: *year,
                min_level,
                max_level,
                avg_level,
                start_level,
                end_level,
            }
        })
        .collect();

    stats.sort_by_key(|s| std::cmp::Reverse(s.water_year));
    stats
}

#[component]
pub fn WaterYearTable(
    database: Database,
    station_id: Option<String>,
    start_date: String,
    end_date: String,
) -> Element {
    let mut stats = use_signal(|| Vec::<WaterYearStats>::new());
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);

    // Load data when inputs change
    use_effect(move || {
        let db = database.clone();
        let station = station_id.clone();
        let start = start_date.clone();
        let end = end_date.clone();

        spawn(async move {
            loading.set(true);
            error_msg.set(None);

            let result = if let Some(sid) = station {
                db.get_reservoir_data(&sid, &start, &end).await
            } else {
                db.get_data(&start, &end).await
            };

            match result {
                Ok(data) => {
                    info!("Calculating water year statistics for {} data points", data.len());
                    let water_year_stats = calculate_water_year_stats(&data);
                    stats.set(water_year_stats);
                    loading.set(false);
                }
                Err(e) => {
                    info!("Error loading data for table: {}", e);
                    error_msg.set(Some(e));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div {
            class: "water-year-table-wrapper",
            style: "margin: 20px 0;",

            h3 {
                style: "color: #2c3e50; margin-bottom: 15px;",
                "Water Year Statistics"
            }

            p {
                style: "color: #666; font-size: 14px; margin-bottom: 15px;",
                "Water years run from October 1 to September 30. All values in acre-feet."
            }

            if let Some(error) = error_msg() {
                div {
                    class: "error-message",
                    style: "background-color: #fee; color: #c33; padding: 10px; border-radius: 4px; margin: 10px 0;",
                    "Error: {error}"
                }
            }

            if loading() {
                div {
                    style: "text-align: center; padding: 20px; color: #666;",
                    "Loading statistics..."
                }
            } else if stats().is_empty() {
                div {
                    style: "text-align: center; padding: 20px; color: #666;",
                    "No data available for the selected range"
                }
            } else {
                div {
                    style: "overflow-x: auto;",
                    table {
                        style: "width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1);",

                        thead {
                            tr {
                                style: "background: #3498db; color: white;",
                                th { style: "padding: 12px; text-align: left; font-weight: 600;", "Water Year" }
                                th { style: "padding: 12px; text-align: right; font-weight: 600;", "Min" }
                                th { style: "padding: 12px; text-align: right; font-weight: 600;", "Max" }
                                th { style: "padding: 12px; text-align: right; font-weight: 600;", "Avg" }
                                th { style: "padding: 12px; text-align: right; font-weight: 600;", "Start" }
                                th { style: "padding: 12px; text-align: right; font-weight: 600;", "End" }
                            }
                        }

                        tbody {
                            for (idx, stat) in stats().iter().enumerate() {
                                {
                                    let min_formatted = format!("{}", stat.min_level);
                                    let max_formatted = format!("{}", stat.max_level);
                                    let avg_formatted = format!("{}", stat.avg_level);
                                    let start_formatted = format!("{}", stat.start_level);
                                    let end_formatted = format!("{}", stat.end_level);

                                    rsx! {
                                        tr {
                                            key: "{stat.water_year}",
                                            style: if idx % 2 == 0 { "background: #f8f9fa;" } else { "background: white;" },
                                            td { style: "padding: 10px; border-top: 1px solid #dee2e6;", "{stat.water_year}" }
                                            td { style: "padding: 10px; text-align: right; border-top: 1px solid #dee2e6;", "{min_formatted}" }
                                            td { style: "padding: 10px; text-align: right; border-top: 1px solid #dee2e6;", "{max_formatted}" }
                                            td { style: "padding: 10px; text-align: right; border-top: 1px solid #dee2e6;", "{avg_formatted}" }
                                            td { style: "padding: 10px; text-align: right; border-top: 1px solid #dee2e6;", "{start_formatted}" }
                                            td { style: "padding: 10px; text-align: right; border-top: 1px solid #dee2e6;", "{end_formatted}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
