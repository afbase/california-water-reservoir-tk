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
            style: "padding: 12px 16px; margin: 8px 0; background: #FFEBEE; color: #C62828; border-radius: 4px; border: 1px solid #EF9A9A;",
            strong { "Error: " }
            "{props.message}"
        }
    }
}
