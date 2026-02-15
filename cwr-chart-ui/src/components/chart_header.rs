//! Chart header component with title and Y-axis unit explanation.

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ChartHeaderProps {
    /// Chart title
    pub title: String,
    /// Y-axis unit explanation (e.g., "Acre-Feet (AF)")
    #[props(default = String::new())]
    pub unit_description: String,
}

/// Header for chart sections showing title and optional unit description.
#[component]
pub fn ChartHeader(props: ChartHeaderProps) -> Element {
    rsx! {
        div {
            style: "margin-bottom: 8px;",
            h3 {
                style: "margin: 0 0 4px 0; font-size: 16px;",
                "{props.title}"
            }
            if !props.unit_description.is_empty() {
                p {
                    style: "margin: 0; font-size: 12px; color: #666;",
                    "Y-axis: {props.unit_description}"
                }
            }
        }
    }
}
