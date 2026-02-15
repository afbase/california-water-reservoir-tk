//! Date range picker with start and end date inputs.

use crate::state::AppState;
use dioxus::prelude::*;

/// Date range picker for filtering chart data.
#[component]
pub fn DateRangePicker() -> Element {
    let mut state = use_context::<AppState>();
    let start = (state.start_date)();
    let end = (state.end_date)();

    let on_start_change = move |evt: Event<FormData>| {
        state.start_date.set(evt.value());
    };

    let on_end_change = move |evt: Event<FormData>| {
        state.end_date.set(evt.value());
    };

    rsx! {
        div {
            style: "margin: 8px 0; display: flex; gap: 12px; align-items: center;",
            label {
                style: "font-weight: bold;",
                "From: "
                input {
                    r#type: "date",
                    value: "{start}",
                    onchange: on_start_change,
                }
            }
            label {
                style: "font-weight: bold;",
                "To: "
                input {
                    r#type: "date",
                    value: "{end}",
                    onchange: on_end_change,
                }
            }
        }
    }
}
