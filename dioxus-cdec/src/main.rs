use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

mod database;
mod components;

use database::Database;
use components::{ChartComponent, DateControls};

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
    let mut min_date = use_signal(|| MIN_DATE.to_string());
    let mut max_date = use_signal(|| MAX_DATE.to_string());
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
                "Total statewide reservoir levels over time"
            }

            if let Some(error) = error_msg() {
                div {
                    class: "error",
                    style: "background-color: #fee; color: #c33; padding: 10px; border-radius: 4px; margin: 10px 0;",
                    "{error}"
                }
            }

            if let Some(db) = database() {
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
            } else {
                div {
                    style: "text-align: center; padding: 40px; color: #666;",
                    "Loading database..."
                }
            }
        }
    }
}
