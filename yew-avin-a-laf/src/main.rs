#![feature(drain_filter)]
use cdec::{
    observable::{CompressedSurveyBuilder, InterpolateObservableRanges, ObservableRange},
    reservoir::Reservoir,
    survey::Survey,
};
// #![feature(map_first_last)]
use chrono::NaiveDate;
use ecco::reservoir_observations::ReservoirObservations;
use gloo_console::log as gloo_log;
// use itertools::Itertools;
use js_sys::JsString;
use plotters::prelude::*;
use std::{
    collections::HashMap,
    ops::Range,
};
use wasm_bindgen::JsCast;
use yew::prelude::*;

const DATE_FORMAT: &str = "%Y-%m-%d";
const END_DATE_NAME: &str = "end-date";
const START_DATE_NAME: &str = "start-date";
const DIV_END_DATE_NAME: &str = "div-end-date";
const DIV_START_DATE_NAME: &str = "div-start-date";
const _ELEMENT_ID: &str = "svg-chart";
const DIV_BLOG_NAME: &str = "california-chart";
const START_DATE_STRING: &str = "Start Date: ";
const END_DATE_STRING: &str = "End Date: ";
const DIV_RESERVOIR_SELECTION_ID: &str = "div-reservoir-selections";
const SELECT_RESERVOIR_TEXT: &str = "Select Reservoir: ";
const RESERVOIR_SELECTION_ID: &str = "reservoir-selections";

#[derive(Debug, Clone)]
struct ObservationsModel {
    // try not to delete this. just init it once.
    pub observations: HashMap<String, ReservoirObservations>,
    // The selected reservoir
    pub selected_reservoir: String,
    // the data for the selected reservoir
    pub selected_reservoir_data: Vec<Survey>,
    // use this as the date to reference in the charts
    pub start_date: NaiveDate,
    // use this as the date to reference in the charts
    pub end_date: NaiveDate,
    // use this date as the earliest date for the selected reservoir
    pub min_date: NaiveDate,
    // use this date as the latest date for the selected reservoir
    pub max_date: NaiveDate,
    // use this to get reservoir information
    pub reservoir_vector: Vec<Reservoir>,
}

pub enum CallbackChangeEvent {
    StartDateUpdated(NaiveDate),
    EndDateUpdated(NaiveDate),
    SelectReservoir(String),
    WindowDocumentFail,
    ReservoirSelectionFail,
    StartDateFail,
    EndDateFail,
    DomIdFail,
}

fn string_log(log_string: String) {
    let log_js_string: JsString = log_string.into();
    gloo_log!(log_js_string);
}

fn generic_callback(_event: Event, dom_id_str: &str) -> CallbackChangeEvent {
    web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_string = "window document object not found.".to_string();
                string_log(log_string);
                CallbackChangeEvent::WindowDocumentFail
            },
            |document| match dom_id_str {
                RESERVOIR_SELECTION_ID => match document.get_element_by_id(dom_id_str) {
                    Some(input) => {
                        let input_element = input.dyn_into::<web_sys::HtmlSelectElement>().unwrap();
                        CallbackChangeEvent::SelectReservoir(input_element.value())
                    }
                    None => {
                        let log_string = format!("{} {}", dom_id_str, "dom object not found.");
                        string_log(log_string);
                        CallbackChangeEvent::ReservoirSelectionFail
                    }
                },
                START_DATE_NAME => match document.get_element_by_id(dom_id_str) {
                    Some(input) => {
                        let input_element = input.dyn_into::<web_sys::HtmlInputElement>().unwrap();
                        let date_value: String = input_element.value();
                        let result = NaiveDate::parse_from_str(&date_value, DATE_FORMAT).unwrap();
                        let log_string = format!("callback: {}", result.format(DATE_FORMAT));
                        string_log(log_string);
                        CallbackChangeEvent::StartDateUpdated(result)
                    }
                    None => {
                        let log_string = format!("{} {}", dom_id_str, "dom object not found.");
                        string_log(log_string);
                        CallbackChangeEvent::StartDateFail
                    }
                },
                END_DATE_NAME => match document.get_element_by_id(dom_id_str) {
                    Some(input) => {
                        let input_element = input.dyn_into::<web_sys::HtmlInputElement>().unwrap();
                        let date_value: String = input_element.value();
                        let result = NaiveDate::parse_from_str(&date_value, DATE_FORMAT).unwrap();
                        let log_string = format!("callback: {}", result.format(DATE_FORMAT));
                        string_log(log_string);
                        CallbackChangeEvent::EndDateUpdated(result)
                    }
                    None => {
                        let log_string = format!("{} {}", dom_id_str, "dom object not found.");
                        string_log(log_string);
                        CallbackChangeEvent::EndDateFail
                    }
                },
                _ => {
                    CallbackChangeEvent::DomIdFail
                }
            },
        )
}

impl<'a> ObservationsModel {
    fn interpolate_data_for_selected_reservoir(&mut self) {
        // interpolate all data and then select the data with the date range
        let mut observable_range = ObservableRange::new(self.min_date, self.max_date);
        observable_range.observations = self.selected_reservoir_data.clone();
        let mut vec_observable_range: Vec<ObservableRange> = vec![observable_range];
        vec_observable_range.interpolate_reservoir_observations();
        if let Some(observable_range) = vec_observable_range.first_mut() {
            self.selected_reservoir_data = observable_range
                .observations
                .drain_filter(|survey| {
                    let date_observation = survey.get_tap().date_observation;
                    self.start_date <= date_observation && date_observation <= self.end_date
                })
                .collect::<Vec<_>>();
        };
    }

    pub fn generate_svg(
        observation_model: &ObservationsModel,
        svg_inner_string: &'a mut String,
    ) -> DrawResult<(), SVGBackend<'a>> {
        // Need to get selected data as svg
        let date_range = Range {
            start: observation_model.start_date,
            end: observation_model.end_date,
        };
        let ranged_date: RangedDate<NaiveDate> = date_range.into();
        let mut values = observation_model
            .selected_reservoir_data
            .iter()
            .map(|survey| {
                let date = survey.get_tap().date_observation;
                let value = survey.get_tap().value_as_f64();
                (date, value)
            })
            .collect::<Vec<(NaiveDate, f64)>>();
        values.sort_by(|a, b| {
                let a_date = a.0;
                let b_date = b.0;
                a_date.partial_cmp(&b_date).unwrap()
            });
        let y_max: f64 = {
            let mut tmp: f64 = values
                .iter()
                .map(|point| point.1)
                .max_by(|a, b| a.total_cmp(b))
                .unwrap();
            if tmp > 500000.0 {
                tmp += 500000.0;
            } else {
                tmp += tmp / 5.0;
            }
            tmp
        };
        // set up svg drawing area
        let size = (800u32, 600u32);
        let backend = SVGBackend::with_string(svg_inner_string, size);
        let backend_drawing_area = backend.into_drawing_area();
        backend_drawing_area.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&backend_drawing_area)
            .margin(20i32)
            .x_label_area_size(20u32)
            .y_label_area_size(40u32)
            .build_cartesian_2d(ranged_date, 0f64..y_max)
            .unwrap();
        chart.configure_mesh().x_labels(10_usize).draw()?;

        // populate the canvas with the data
        chart
            .draw_series(LineSeries::new(values, RED))
            .unwrap()
            .label(observation_model.selected_reservoir.clone())
            .legend(|(x,y)| Rectangle::new([(x - 15, y + 1), (x, y)], RED));
            // .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .margin(20)
            .background_style(WHITE.mix(0.1))
            .border_style(BLACK.mix(0.7))
            .draw()
            .unwrap();
        backend_drawing_area.present().unwrap();
        Ok(())
    }
}

impl Component for ObservationsModel {
    type Message = CallbackChangeEvent;
    type Properties = ();
    fn create(_ctx: &Context<Self>) -> Self {
        let reservoir_vector = Reservoir::get_reservoir_vector();
        let observations = ReservoirObservations::init_from_lzma_without_interpolation();
        let selected_reservoir = String::from("ORO");
        if let Some(selected_reservoir_observations) = observations.get(&selected_reservoir) {
            let (start_date, end_date) = (
                selected_reservoir_observations.start_date,
                selected_reservoir_observations.end_date,
            );
            let selected_reservoir_data = selected_reservoir_observations.observations.clone();
            let mut active_model = Self {
                observations,
                selected_reservoir,
                selected_reservoir_data,
                start_date,
                end_date,
                min_date: start_date,
                max_date: end_date,
                reservoir_vector,
            };
            active_model.interpolate_data_for_selected_reservoir();
            return active_model;
        }
        // if we get to this point, we've failed
        let log_string = format!("Failed to get data for selected reservoir {selected_reservoir}");
        string_log(log_string.clone());
        panic!("{}", log_string);
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            CallbackChangeEvent::WindowDocumentFail
            | CallbackChangeEvent::DomIdFail
            | CallbackChangeEvent::EndDateFail
            | CallbackChangeEvent::StartDateFail
            | CallbackChangeEvent::ReservoirSelectionFail => false,
            CallbackChangeEvent::SelectReservoir(selected_reservoir) => {
                // Set the selected reservoir and fetch the data for that reservoir
                let mut reversed = selected_reservoir.chars().rev().collect::<String>();
                reversed.truncate(3);
                let station_id = reversed.chars().rev().collect::<String>();
                if let Some(selected_reservoir_observations) = self.observations.get(&station_id) {
                    let (start_date, end_date) = (
                        selected_reservoir_observations.start_date,
                        selected_reservoir_observations.end_date,
                    );
                    self.selected_reservoir = station_id;
                    self.start_date = start_date;
                    self.min_date = start_date;
                    self.end_date = end_date;
                    self.max_date = end_date;
                    self.interpolate_data_for_selected_reservoir();
                }
                true
            }
            CallbackChangeEvent::EndDateUpdated(new_end_date) => {
                let end_date = self.end_date;
                if end_date == new_end_date {
                    false
                } else {
                    if self.start_date <= new_end_date && new_end_date <= self.max_date {
                        let log_string = format!(
                            "update EndDateUpdated: {} from {}",
                            new_end_date.format(DATE_FORMAT),
                            end_date.format(DATE_FORMAT)
                        );
                        string_log(log_string);
                        self.end_date = new_end_date;
                    } else if self.min_date <= new_end_date {
                        let log_string = format!(
                            "update EndDateUpdated: {} from {}; reset start date to min",
                            new_end_date.format(DATE_FORMAT),
                            end_date.format(DATE_FORMAT)
                        );
                        string_log(log_string);
                        self.start_date = self.min_date;
                        self.end_date = new_end_date;
                    }
                    true
                }
            }
            CallbackChangeEvent::StartDateUpdated(new_start_date) => {
                let start_date = self.start_date;
                if start_date == new_start_date {
                    false
                } else {
                    if self.min_date <= new_start_date && new_start_date <= self.end_date {
                        let log_string = format!(
                            "update StartDateUpdated: {} from {}",
                            new_start_date.format(DATE_FORMAT),
                            start_date.format(DATE_FORMAT)
                        );
                        string_log(log_string);
                        self.start_date = new_start_date;
                    } else if new_start_date <= self.max_date {
                        let log_string = format!(
                            "update StartDateUpdated: {} from {}; reset end date to max",
                            new_start_date.format(DATE_FORMAT),
                            start_date.format(DATE_FORMAT)
                        );
                        string_log(log_string);
                        self.start_date = new_start_date;
                        self.end_date = self.max_date;
                    }
                    true
                }
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let reservoir_selection_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, RESERVOIR_SELECTION_ID));
        let start_date_change_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, START_DATE_NAME));
        let end_date_change_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, END_DATE_NAME));
        let start_date = self.start_date;
        let end_date = self.end_date;
        let mut reservoir_ids_sorted = self.observations.keys().cloned().collect::<Vec<_>>();
        reservoir_ids_sorted.sort();
        let mut svg_inner = String::new();
        let _svg_result = ObservationsModel::generate_svg(self, &mut svg_inner);
        let svg_vnode = web_sys::window()
            .and_then(|window| window.document())
            .map_or_else(
                || {
                    html! { <p id="error">{ "Failed to resolve `document`." }</p> }
                },
                |document| match document.get_element_by_id("svg-chart") {
                    Some(svg) => {
                        svg.set_inner_html(svg_inner.as_str());
                        yew::virtual_dom::VNode::VRef(svg.into())
                    }
                    None => {
                        // https://www.brightec.co.uk/blog/svg-wouldnt-render
                        let svg = document
                            .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
                            .unwrap();
                        svg.set_attribute("id", "svg-chart").unwrap();
                        svg.set_attribute("width", "800").unwrap();
                        svg.set_attribute("height", "600").unwrap();
                        svg.set_inner_html(svg_inner.as_str());
                        yew::virtual_dom::VNode::VRef(svg.into())
                    }
                },
            );
        html! {
            <div id="chart">
                <div id={DIV_START_DATE_NAME}>
                    {START_DATE_STRING} <input min={self.min_date.format(DATE_FORMAT).to_string()} max={self.max_date.format(DATE_FORMAT).to_string()} onchange={start_date_change_callback} type="date" id={START_DATE_NAME} value={start_date.format(DATE_FORMAT).to_string()}/>
                </div>
                <div id={DIV_END_DATE_NAME}>
                    {END_DATE_STRING} <input min={self.min_date.format(DATE_FORMAT).to_string()} max={self.max_date.format(DATE_FORMAT).to_string()} onchange={end_date_change_callback} type="date" id={END_DATE_NAME} value={end_date.format(DATE_FORMAT).to_string()}/>
                </div>
                <div id={DIV_RESERVOIR_SELECTION_ID}>
                // Dropdown list for selecting a reservoir
                {SELECT_RESERVOIR_TEXT}
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
                </div>
                {svg_vnode}
            </div>
        }
    }
}

fn main() {
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
    let renderer = yew::Renderer::<ObservationsModel>::with_root(div_element);
    renderer.render();
}
