use chrono::NaiveDate;
use easy_cast::Cast;
use ecco::water_level_observations::WaterLevelObservations;
use log::{info, LevelFilter};
use my_log::MY_LOGGER;
use plotters::prelude::*;
use std::{collections::BTreeMap, ops::Range};
use wasm_bindgen::JsCast;
use yew::prelude::*;

const DATE_FORMAT: &str = "%Y-%m-%d";
const END_DATE_NAME: &str = "end-date";
const START_DATE_NAME: &str = "start-date";
const DIV_END_DATE_NAME: &str = "div-end-date";
const DIV_START_DATE_NAME: &str = "div-start-date";
const _ELEMENT_ID: &str = "svg-chart";
const DIV_BLOG_NAME: &str = "yew-wu";
const START_DATE_STRING: &str = "Start Date: ";
const END_DATE_STRING: &str = "End Date: ";

#[derive(Debug, Clone)]
struct ObservationsModel {
    // try not to delete this. just init it once.
    observations: BTreeMap<NaiveDate, u32>,
    // use this as the date to reference in the charts
    start_date: NaiveDate,
    // use this as the date to reference in the charts
    end_date: NaiveDate,
    // use this date as the earliest date in observations
    min_date: NaiveDate,
    // use this date as the latest date in observations
    max_date: NaiveDate,
}

pub enum DateChangeEvent {
    StartDateUpdated(NaiveDate),
    EndDateUpdated(NaiveDate),
}

fn generic_callback(_event: Event, event_is_end: bool, dom_id_str: &str) -> DateChangeEvent {
    let updated_date = web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_string = "window document object not found.".to_string();
                info!("{}", log_string);
                NaiveDate::from_ymd_opt(1992, 3, 26).unwrap()
            },
            |document| match document.get_element_by_id(dom_id_str) {
                Some(input) => {
                    let input_element = input.dyn_into::<web_sys::HtmlInputElement>().unwrap();
                    let date_value: String = input_element.value();
                    let result = NaiveDate::parse_from_str(&date_value, DATE_FORMAT).unwrap();
                    let log_string = format!("callback: {}", result.format(DATE_FORMAT));
                    info!("{}", log_string);
                    result
                }
                None => {
                    let log_string = format!("{} {}", dom_id_str, "dom object not found.");
                    info!("{}", log_string);
                    NaiveDate::from_ymd_opt(1999, 1, 1).unwrap()
                }
            },
        );
    if event_is_end {
        DateChangeEvent::EndDateUpdated(updated_date)
    } else {
        DateChangeEvent::StartDateUpdated(updated_date)
    }
}

impl<'a> ObservationsModel {
    pub fn generate_svg(
        observation_model: &ObservationsModel,
        svg_inner_string: &'a mut String,
    ) -> DrawResult<(), SVGBackend<'a>> {
        // TODO: use the parameter dates and corresponding values for the chart
        let _dates: Vec<NaiveDate> = observation_model.observations.keys().cloned().collect();
        // let x_labels_amount =
        //     (observation_model.end_date.year() - observation_model.start_date.year()) as usize;
        //Goal get max and min value of btree:
        let date_range = Range {
            start: observation_model.start_date,
            end: observation_model.end_date,
        };
        let ranged_date: RangedDate<NaiveDate> = date_range.clone().into();
        let values: Vec<u32> = observation_model
            .observations
            .range(date_range)
            .map(|(&_key, &value)| value)
            .collect();
        let y_max: f64 = ((*values.iter().max().unwrap() + 500000) as i64).cast();
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
            .draw_series(LineSeries::new(
                observation_model
                    .observations
                    .iter()
                    .map(|x| (*x.0, *x.1 as i32 as f64))
                    .collect::<Vec<_>>(),
                RED,
            ))
            .unwrap()
            .label("water")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()
            .unwrap();
        backend_drawing_area.present().unwrap();
        Ok(())
    }
}

impl Component for ObservationsModel {
    type Message = DateChangeEvent;
    type Properties = ();
    fn create(_ctx: &Context<Self>) -> Self {
        let w = WaterLevelObservations::init_from_lzma_v2();
        let log_string = format!(
            "oldest date: {}\nnewest date: {}",
            w.min_date.format(DATE_FORMAT),
            w.max_date.format(DATE_FORMAT)
        );
        info!("{}", log_string);
        Self {
            observations: w.observations,
            start_date: w.start_date,
            end_date: w.end_date,
            max_date: w.max_date,
            min_date: w.min_date,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            DateChangeEvent::EndDateUpdated(new_end_date) => {
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
                        info!("{}", log_string);
                        self.end_date = new_end_date;
                    } else if self.min_date <= new_end_date {
                        let log_string = format!(
                            "update EndDateUpdated: {} from {}; reset start date to min",
                            new_end_date.format(DATE_FORMAT),
                            end_date.format(DATE_FORMAT)
                        );
                        info!("{}", log_string);
                        self.start_date = self.min_date;
                        self.end_date = new_end_date;
                    }
                    true
                }
            }
            DateChangeEvent::StartDateUpdated(new_start_date) => {
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
                        info!("{}", log_string);
                        self.start_date = new_start_date;
                    } else if new_start_date <= self.max_date {
                        let log_string = format!(
                            "update StartDateUpdated: {} from {}; reset end date to max",
                            new_start_date.format(DATE_FORMAT),
                            start_date.format(DATE_FORMAT)
                        );
                        info!("{}", log_string);
                        self.start_date = new_start_date;
                        self.end_date = self.max_date;
                    }
                    true
                }
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let start_date_change_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, false, START_DATE_NAME));
        let end_date_change_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, true, END_DATE_NAME));
        let start_date = self.start_date;
        let end_date = self.end_date;
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
                        svg.set_attribute("width", "850").unwrap();
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
                {svg_vnode}
            </div>
        }
    }
}

fn main() {
    log::set_logger(&MY_LOGGER).unwrap();
    log::set_max_level(LevelFilter::Info);
    web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_str = "failed to load wasm module successfully";
                let log_string = String::from(log_str);
                info!("{}", log_string);
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
                info!("{}", log_string);
                panic!("{}", log_str);
            },
            |document| match document.get_element_by_id(DIV_BLOG_NAME) {
                Some(div_element) => div_element,
                None => {
                    let log_str = "failed to load wasm module successfully part 3";
                    let log_string = String::from(log_str);
                    info!("{}", log_string);
                    panic!("{}", log_str);
                }
            },
        );
    let renderer = yew::Renderer::<ObservationsModel>::with_root(div_element);
    renderer.render();
}
