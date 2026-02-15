//! Chart container component with loading state.

use dioxus::prelude::*;

/// Props for ChartContainer
#[derive(Props, Clone, PartialEq)]
pub struct ChartContainerProps {
    /// The DOM id for the chart container (D3 will render into this)
    pub id: String,
    /// Whether the chart is still loading
    #[props(default = false)]
    pub loading: bool,
    /// Optional minimum height in pixels
    #[props(default = 400)]
    pub min_height: u32,
}

/// A container div for D3.js charts with loading overlay.
#[component]
pub fn ChartContainer(props: ChartContainerProps) -> Element {
    let style = format!(
        "min-height: {}px; position: relative; width: 100%;",
        props.min_height
    );

    rsx! {
        div {
            style: "{style}",
            if props.loading {
                div {
                    style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); color: #666;",
                    "Loading chart..."
                }
            }
            div {
                id: "{props.id}",
                style: "width: 100%;",
            }
        }
    }
}
