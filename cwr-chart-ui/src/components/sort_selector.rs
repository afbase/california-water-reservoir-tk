//! Sort mode selector for water year display.

use crate::state::AppState;
use dioxus::prelude::*;

/// Controls for which water years to show, and how many.
#[component]
pub fn SortSelector() -> Element {
    let mut state = use_context::<AppState>();
    let current_mode = (state.sort_mode)();
    let current_count = (state.display_count)();

    rsx! {
        label {
            class: "cwr-field",
            "Sort by"
            select {
                onchange: move |evt| state.sort_mode.set(evt.value()),
                option { value: "most_recent", selected: current_mode == "most_recent", "Most recent" }
                option { value: "driest", selected: current_mode == "driest", "Driest years" }
                option { value: "wettest", selected: current_mode == "wettest", "Wettest years" }
            }
        }
        label {
            class: "cwr-field",
            "Show"
            input {
                r#type: "number",
                value: "{current_count}",
                min: "1",
                max: "100",
                onchange: move |evt| {
                    if let Ok(count) = evt.value().parse::<usize>() {
                        state.display_count.set(count.clamp(1, 100));
                    }
                },
            }
            "years"
        }
    }
}
