//! Sort mode selector for water year display.

use crate::state::AppState;
use dioxus::prelude::*;

/// Dropdown selector for water year sort mode.
#[component]
pub fn SortSelector() -> Element {
    let mut state = use_context::<AppState>();
    let current_mode = (state.sort_mode)();
    let current_count = (state.display_count)();

    let on_mode_change = move |evt: Event<FormData>| {
        state.sort_mode.set(evt.value());
    };

    let on_count_change = move |evt: Event<FormData>| {
        if let Ok(count) = evt.value().parse::<usize>() {
            state.display_count.set(count.clamp(1, 100));
        }
    };

    rsx! {
        div {
            style: "margin: 8px 0; display: flex; gap: 12px; align-items: center;",
            label {
                style: "font-weight: bold;",
                "Sort by: "
                select {
                    onchange: on_mode_change,
                    option {
                        value: "most_recent",
                        selected: current_mode == "most_recent",
                        "Most Recent"
                    }
                    option {
                        value: "driest",
                        selected: current_mode == "driest",
                        "Driest Years"
                    }
                    option {
                        value: "wettest",
                        selected: current_mode == "wettest",
                        "Wettest Years"
                    }
                }
            }
            label {
                style: "font-weight: bold;",
                "Show: "
                input {
                    r#type: "number",
                    value: "{current_count}",
                    min: "1",
                    max: "100",
                    style: "width: 60px;",
                    onchange: on_count_change,
                }
                " years"
            }
        }
    }
}
