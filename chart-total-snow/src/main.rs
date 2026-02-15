//! Total California Snow Water Equivalent Levels

use dioxus::prelude::*;

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("total-snow-root"))
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div { "Loading Total California Snow Water Equivalent Levels..." }
    }
}
