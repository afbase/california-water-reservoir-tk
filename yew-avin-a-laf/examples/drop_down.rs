use ecco::calendar_year_model::CalendarYearModel;
use yew::prelude::*;
use cdec::reservoir::Reservoir::get_reservoir_vector;
extern crate yew_avin_a_laf;
use yew_avin_a_laf::reservoir_drop_down_list::{ReservoirDropDownList, reservoir_drop_down_list, ReservoirsDropDownProps};


// pub struct ReservoirsDropDownProps {
//     // see generic_callback
//     pub on_change: Fn(Event, &str) -> ReservoirSelectionEvent,
//     pub div_id: String,
//     pub select_id: String,
//     pub model: CalendarYearModel,
// }

pub fn generic_callback(_event: Event, dom_id_str: &str) -> ReservoirSelectionEvent {
    let updated_reservoir = web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_string = "window document object not found.".to_string();
                string_log(log_string);
                String::from("none")
            },
            |document| match document.get_element_by_id(dom_id_str) {
                Some(input) => {
                    let input_element = input.dyn_into::<HtmlSelectElement>().unwrap();
                    input_element.value()
                }
                None => {
                    let log_string = format!("{} {}", dom_id_str, "dom object not found.");
                    string_log(log_string);
                    String::from("none")
                }
            },
        );
    Msg::SelectReservoir(updated_reservoir)
}

fn main() {
    let model = CalendarYearModel::default();
    let cant_stop_giving_la_props = ReservoirsDropDownProps {
        div_id: String::from("yolo"),
        select_id: String::from("SHA"),
        model,
        on_change: generic_callback
    };
    web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_str = "failed to load wasm module successfully";
                let log_string = String::from(log_str);
                string_log(log_string);
                panic!("{}", log_str);
            },
            |document| match document.get_element_by_id(DIV_BLOG_NAME) {
                Some(_div_element) => {}
                None => {
                    let div_element = document.create_element("div").unwrap();
                    div_element.set_attribute("id", DIV_BLOG_NAME).unwrap();
                }
            },
        );
    let div_element = web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_str = "failed to load wasm module successfully part 2";
                let log_string = String::from(log_str);
                string_log(log_string);
                panic!("{}", log_str);
            },
            |document| match document.get_element_by_id(DIV_BLOG_NAME) {
                Some(div_element) => div_element,
                None => {
                    let log_str = "failed to load wasm module successfully part 3";
                    let log_string = String::from(log_str);
                    string_log(log_string);
                    panic!("{}", log_str);
                }
            },
        );
    let render = yew::Renderer::<ReservoirDropDownList>::with_root_and_props(div_element, cant_stop_giving_la_props);
    render.render();
}