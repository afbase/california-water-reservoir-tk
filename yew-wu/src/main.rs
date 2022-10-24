use chrono::{NaiveDate};
use ecco::water_level_observations::WaterLevelObservations;
use gloo_console::log as gloo_log;
use js_sys::JsString;
use std::collections::BTreeMap;
use wasm_bindgen::JsCast;
use yew::prelude::*;


const DATE_FORMAT: &str = "%Y-%m-%d";
const END_DATE_NAME: &str = "end-date";
const START_DATE_NAME: &str = "start-date";
const DIV_END_DATE_NAME: &str = "div-end-date";
const DIV_START_DATE_NAME: &str = "div-start-date";
const ELEMENT_ID: &str = "canvas-chart";

struct ObservationsModel {
    // try not to delete this. just init it once.
    observations: WaterLevelObservations,
    // use this as the date to reference in the charts
    start_date: NaiveDate,
    // use this as the date to reference in the charts
    end_date: NaiveDate,
}

fn string_log(log_string: String) {
    let log_js_string: JsString = log_string.into();
    gloo_log!(log_js_string);
}

fn generic_callback(_event: Event, event_is_end: bool, dom_id_str: &str) -> DateChangeEvent {
    let updated_date =    
    web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_string = "window document object not found.".to_string();
                string_log(log_string);
                NaiveDate::from_ymd(1992,3,26)
            },
            |document| match document.get_element_by_id(dom_id_str) {
                Some(input) => {
                    let input_element = input.dyn_into::<web_sys::HtmlInputElement>().unwrap();
                    let date_value: String = input_element.value();
                    let result = NaiveDate::parse_from_str(&date_value, DATE_FORMAT).unwrap();
                    let log_string = format!("callback: {}", result.format(DATE_FORMAT));
                    string_log(log_string);
                    result
                },
                None => {
                    let log_string = format!("{} {}", dom_id_str, "dom object not found.");
                    string_log(log_string);
                    NaiveDate::from_ymd(1999,1,1)
                }
            },
        );
        if event_is_end {
            DateChangeEvent::EndDateUpdated(updated_date)
        } else {
            DateChangeEvent::StartDateUpdated(updated_date)
        }
}

pub enum DateChangeEvent {
    StartDateUpdated(NaiveDate),
    EndDateUpdated(NaiveDate),
}

fn main() {
    yew::start_app::<ObservationsModel>();
}

impl Component for ObservationsModel {
    type Message = DateChangeEvent;
    type Properties = ();
    fn create(ctx: &Context<Self>) -> Self {
        let observations = WaterLevelObservations::init_from_lzma();
        let start_date = observations.first_entry().unwrap().0;
        let end_date = observations.last_entry().unwrap().0;
        Self {
            observations,
            start_date,
            end_date
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            DateChangeEvent::EndDateUpdated(new_end_date) =>{
                let end_date = self.end_date
                if *end_date == new_end_date {
                    false
                } else {
                    let log_string = format!("update EndDateUpdated: {} from {}", new_end_date.format(DATE_FORMAT), end_date.format(DATE_FORMAT));
                    string_log(log_string);
                    self.update_end_date(new_end_date);
                    true
                }
            },
            DateChangeEvent::StartDateUpdated(new_start_date) => {
                let start_date = self.start_date
                if *start_date == new_start_date {
                    false
                } else {
                    let log_string = format!("update EndDateUpdated: {} from {}", new_start_date.format(DATE_FORMAT), start_date.format(DATE_FORMAT));
                    string_log(log_string);
                    self.update_end_date(new_end_date);
                    true
                }
            }
        }
    }
    
    fn view(&self, ctx: &Context<Self>) -> Html {
        let start_date_change_callback = ctx.link().callback(|event: Event| 
            generic_callback(event, false, START_DATE_NAME));
        let end_date_change_callback = ctx.link().callback(|event: Event| 
            generic_callback(event, true, END_DATE_NAME));
        let start_date = self.observations.first_key_value().unwrap().0;
        let end_date = self.observations.last_key_value().unwrap().0;
        let canvas_chart = web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                html! { <p>{ "Failed to resolve `document`." }</p> }
            },
            |document| match document.get_element_by_id(ELEMENT_ID) {
                Some(canvas) => {
                    self.draw_wasm(canvas);
                    yew::virtual_dom::VNode::VRef(canvas)
                }
                None => {
                    // https://www.brightec.co.uk/blog/svg-wouldnt-render
                    let canvas = document.create_element("canvas").unwrap();
                    canvas.set_attribute("id", ELEMENT_ID);
                    yew::virtual_dom::VNode::VRef(canvas)
                }
            },
        );
        html! {
            <div id="chart">
                {canvas_chart}
                <div id={DIV_START_DATE_NAME}>
                    <input onchange={start_date_change_callback} type="date" id={START_DATE_NAME} value={start_date.format(DATE_FORMAT).to_string()}/>
                </div>
                <div id={DIV_END_DATE_NAME}>
                    <input onchange={end_date_change_callback} type="date" id={END_DATE_NAME} value={end_date.format(DATE_FORMAT).to_string()}/>
                </div>
            </div>
        }
    }
}