//! Error display component.

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ErrorDisplayProps {
    pub message: String,
}

/// Displays an error message in a styled box.
#[component]
pub fn ErrorDisplay(props: ErrorDisplayProps) -> Element {
    rsx! {
        div {
            class: "cwr-error",
            role: "alert",
            strong { "Error: " }
            "{props.message}"
        }
    }
}
