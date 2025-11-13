use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

mod database;
mod components;

use database::Database;
use components::{ChartComponent, DateControls, ReservoirSelector, PerReservoirChart, WaterYearTable, NormalizedYearChart};

const MIN_DATE: &str = "1925-01-01";
const MAX_DATE: &str = "2024-12-31";

#[derive(Clone, PartialEq)]
enum View {
    Statewide,
    PerReservoir,
    Statistics,
    NormalizedComparison,
}

fn main() {
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("Starting Dioxus CDEC application");
    launch(App);
}

#[component]
fn App() -> Element {
    let mut database = use_signal(|| None::<Database>);
    let mut start_date = use_signal(|| MIN_DATE.to_string());
    let mut end_date = use_signal(|| MAX_DATE.to_string());
    let mut min_date = use_signal(|| MIN_DATE.to_string());
    let mut max_date = use_signal(|| MAX_DATE.to_string());
    let mut error_msg = use_signal(|| None::<String>);
    let mut current_view = use_signal(|| View::Statewide);
    let mut selected_station = use_signal(|| None::<String>);

    // Initialize database on mount
    use_effect(move || {
        spawn(async move {
            info!("Loading database...");
            match Database::new().await {
                Ok(db) => {
                    info!("Database loaded successfully");
                    // Get actual min/max dates from database
                    if let Ok((min, max)) = db.get_date_range().await {
                        min_date.set(min.clone());
                        max_date.set(max.clone());
                        start_date.set(min);
                        end_date.set(max);
                    }
                    database.set(Some(db));
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to load database: {}", e)));
                }
            }
        });
    });

    rsx! {
        div {
            class: "container",
            style: "max-width: 1200px; margin: 0 auto; padding: 20px; font-family: sans-serif;",

            h1 {
                style: "text-align: center; color: #2c3e50; margin-bottom: 10px;",
                "California Water Reservoir Data"
            }

            p {
                style: "text-align: center; color: #666; margin-bottom: 20px;",
                "Comprehensive visualization and analysis of California reservoir levels"
            }

            if let Some(error) = error_msg() {
                div {
                    class: "error",
                    style: "background-color: #fee; color: #c33; padding: 10px; border-radius: 4px; margin: 10px 0;",
                    "{error}"
                }
            }

            if let Some(db) = database() {
                // Tab navigation
                div {
                    class: "tabs",
                    style: "display: flex; gap: 10px; margin-bottom: 20px; border-bottom: 2px solid #ddd;",

                    button {
                        class: "tab-button",
                        style: if current_view() == View::Statewide {
                            "padding: 12px 24px; border: none; background: #3498db; color: white; cursor: pointer; border-radius: 4px 4px 0 0; font-weight: 600;"
                        } else {
                            "padding: 12px 24px; border: none; background: #ecf0f1; color: #333; cursor: pointer; border-radius: 4px 4px 0 0;"
                        },
                        onclick: move |_| current_view.set(View::Statewide),
                        "Statewide"
                    }

                    button {
                        class: "tab-button",
                        style: if current_view() == View::PerReservoir {
                            "padding: 12px 24px; border: none; background: #3498db; color: white; cursor: pointer; border-radius: 4px 4px 0 0; font-weight: 600;"
                        } else {
                            "padding: 12px 24px; border: none; background: #ecf0f1; color: #333; cursor: pointer; border-radius: 4px 4px 0 0;"
                        },
                        onclick: move |_| current_view.set(View::PerReservoir),
                        "Per Reservoir"
                    }

                    button {
                        class: "tab-button",
                        style: if current_view() == View::Statistics {
                            "padding: 12px 24px; border: none; background: #3498db; color: white; cursor: pointer; border-radius: 4px 4px 0 0; font-weight: 600;"
                        } else {
                            "padding: 12px 24px; border: none; background: #ecf0f1; color: #333; cursor: pointer; border-radius: 4px 4px 0 0;"
                        },
                        onclick: move |_| current_view.set(View::Statistics),
                        "Statistics"
                    }

                    button {
                        class: "tab-button",
                        style: if current_view() == View::NormalizedComparison {
                            "padding: 12px 24px; border: none; background: #3498db; color: white; cursor: pointer; border-radius: 4px 4px 0 0; font-weight: 600;"
                        } else {
                            "padding: 12px 24px; border: none; background: #ecf0f1; color: #333; cursor: pointer; border-radius: 4px 4px 0 0;"
                        },
                        onclick: move |_| current_view.set(View::NormalizedComparison),
                        "Year Comparison"
                    }
                }

                // Render current view
                match current_view() {
                    View::Statewide => rsx! {
                        DateControls {
                            start_date: start_date(),
                            end_date: end_date(),
                            min_date: min_date(),
                            max_date: max_date(),
                            on_start_change: move |new_date| {
                                start_date.set(new_date);
                            },
                            on_end_change: move |new_date| {
                                end_date.set(new_date);
                            }
                        }

                        ChartComponent {
                            database: db,
                            start_date: start_date(),
                            end_date: end_date()
                        }
                    },

                    View::PerReservoir => rsx! {
                        ReservoirSelector {
                            database: db.clone(),
                            selected_station: selected_station(),
                            on_select: move |station| {
                                selected_station.set(Some(station));
                            }
                        }

                        if let Some(station) = selected_station() {
                            DateControls {
                                start_date: start_date(),
                                end_date: end_date(),
                                min_date: min_date(),
                                max_date: max_date(),
                                on_start_change: move |new_date| {
                                    start_date.set(new_date);
                                },
                                on_end_change: move |new_date| {
                                    end_date.set(new_date);
                                }
                            }

                            PerReservoirChart {
                                database: db,
                                station_id: station,
                                start_date: start_date(),
                                end_date: end_date()
                            }
                        } else {
                            div {
                                style: "text-align: center; padding: 40px; color: #666;",
                                "Select a reservoir to view its data"
                            }
                        }
                    },

                    View::Statistics => rsx! {
                        div {
                            style: "margin-bottom: 20px;",

                            ReservoirSelector {
                                database: db.clone(),
                                selected_station: selected_station(),
                                on_select: move |station| {
                                    selected_station.set(Some(station));
                                }
                            }

                            button {
                                style: "margin-top: 10px; padding: 8px 16px; background: #95a5a6; color: white; border: none; border-radius: 4px; cursor: pointer;",
                                onclick: move |_| selected_station.set(None),
                                "Show Statewide Statistics"
                            }
                        }

                        WaterYearTable {
                            database: db,
                            station_id: selected_station(),
                            start_date: start_date(),
                            end_date: end_date()
                        }
                    },

                    View::NormalizedComparison => rsx! {
                        div {
                            style: "margin-bottom: 20px;",

                            ReservoirSelector {
                                database: db.clone(),
                                selected_station: selected_station(),
                                on_select: move |station| {
                                    selected_station.set(Some(station));
                                }
                            }

                            button {
                                style: "margin-top: 10px; padding: 8px 16px; background: #95a5a6; color: white; border: none; border-radius: 4px; cursor: pointer;",
                                onclick: move |_| selected_station.set(None),
                                "Show Statewide Comparison"
                            }
                        }

                        NormalizedYearChart {
                            database: db,
                            station_id: selected_station(),
                            selected_years: vec![]
                        }
                    }
                }
            } else {
                div {
                    style: "text-align: center; padding: 40px; color: #666;",
                    "Loading database..."
                }
            }
        }
    }
}
