//! Loading indicator component.

use dioxus::prelude::*;

/// Simple loading indicator.
#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div { class: "cwr-status", "Loading data\u{2026}" }
    }
}
