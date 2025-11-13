use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

mod database;
mod chart;

use database::Database;
use chart::ChartComponent;

const MIN_DATE: &str = "1925-01-01";
const MAX_DATE: &str = "2024-12-31";

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
    let mut error_msg = use_signal(|| None::<String>);

    // Initialize database on mount
    use_effect(move || {
        spawn(async move {
            info!("Loading database...");
            match Database::new().await {
                Ok(db) => {
                    info!("Database loaded successfully");
                    // Get actual min/max dates from database
                    if let Ok((min, max)) = db.get_date_range().await {
                        start_date.set(min.clone());
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
                style: "text-align: center; color: #2c3e50;",
                "California Water Reservoir Data"
            }

            if let Some(error) = error_msg() {
                div {
                    class: "error",
                    style: "background-color: #fee; color: #c33; padding: 10px; border-radius: 4px; margin: 10px 0;",
                    "{error}"
                }
            }

            if database().is_some() {
                div {
                    class: "controls",
                    style: "display: flex; gap: 20px; margin: 20px 0; justify-content: center; align-items: center;",

                    div {
                        label {
                            style: "margin-right: 10px;",
                            "Start Date: "
                        }
                        input {
                            r#type: "date",
                            value: "{start_date}",
                            min: MIN_DATE,
                            max: "{end_date}",
                            oninput: move |evt| {
                                start_date.set(evt.value().clone());
                            }
                        }
                    }

                    div {
                        label {
                            style: "margin-right: 10px;",
                            "End Date: "
                        }
                        input {
                            r#type: "date",
                            value: "{end_date}",
                            min: "{start_date}",
                            max: MAX_DATE,
                            oninput: move |evt| {
                                end_date.set(evt.value().clone());
                            }
                        }
                    }
                }

                ChartComponent {
                    database: database().clone().unwrap(),
                    start_date: start_date(),
                    end_date: end_date()
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
