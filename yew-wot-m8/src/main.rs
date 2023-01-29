use cdec::{
    normalized_naive_date::NormalizedNaiveDate,
    reservoir::Reservoir,
    water_year::{NormalizeWaterYears, WaterYear, WaterYearStatistics},
};
use chrono::{Datelike, NaiveDate};
// use chrono::{DateTime, Duration, IsoWeek, Local, NaiveDate, Weekday};
// use easy_cast::traits::Cast;
use ecco::{
    calendar_year_model::get_colors,
    reservoir_observations::{GetWaterYears, ReservoirObservations},
};
use gloo_console::log as gloo_log;
use js_sys::JsString;
use plotters::prelude::*;
use std::collections::HashMap;
use std::ops::Range;
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

const DIV_SORT_BY_SELECTION_ID: &str = "div-select-sort-by";
pub const DIV_BLOG_NAME: &str = "california-years";
pub const DIV_RESERVOIR_SELECTION_ID: &str = "div-reservoir-selections";
const _ELEMENT_ID: &str = "svg-chart";
const MOST_RECENT: &str = "Most Recent";
const DRIEST: &str = "Driest";
const DRIEST_OPTION_TEXT: &str = "Sort By Driest";
const MOST_RECENT_OPTION_TEXT: &str = "Sort By Most Recent";
const SORT_BY_SELECTION_ID: &str = "select-sort-by";
const SELECT_RESERVOIR_TEXT: &str = "Select Reservoir: ";
const SORT_BY_TEXT: &str = "Sort by: ";
pub const RESERVOIR_SELECTION_ID: &str = "reservoir-selections";
pub const NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT: usize = 20;

fn string_log(log_string: String) {
    let log_js_string: JsString = log_string.into();
    gloo_log!(log_js_string);
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

#[derive(Debug)]
pub enum SortBy {
    MostRecent,
    DriestYears,
}

#[derive(Debug)]
pub enum Msg {
    // The user selected a reservoir from the dropdown list
    SelectReservoir(String),
    SelectedSort(SortBy),
}

pub struct Model {
    // The selected reservoir
    pub selected_reservoir: String,
    pub selected_sort: Msg,
    // The data for the selected reservoir
    pub reservoir_data: HashMap<String, Vec<WaterYear>>,
    pub reservoir_vector: Vec<Reservoir>,
}

impl<'a> Model {
    fn derive_legend_name(&self) -> String {
        let data = self.reservoir_data.get(&self.selected_reservoir).unwrap();
        let station_id = data[0].clone().0[0].tap().station_id.clone();
        let reservoir = self
            .reservoir_vector
            .iter()
            .find_map(|resy| {
                let mut result = None;
                let reservoir_station_id = resy.station_id.clone();
                let station_id_cloned = station_id.clone();
                if reservoir_station_id == station_id_cloned {
                    result = Some(resy.clone());
                }
                result
            })
            .unwrap();
        format!("{} - {}", reservoir.dam, station_id)
    }
    pub fn generate_svg(&self, svg_inner_string: &'a mut String) -> DrawResult<(), SVGBackend<'a>> {
        let legend_base = self.derive_legend_name();
        if let Some(mut normalized_water_years) = {
            let test = self.reservoir_data.get(&self.selected_reservoir);
            test.map(|selected_reservoir_data| {
                let result = selected_reservoir_data.get_complete_normalized_water_years();
                result
            })
        } {
            let date_range_tuple = NormalizedNaiveDate::get_normalized_tuple_date_range();
            let range_date = Range {
                start: date_range_tuple.0,
                end: date_range_tuple.1,
            };
            let ranged_date: RangedDate<NaiveDate> = range_date.into();
            let log_string = format!("selected sort: {:?}", self.selected_sort);
            string_log(log_string);
            match self.selected_sort {
                Msg::SelectedSort(SortBy::DriestYears) => {
                    normalized_water_years.sort_by_lowest_recorded_years()
                }
                Msg::SelectedSort(SortBy::MostRecent) => {
                    normalized_water_years.sort_by_most_recent()
                }
                // the most recent seems to be the more climate science-y method
                _ => normalized_water_years.sort_by_most_recent(),
            }
            let y_max = normalized_water_years
                .get_largest_acrefeet_over_n_years(NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT)
                .unwrap();
            let colors_for_water_years = get_colors(NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT).unwrap();
            let plot_and_color = normalized_water_years
                .iter()
                .zip(colors_for_water_years.iter());
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
            for (water_year, rgb_color) in plot_and_color {
                // date_recording is the original date in normalization
                let (first, last) = water_year.calendar_year_from_normalized_water_year();
                let year_string = format!("{}-{}", first.year(), last.format("%y"));
                let final_legend_title_string = format!("{year_string} {legend_base}");
                let final_legend_title = final_legend_title_string.as_str();
                chart
                    .draw_series(LineSeries::new(
                        water_year
                            .0
                            .iter()
                            .map(|survey| {
                                let normalized_date_observation: NormalizedNaiveDate =
                                    survey.get_tap().date_observation.into();
                                let normalized_naive_date_observation =
                                    normalized_date_observation.into();
                                let observation = survey.get_tap().value_as_f64();
                                (normalized_naive_date_observation, observation)
                            })
                            .collect::<Vec<_>>(),
                        rgb_color,
                    ))
                    .unwrap()
                    .label(final_legend_title)
                    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], *rgb_color));
            }
            chart
                .configure_series_labels()
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw()
                .unwrap();
            backend_drawing_area.present().unwrap();
        };
        Ok(())
    }
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
            selected_sort: Msg::SelectedSort(SortBy::MostRecent),
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
            Msg::SelectedSort(sortie) => match sortie {
                SortBy::DriestYears => {
                    self.selected_sort = Msg::SelectedSort(SortBy::DriestYears);
                }
                SortBy::MostRecent => {
                    self.selected_sort = Msg::SelectedSort(SortBy::MostRecent);
                }
            },
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let mut svg_inner = String::new();
        let _svg_result = Model::generate_svg(self, &mut svg_inner);
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
        let sort_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, SORT_BY_SELECTION_ID));
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
            let mut reservoir_ids_sorted = self.reservoir_data.keys().cloned().collect::<Vec<_>>();
            reservoir_ids_sorted.sort();

            html! {
                <div id={DIV_BLOG_NAME}>
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
                    // Needs to show normalized annual charts
                    <div id={DIV_SORT_BY_SELECTION_ID}>
                    {SORT_BY_TEXT}
                        <select id={SORT_BY_SELECTION_ID} onchange={sort_callback}>
                        {
                            match self.selected_sort {
                                Msg::SelectedSort(SortBy::MostRecent) => {
                                    html!{
                                        <option value={MOST_RECENT} selected=true>{MOST_RECENT_OPTION_TEXT}</option>
                                    }
                                },
                                Msg::SelectedSort(SortBy::DriestYears) => {
                                    html!{
                                    <option value={MOST_RECENT}>{MOST_RECENT_OPTION_TEXT}</option>
                                    }
                                },
                                _ => {
                                    html!{
                                        <option value={MOST_RECENT} selected=true>{MOST_RECENT_OPTION_TEXT}</option>
                                    }
                                },
                            }
                        }
                        {
                            match self.selected_sort {
                                Msg::SelectedSort(SortBy::MostRecent) => {
                                    html!{
                                        <option value={DRIEST}>{DRIEST_OPTION_TEXT}</option>
                                    }
                                },
                                Msg::SelectedSort(SortBy::DriestYears) => {
                                    html!{
                                    <option value={DRIEST} selected=true>{DRIEST_OPTION_TEXT}</option>
                                    }
                                },
                                _ => {
                                    html!{
                                        <option value={DRIEST}>{DRIEST_OPTION_TEXT}</option>
                                    }
                                },
                            }
                        }
                        </select>
                    </div>
                    {svg_vnode}
                </div>
            }
        } else {
            html! {}
        }
    }
}

// TODO fix this so it is not about dates but reservoir ids
pub fn generic_callback(_event: Event, dom_id_str: &str) -> Msg {
    let input_string = web_sys::window()
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
    match dom_id_str {
        RESERVOIR_SELECTION_ID => Msg::SelectReservoir(input_string),
        SORT_BY_SELECTION_ID => {
            let input_str = input_string.as_str();
            match input_str {
                MOST_RECENT => Msg::SelectedSort(SortBy::MostRecent),
                DRIEST => Msg::SelectedSort(SortBy::DriestYears),
                // this seems to be the least harmful
                _ => Msg::SelectedSort(SortBy::MostRecent),
            }
        }
        _ => {
            // fix this if there is ever a false positive
            // this seems to be the least harmful
            Msg::SelectedSort(SortBy::MostRecent)
        }
    }
}
