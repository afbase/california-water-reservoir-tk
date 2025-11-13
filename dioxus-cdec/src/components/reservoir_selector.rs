use dioxus::prelude::*;
use crate::database::{Database, Reservoir};

#[component]
pub fn ReservoirSelector(
    database: Database,
    selected_station: Option<String>,
    on_select: EventHandler<String>,
) -> Element {
    let mut reservoirs = use_signal(|| Vec::<Reservoir>::new());
    let mut loading = use_signal(|| true);

    // Load reservoirs on mount
    use_effect(move || {
        let db = database.clone();
        spawn(async move {
            match db.get_reservoirs().await {
                Ok(list) => {
                    reservoirs.set(list);
                    loading.set(false);
                }
                Err(e) => {
                    dioxus_logger::tracing::error!("Failed to load reservoirs: {}", e);
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div {
            class: "reservoir-selector",
            style: "margin: 20px 0;",

            label {
                style: "display: block; margin-bottom: 8px; font-weight: bold; color: #2c3e50;",
                "Select Reservoir:"
            }

            if loading() {
                div {
                    style: "padding: 10px; color: #666;",
                    "Loading reservoirs..."
                }
            } else {
                select {
                    style: "width: 100%; padding: 10px; font-size: 16px; border: 1px solid #ddd; border-radius: 4px;",
                    value: selected_station.clone().unwrap_or_default(),
                    onchange: move |evt| {
                        on_select.call(evt.value());
                    },

                    option {
                        value: "",
                        disabled: true,
                        selected: selected_station.is_none(),
                        "-- Choose a reservoir --"
                    }

                    for reservoir in reservoirs() {
                        option {
                            value: "{reservoir.station_id}",
                            selected: Some(reservoir.station_id.clone()) == selected_station,
                            "{reservoir.lake_name.clone().unwrap_or(reservoir.dam_name.clone().unwrap_or(reservoir.station_id.clone()))} ({reservoir.station_id})"
                        }
                    }
                }
            }
        }
    }
}
