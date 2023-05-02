#![feature(drain_filter)]
use cdec::{
    normalized_naive_date::NormalizedNaiveDate,
    observable::{CompressedSurveyBuilder, InterpolateObservableRanges, ObservableRange},
    reservoir::Reservoir,
    water_year::{NormalizeWaterYears, WaterYear},
};
use chrono::{Datelike, NaiveDate};
use ecco::{calendar_year_model::get_colors, reservoir_observations::ReservoirObservations};
use log::{info, LevelFilter};
use my_log::MY_LOGGER;
use plotters::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

const DIV_SORT_BY_SELECTION_ID: &str = "div-select-sort-by";
pub const DIV_BLOG_NAME: &str = "yew-wot_m8";
pub const DIV_RESERVOIR_SELECTION_ID: &str = "div-reservoir-selections"; //
const _ELEMENT_ID: &str = "svg-chart";
const MOST_RECENT: &str = "Most Recent";
const DRIEST: &str = "Driest";
const DRIEST_OPTION_TEXT: &str = "Sort By Driest";
const MOST_RECENT_OPTION_TEXT: &str = "Sort By Most Recent";
const SORT_BY_SELECTION_ID: &str = "select-sort-by";
const SELECT_RESERVOIR_TEXT: &str = "Select Reservoir: "; //
const SORT_BY_TEXT: &str = "Sort by: ";
pub const RESERVOIR_SELECTION_ID: &str = "reservoir-selections";
pub const NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT: usize = 20;

fn main() {
    log::set_logger(&MY_LOGGER).unwrap();
    log::set_max_level(LevelFilter::Info);
    web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_str = "failed to load wasm module successfully part 1";
                let log_string = String::from(log_str);
                info!("{}", log_string);
                panic!("{}", log_str);
            },
            |document| match document.get_element_by_id(DIV_BLOG_NAME) {
                Some(div_element) => {
                    let renderer = yew::Renderer::<ObservationsModel>::with_root(div_element);
                    renderer.render();
                }
                None => {
                    let log_str = format!(
                        "Unable to find div {}. failed to load wasm module successfully part 2",
                        DIV_BLOG_NAME
                    );
                    info!("{}", log_str);
                    panic!("{}", log_str);
                }
            },
        );
}

#[derive(Debug, Clone)]
pub enum SortBy {
    MostRecent,
    DriestYears,
}

#[derive(Debug, Clone)]
pub enum Msg {
    // The user selected a reservoir from the dropdown list
    SelectReservoir(String),
    SelectedSort(SortBy),
}

#[derive(Debug, Clone)]
struct ObservationsModel {
    // The selected reservoir
    pub selected_reservoir: String,
    // the type of sort
    pub selected_sort: Msg,
    // most recent water years
    pub most_recent_water_years: HashMap<String, Vec<WaterYear>>,
    // driest whater years
    pub driest_water_years: HashMap<String, Vec<WaterYear>>,
    // use this to get reservoir information
    pub reservoir_vector: Vec<Reservoir>,
    // use this in the view()
    pub station_ids_sorted: Vec<String>,
}

impl<'a> ObservationsModel {
    fn derive_legend_name(&self) -> String {
        // let data = self.reservoir_data.get(&self.selected_reservoir).unwrap();
        // let station_id = data[0].clone().0[0].tap().station_id.clone();
        let reservoir = self
            .reservoir_vector
            .iter()
            .find_map(|reservoir_item| {
                let mut result = None;
                let reservoir_station_id = &reservoir_item.station_id;
                if reservoir_station_id == &self.selected_reservoir {
                    result = Some(reservoir_item);
                }
                result
            })
            .unwrap();
        format!("{} - {}", reservoir.dam, self.selected_reservoir)
    }

    pub fn generate_svg(&self, svg_inner_string: &'a mut String) -> DrawResult<(), SVGBackend<'a>> {
        let legend_base = self.derive_legend_name();
        let date_range_tuple = NormalizedNaiveDate::get_normalized_tuple_date_range();
        let range_date = Range {
            start: date_range_tuple.0,
            end: date_range_tuple.1,
        };
        let ranged_date: RangedDate<NaiveDate> = range_date.into();
        let water_years_data = {
            match self.selected_sort {
                Msg::SelectedSort(SortBy::DriestYears) => {
                    self.driest_water_years.get(&self.selected_reservoir)
                }
                Msg::SelectedSort(SortBy::MostRecent) => {
                    self.most_recent_water_years.get(&self.selected_reservoir)
                }
                _ => self.most_recent_water_years.get(&self.selected_reservoir),
            }
        }
        .unwrap();
        let selected_reservoir = self.selected_reservoir.clone();
        let water_years_len = water_years_data.len();
        info!("Generating SVG for {selected_reservoir}; number of water years {water_years_len}");
        let y_max = water_years_data
            .get_largest_acrefeet_over_n_years(NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT)
            .unwrap();
        let colors_for_water_years = get_colors(NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT).unwrap();
        // let plot_and_color = water_years_data.iter().zip(colors_for_water_years.iter());
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
        for idx in 0..water_years_len {
            let rgb_color = &colors_for_water_years[idx];
            let water_year = &water_years_data[idx];
            let survey_count = water_year.0.len();
            // date_recording is the original date in normalization
            let (first, last) = water_year.calendar_year_from_normalized_water_year();
            info!("{selected_reservoir} has {survey_count} surveys starting from {first} through {last}");
            let year_string = format!("{}-{}", first.year(), last.format("%y"));
            let final_legend_title_string = format!("{year_string} {legend_base}");
            let final_legend_title = final_legend_title_string.as_str();
            chart
                .draw_series(LineSeries::new(
                    water_year
                        .0
                        .iter()
                        .map(|survey| {
                            let observation = survey.get_tap().value_as_f64();
                            (survey.get_tap().date_observation, observation)
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
        Ok(())
    }
}

impl Component for ObservationsModel {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        info!("un-lzma csv things");
        let observations = ReservoirObservations::init_from_lzma_without_interpolation();
        info!("un-lzma csv things done!");
        info!("setting the reservoir observed keys");
        let reservoirs_observed = observations.keys().cloned().collect::<HashSet<_>>();
        info!("create reservoir vector");
        let mut reservoir_vector = Reservoir::get_reservoir_vector();
        let station_ids = reservoir_vector
            .iter()
            .map(|resy| resy.station_id.clone())
            .collect::<HashSet<_>>();
        let mut station_ids_sorted = station_ids
            .intersection(&reservoirs_observed)
            .cloned()
            .collect::<Vec<_>>();
        station_ids_sorted.sort();
        info!("station ids ready to go!!!");
        let selected_reservoir = String::from("ORO");
        let selected_sort = Msg::SelectedSort(SortBy::MostRecent);
        let mut driest_water_years: HashMap<String, Vec<WaterYear>> = HashMap::new();
        let mut most_recent_water_years: HashMap<String, Vec<WaterYear>> = HashMap::new();
        for (reservoir_id, reservoir_observations) in observations {
            let mut most_recent_vec: Vec<WaterYear> = Vec::new();
            let mut driest_vec: Vec<WaterYear> = Vec::new();
            let mut observable_range = ObservableRange::new(
                reservoir_observations.start_date,
                reservoir_observations.end_date,
            );
            observable_range.observations = reservoir_observations.observations;
            let mut vec_observable_range: Vec<ObservableRange> = vec![observable_range];
            vec_observable_range.interpolate_reservoir_observations();
            if let Some(observable_range) = vec_observable_range.first() {
                let mut water_years =
                    WaterYear::water_years_from_observable_range(observable_range);
                // need to sort by most recent, store the top 20
                // and then sort by driest, store the top 20
                water_years.normalize_dates();
                water_years.sort_by_most_recent();
                let water_years_len = water_years.len();
                let idx_max = NUMBER_OF_CHARTS_TO_DISPLAY_DEFAULT.min(water_years_len);
                if idx_max <= 2 {
                    info!("skipping station: {reservoir_id}; water_years_len: {water_years_len}");
                    let _ = reservoir_vector
                        .drain_filter(|r| r.station_id == reservoir_id)
                        .collect::<Vec<_>>();
                    let mut r_clone = reservoir_id.clone();
                    let _ = station_ids_sorted
                        .drain_filter(|s| {
                            let r_id_mut = r_clone.as_mut();
                            let s_str_mut = s.as_mut();
                            s_str_mut.eq(&r_id_mut)
                        })
                        .collect::<Vec<_>>();
                    continue;
                }
                info!("using station: {reservoir_id}; water_years_len: {water_years_len}");
                let mut other = water_years[0..idx_max].to_vec().clone();
                other.sort_surveys();
                most_recent_vec.append(&mut other);
                most_recent_water_years.insert(reservoir_id.clone(), most_recent_vec);
                water_years.sort_by_lowest_recorded_years();
                other = water_years[0..idx_max].to_vec().clone();
                other.sort_surveys();
                driest_vec.append(&mut other);
                driest_water_years.insert(reservoir_id, driest_vec);
            };
        }
        Self {
            selected_reservoir,
            selected_sort,
            most_recent_water_years,
            driest_water_years,
            reservoir_vector,
            station_ids_sorted,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            // The user selected a reservoir from the dropdown list
            Msg::SelectReservoir(reservoir) => {
                // Set the selected reservoir and fetch the data for that reservoir
                let mut reversed = reservoir.chars().rev().collect::<String>();
                reversed.truncate(3);
                self.selected_reservoir = reversed.chars().rev().collect::<String>();
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
        let sort_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, SORT_BY_SELECTION_ID));
        let reservoir_selection_callback = ctx
            .link()
            .callback(|event: Event| generic_callback(event, RESERVOIR_SELECTION_ID));

        html! {
            <div id={DIV_BLOG_NAME}>
                <div id={DIV_RESERVOIR_SELECTION_ID}>
                    // Dropdown list for selecting a reservoir
                    {SELECT_RESERVOIR_TEXT}
                    <select id={RESERVOIR_SELECTION_ID} onchange={reservoir_selection_callback}>
                    { for
                        self.station_ids_sorted.iter().map(|station_id| {
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
    }
}

pub fn generic_callback(_event: Event, dom_id_str: &str) -> Msg {
    let input_string = web_sys::window()
        .and_then(|window| window.document())
        .map_or_else(
            || {
                let log_string = "window document object not found.".to_string();
                info!("{}", log_string);
                String::from("none")
            },
            |document| match document.get_element_by_id(dom_id_str) {
                Some(input) => {
                    let input_element = input.dyn_into::<HtmlSelectElement>().unwrap();
                    input_element.value()
                }
                None => {
                    let log_string = format!("{} {}", dom_id_str, "dom object not found.");
                    info!("{}", log_string);
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
