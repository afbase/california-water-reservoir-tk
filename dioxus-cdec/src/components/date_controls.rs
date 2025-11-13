use dioxus::prelude::*;

#[component]
pub fn DateControls(
    start_date: String,
    end_date: String,
    min_date: String,
    max_date: String,
    on_start_change: EventHandler<String>,
    on_end_change: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: "controls",
            style: "display: flex; gap: 20px; margin: 20px 0; justify-content: center; align-items: center;",

            div {
                label {
                    style: "margin-right: 10px; font-weight: 500; color: #555;",
                    "Start Date:"
                }
                input {
                    r#type: "date",
                    value: "{start_date}",
                    min: "{min_date}",
                    max: "{end_date}",
                    style: "padding: 8px 12px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px;",
                    oninput: move |evt| {
                        on_start_change.call(evt.value().clone());
                    }
                }
            }

            div {
                label {
                    style: "margin-right: 10px; font-weight: 500; color: #555;",
                    "End Date:"
                }
                input {
                    r#type: "date",
                    value: "{end_date}",
                    min: "{start_date}",
                    max: "{max_date}",
                    style: "padding: 8px 12px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px;",
                    oninput: move |evt| {
                        on_end_change.call(evt.value().clone());
                    }
                }
            }
        }
    }
}
