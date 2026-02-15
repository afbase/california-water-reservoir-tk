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

    let on_change = move |evt: Event<FormData>| {
        let value = evt.value();
        state.selected_station.set(value);
    };

    rsx! {
        div {
            style: "margin: 8px 0;",
            label {
                r#for: "reservoir-select",
                style: "font-weight: bold; margin-right: 8px;",
                "Reservoir: "
            }
            select {
                id: "reservoir-select",
                onchange: on_change,
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
