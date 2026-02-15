//! Loading spinner component.

use dioxus::prelude::*;

/// Simple loading indicator.
#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; padding: 40px; color: #666;",
            "Loading data..."
        }
    }
}
