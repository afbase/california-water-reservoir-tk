//! Container the D3 scripts render a chart or table into.

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ChartContainerProps {
    /// The DOM id D3 renders into.
    pub id: String,
    /// Minimum height in pixels, reserved so the page does not jump when the chart appears.
    #[props(default = 400)]
    pub min_height: u32,
}

#[component]
pub fn ChartContainer(props: ChartContainerProps) -> Element {
    rsx! {
        div {
            class: "cwr-chart-wrap",
            style: "min-height: {props.min_height}px;",
            div { id: "{props.id}", class: "cwr-chart" }
        }
    }
}
