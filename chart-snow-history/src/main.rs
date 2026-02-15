//! Historical Snow Water Equivalent by Sensor

use dioxus::prelude::*;

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname("snow-history-root"))
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div { "Loading Historical Snow Water Equivalent by Sensor..." }
    }
}
