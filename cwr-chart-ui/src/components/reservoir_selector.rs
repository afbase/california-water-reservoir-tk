//! Dropdown selector for choosing a reservoir.

use crate::state::AppState;
use dioxus::prelude::*;

/// Reservoir dropdown selector.
/// Reads available reservoirs from AppState and updates selected_station on change.
#[component]
pub fn ReservoirSelector() -> Element {
    let mut state = use_context::<AppState>();
    let reservoirs = state.reservoirs.read().clone();
    let selected = (state.selected_station)();

    rsx! {
        label {
            class: "cwr-field",
            "Reservoir"
            select {
                onchange: move |evt| state.selected_station.set(evt.value()),
                for reservoir in reservoirs.iter() {
                    option {
                        value: "{reservoir.station_id}",
                        selected: reservoir.station_id == selected,
                        "{reservoir.dam} - {reservoir.station_id}"
                    }
                }
            }
        }
    }
}
