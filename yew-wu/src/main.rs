use chrono::{Datelike, NaiveDate};
use easy_cast::Cast;
use ecco::water_level_observations::WaterLevelObservations;
use gloo_console::log as gloo_log;
use js_sys::JsString;
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

#[derive(Debug, Clone)]
struct ObservationsModel {
    // try not to delete this. just init it once.
    observations: BTreeMap<NaiveDate, u32>,
    // use this as the date to reference in the charts
    start_date: NaiveDate,
    // use this as the date to reference in the charts
    end_date: NaiveDate,
}
impl<'a> ObservationsModel {
    pub fn generate_svg(observation_model: &ObservationsModel, svg_inner_string: &'a mut String) -> DrawResult<(), SVGBackend<'a>> {
        // TODO: use the parameter dates and corresponding values for the chart
        let _dates: Vec<NaiveDate> = observation_model.observations.keys().cloned().collect();
        let x_labels_amount = (observation_model.end_date.year() - observation_model.start_date.year()) as usize;
        // let start_date = dates.as_slice().first().unwrap();
        // let end_date = dates.as_slice().last().unwrap();
        //Goal get max and min value of btree:
        let date_range = Range {
            start: observation_model.start_date,
            end: observation_model.end_date
        };
        let ranged_date: RangedDate<NaiveDate> = date_range.clone().into();
        let values: Vec<u32> = observation_model
        .observations
        .range(date_range)
        .map(|(&_key, &value)| value)
        .collect();
        let y_max: f64 = (*values.iter().max().unwrap() as i64).cast();
        let y_min: f64 = (*values.iter().min().unwrap() as i64).cast();
        let _x_max = values.len() as f64;
        // let x_labels_amount = (date_range.end.year() - date_range.start.year()) as usize;
        // set up svg drawing area
        let size = (800u32, 600u32);
        let backend = SVGBackend::with_string(svg_inner_string, size);
        let backend_drawing_area = backend.into_drawing_area();
        backend_drawing_area.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&backend_drawing_area)
            .margin(20i32)
            .x_label_area_size(10u32)
            .y_label_area_size(10u32)
            .build_cartesian_2d(ranged_date, y_min..y_max)
            .unwrap();
            chart
            .configure_mesh()
            .x_labels(x_labels_amount)
            // .disable_x_mesh()
            // .disable_y_mesh()
            .draw()?;

        // populate the canvas with the data
        chart
            .draw_series(LineSeries::new(
                observation_model.observations.iter()
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
    // pub fn draw_wasm(om: &ObservationsModel, canvas: &mut HtmlCanvasElement, start_date: NaiveDate, end_date: NaiveDate) -> DrawResult<(), CanvasBackend> {
    //     // TODO: use the parameter dates and corresponding values for the chart
    //     let dates: Vec<NaiveDate> = om.observations.keys().cloned().collect();
    //     // let start_date = dates.as_slice().first().unwrap();
    //     // let end_date = dates.as_slice().last().unwrap();
    //     //Goal get max and min value of btree:
    //     let date_range = Range {
    //         start: start_date,
    //         end: end_date,
    //     };
    //     let ranged_date: RangedDate<NaiveDate> = date_range.into();
    //     let values = om.observations.values().cloned().collect::<Vec<u32>>();
    //     let y_max: f64 = (*values.iter().max().unwrap() as i64).cast();
    //     let y_min: f64 = (*values.iter().min().unwrap() as i64).cast();
    //     let _x_max = values.len() as f64;
    //     let x_labels_amount = (end_date.year() - start_date.year()) as usize;
    //     // setup chart
    //     // setup canvas drawing area
    //     let canvas_clone = canvas.clone();
    //     let backend = CanvasBackend::with_canvas_object(canvas_clone).unwrap();
    //     let backend_drawing_area = backend.into_drawing_area();
    //     backend_drawing_area.fill(&WHITE).unwrap();
    //     let mut chart = ChartBuilder::on(&backend_drawing_area)
    //         .margin(20i32)
    //         .x_label_area_size(10u32)
    //         .y_label_area_size(10u32)
    //         .build_cartesian_2d(ranged_date, y_min..y_max)
    //         .unwrap();
    //         chart
    //         .configure_mesh()
    //         .x_labels(x_labels_amount)
    //         // .disable_x_mesh()
    //         // .disable_y_mesh()
    //         .draw()?;

    //     // populate the canvas with the data
    //     chart
    //         .draw_series(LineSeries::new(
    //             om.observations.iter()
    //                 .map(|x| (*x.0, *x.1 as i32 as f64))
    //                 .collect::<Vec<_>>(),
    //             &RED,
    //         ))
    //         .unwrap()
    //         .label("water")
    //         .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    //     chart
    //         .configure_series_labels()
    //         .background_style(&WHITE.mix(0.8))
    //         .border_style(&BLACK)
    //         .draw()
    //         .unwrap();
    //     backend_drawing_area.present().unwrap();
    //     Ok(())
    // }
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
    fn create(_ctx: &Context<Self>) -> Self {
        let w = WaterLevelObservations::init_from_lzma();
        Self {
            observations: w.observations,
            start_date: w.start_date,
            end_date: w.end_date,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            DateChangeEvent::EndDateUpdated(new_end_date) =>{
                let end_date = self.end_date;
                if end_date == new_end_date {
                    false
                } else {
                    let log_string = format!("update EndDateUpdated: {} from {}", new_end_date.format(DATE_FORMAT), end_date.format(DATE_FORMAT));
                    string_log(log_string);
                    self.end_date = new_end_date;
                    true
                }
            },
            DateChangeEvent::StartDateUpdated(new_start_date) => {
                let start_date = self.start_date;
                if start_date == new_start_date {
                    false
                } else {
                    let log_string = format!("update EndDateUpdated: {} from {}", new_start_date.format(DATE_FORMAT), start_date.format(DATE_FORMAT));
                    string_log(log_string);
                    self.start_date = new_start_date;
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
        let start_date = self.start_date;
        let end_date = self.end_date;
        let mut svg_inner = String::new();
        let _svg_result = ObservationsModel::generate_svg(self, &mut svg_inner);
        let console_log = format!("{} {}", "SVG_INNER:", svg_inner);
        string_log(console_log);
        let svg_vnode = web_sys::window()
            .and_then(|window| window.document())
            .map_or_else(
                || {
                    html! { <p>{ "Failed to resolve `document`." }</p> }
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
        // let canvas_chart = web_sys::window()
        // .and_then(|window| window.document())
        // .map_or_else(
        //     || {
        //         html! { <p>{ "Failed to resolve `document`." }</p> }
        //     },
        //     |document| match document.get_element_by_id(ELEMENT_ID) {
        //         Some(canvas) => {
        //             let mut canvas_html_element = canvas.dyn_into::<HtmlCanvasElement>().unwrap();
        //             ObservationsModel::draw_wasm(self, &mut canvas_html_element, self.start_date, self.end_date);
        //             let html_element: &HtmlElement = canvas_html_element.deref();
        //             let element: &Element = html_element.deref();
        //             let node: &Node = element.deref();
        //             yew::virtual_dom::VNode::VRef(node.clone())

        //         }
        //         None => {
        //             // https://www.brightec.co.uk/blog/svg-wouldnt-render
        //             let canvas = document.create_element("canvas").unwrap();
        //             canvas.set_attribute("id", ELEMENT_ID);
        //             yew::virtual_dom::VNode::VRef(canvas.into())
        //         }
        //     },
        // );
        html! {
            <div id="chart">
                {svg_vnode}
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