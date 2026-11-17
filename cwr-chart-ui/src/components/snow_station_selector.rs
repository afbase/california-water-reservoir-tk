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

    rsx! {
        label {
            class: "cwr-field",
            "Snow station"
            select {
                onchange: move |evt| state.selected_station.set(evt.value()),
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
