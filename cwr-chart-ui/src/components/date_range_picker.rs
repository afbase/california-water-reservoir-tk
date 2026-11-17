//! Date range picker with start and end date inputs.

use crate::state::AppState;
use dioxus::prelude::*;

/// Date range picker for filtering chart data.
#[component]
pub fn DateRangePicker() -> Element {
    let mut state = use_context::<AppState>();
    let start = (state.start_date)();
    let end = (state.end_date)();

    rsx! {
        label {
            class: "cwr-field",
            "From"
            input {
                r#type: "date",
                value: "{start}",
                max: "{end}",
                onchange: move |evt| state.start_date.set(evt.value()),
            }
        }
        label {
            class: "cwr-field",
            "To"
            input {
                r#type: "date",
                value: "{end}",
                min: "{start}",
                onchange: move |evt| state.end_date.set(evt.value()),
            }
        }
    }
}
