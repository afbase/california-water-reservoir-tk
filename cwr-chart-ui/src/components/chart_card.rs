//! The themed card every chart app renders into.

use crate::js_bridge;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ChartCardProps {
    /// Card title.
    pub title: String,
    /// Optional explanation of the plotted unit (e.g. "Acre-Feet (AF) - ...").
    #[props(default = String::new())]
    pub unit_description: String,
    children: Element,
}

/// Opaque, themed surface that wraps a chart app.
///
/// Mounting it installs the shared theme stylesheet and D3 chart scripts (see
/// [`js_bridge::init_charts`]), so the chart is readable regardless of the host
/// page's own colors.
#[component]
pub fn ChartCard(props: ChartCardProps) -> Element {
    use_hook(js_bridge::init_charts);

    rsx! {
        div {
            class: "cwr-card",
            div {
                class: "cwr-header",
                h3 { class: "cwr-title", "{props.title}" }
                if !props.unit_description.is_empty() {
                    p { class: "cwr-unit", "{props.unit_description}" }
                }
            }
            {props.children}
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct CalloutProps {
    children: Element,
}

/// A boxed footnote below a chart.
#[component]
pub fn Callout(props: CalloutProps) -> Element {
    rsx! {
        div { class: "cwr-callout", {props.children} }
    }
}
