// use model::{Model, DIV_BLOG_NAME, string_log};
mod model;
use crate::model::{DIV_BLOG_NAME, string_log, generic_callback, Model, RESERVOIR_SELECTION_ID, NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT};

fn main() {
    web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_str = "failed to load wasm module successfully part 1";
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
    let renderer = yew::Renderer::<Model>::with_root(div_element);
    renderer.render();
}
