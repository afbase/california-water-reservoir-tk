use cdec::{
    reservoir::Reservoir,
    water_year::{WaterYear, WaterYearStatistics},
};
use ecco::reservoir_observations::{GetWaterYears, ReservoirObservations};
use gloo_console::log as gloo_log;
use js_sys::JsString;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;
use yew::prelude::*;
const DIV_BLOG_NAME: &str = "california-table";
const RESERVOIR_SELECTION_ID: &str = "reservoir-selections";
struct Model {
    // The selected reservoir
    selected_reservoir: String,
    // The data for the selected reservoir
    reservoir_data: HashMap<String, Vec<WaterYear>>,
    reservoir_vector: Vec<Reservoir>,
}

enum Msg {
    // The user selected a reservoir from the dropdown list
    SelectReservoir(String),
}

fn string_log(log_string: String) {
    let log_js_string: JsString = log_string.into();
    gloo_log!(log_js_string);
}

// TODO fix this so it is not about dates but reservoir ids
fn generic_callback(_event: Event, dom_id_str: &str) -> Msg {
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

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let reservoirs = Reservoir::get_reservoir_vector();
        let observations_hash_map: HashMap<String, ReservoirObservations> =
            ReservoirObservations::init_from_lzma();
        let water_years_from_observable_range =
            observations_hash_map.get_water_years_from_reservoir_observations();
        Self {
            reservoir_data: water_years_from_observable_range,
            selected_reservoir: String::from("SHA"),
            reservoir_vector: reservoirs,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            // The user selected a reservoir from the dropdown list
            Msg::SelectReservoir(reservoir) => {
                // Set the selected reservoir and fetch the data for that reservoir
                let mut reversed = reservoir.chars().rev().collect::<String>();
                reversed.truncate(3);
                let station_id = reversed.chars().rev().collect::<String>();
                self.selected_reservoir = station_id;
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let reservoir_selection_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, RESERVOIR_SELECTION_ID));
        if let Some((_key, water_years)) =
            self.reservoir_data.get_key_value(&self.selected_reservoir)
        {
            let mut water_statistics = water_years
                .iter()
                .map(|water_year| water_year.into())
                .collect::<Vec<WaterYearStatistics>>();
            water_statistics.sort();
            let mut reservoir_ids_sorted = self
                .reservoir_data
                .keys()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();
            reservoir_ids_sorted.sort();

            html! {
                <div>
                    // Dropdown list for selecting a reservoir
                    <select id={RESERVOIR_SELECTION_ID} onchange={reservoir_selection_callback}>
                    { for
                        reservoir_ids_sorted.iter().map(|station_id| {
                            let station_id_value = station_id.clone();
                            let station_id_option = station_id.clone();
                            let reservoir = self.reservoir_vector.iter().find_map(|resy|
                                {
                                    let mut result = None;
                                    let reservoir_station_id = resy.station_id.clone();
                                    let station_id_cloned = station_id.clone();
                                    if reservoir_station_id == station_id_cloned {
                                        result = Some(resy.clone());
                                    }
                                    result
                                }).unwrap();
                            let option_text = format!("{} - {}", reservoir.dam, station_id_option);
                            if *station_id == self.selected_reservoir {
                                    html!{
                                        <option value={station_id_value} selected=true>{option_text}</option>
                                    }
                                } else {
                                    html!{
                                        <option value={station_id_value}>{option_text}</option>
                                    }
                                }

                        })
                    }
                    </select>
                    // Table showing the data for the selected reservoir
                    <table class="table table-striped">
                        <thead>
                            <tr>
                                <th>{"Water Calendar Year"}</th>
                                <th>{"Date of Lowest"}</th>
                                <th>{"Lowest (Acrefeet)"}</th>
                                <th>{"Date of Highest"}</th>
                                <th>{"Highest (Acrefeet)"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            // Iterate over the data for the selected reservoir and create a row for each entry
                            { for water_statistics.iter().map(|data| {
                                let integer: u32 = data.lowest_value as u32;
                                let calendar_year = &data.year;
                                let calendar_year_plus_plus = (calendar_year + 1).to_string();
                                let calendar_year_plus_1_str = calendar_year_plus_plus.as_str();
                                let two_digit = &calendar_year_plus_1_str[2..];
                                let calendar_year_str = format!("{calendar_year}-{two_digit}");
                                match (integer, *calendar_year) {
                                    (0u32, 1976) => {
                                        html! {
                                            <tr class="table-danger">
                                                <th scope="row">{calendar_year_str}</th>
                                                <td>{&data.date_lowest}</td>
                                                <td>{&data.lowest_value}</td>
                                                <td>{&data.date_highest}</td>
                                                <td>{&data.highest_value}</td>
                                            </tr>
                                        }
                                    },
                                    (0u32, 1977) => {
                                        html! {
                                            <tr class="table-danger">
                                                <th scope="row">{calendar_year_str}</th>
                                                <td>{&data.date_lowest}</td>
                                                <td>{&data.lowest_value}</td>
                                                <td>{&data.date_highest}</td>
                                                <td>{&data.highest_value}</td>
                                            </tr>
                                        }
                                    },
                                    (0u32, _) => {
                                        html! {
                                            <tr class="table-warning">
                                                <th scope="row">{calendar_year_str}</th>
                                                <td>{&data.date_lowest}</td>
                                                <td>{&data.lowest_value}</td>
                                                <td>{&data.date_highest}</td>
                                                <td>{&data.highest_value}</td>
                                            </tr>
                                        }
                                    },
                                    (_, _) => {
                                        html! {
                                            <tr>
                                                <th scope="row">{calendar_year_str}</th>
                                                <td>{&data.date_lowest}</td>
                                                <td>{&data.lowest_value}</td>
                                                <td>{&data.date_highest}</td>
                                                <td>{&data.highest_value}</td>
                                            </tr>
                                        }
                                    }
                                }
                            }
                        )}
                        </tbody>
                    </table>
                </div>
            }
        } else {
            html! {}
        }
    }
}

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
