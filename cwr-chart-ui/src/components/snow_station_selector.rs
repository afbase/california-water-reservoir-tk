//! Dropdown selector for choosing a snow station.

use crate::state::AppState;
use dioxus::prelude::*;

/// Snow station dropdown selector.
/// Reads available snow stations from AppState and updates selected_station on change.
#[component]
pub fn SnowStationSelector() -> Element {
    let mut state = use_context::<AppState>();
    let stations = state.snow_stations.read().clone();
    let selected = (state.selected_station)();

    let on_change = move |evt: Event<FormData>| {
        let value = evt.value();
        state.selected_station.set(value);
    };

    rsx! {
        div {
            style: "margin: 8px 0;",
            label {
                r#for: "snow-station-select",
                style: "font-weight: bold; margin-right: 8px;",
                "Snow Station: "
            }
            select {
                id: "snow-station-select",
                onchange: on_change,
                for station in stations.iter() {
                    option {
                        value: "{station.station_id}",
                        selected: station.station_id == selected,
                        "{station.name} - {station.station_id} ({station.elevation} ft)"
                    }
                }
            }
        }
    }
}
