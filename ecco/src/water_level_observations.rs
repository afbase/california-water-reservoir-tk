use cdec::observation::{Observation, DataRecording};
use chrono::{Datelike, NaiveDate};
use easy_cast::Cast;
use std::collections::BTreeMap;
use std::ops::Range;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use web_sys::HtmlCanvasElement;

pub type WaterLevelObservations = BTreeMap<NaiveDate, u32>;

trait WaterLevelObservationsTrait {
    fn update_start_date(self, new_date: NaiveDate);
    fn update_end_date(self, new_end_date: NaiveDate);
    fn draw_wasm(self, canvas: HtmlCanvasElement, start_date: NaiveDate, end_date: NaiveDate) -> DrawResult<(), CanvasBackend>;
    fn init_from_lzma() -> Self;
}

impl WaterLevelObservationsTrait for WaterLevelObservations {
    fn update_start_date(self, new_date: NaiveDate) {
        // don't delete the data just read the data from the new date.

        todo!();

        // let (start_date, end_date) = {
        //     let start = *self.first_entry().unwrap().key();
        //     let end = *self.last_entry().unwrap().key();
        //     if new_date < start {
        //         (new_date, start)
        //     } else if start <= new_date && new_date <= end {
        //         (start, new_date)
        //     } else {
        //         (start, new_date)
        //     }
        // };
        
        // let duration = ((end_date - start_date).num_days() + 1) as usize;
        // let data: Vec<DataPoint> = start_date
        //     .iter_days()
        //     .take(duration)
        //     .enumerate()
        //     .map(|(idx, _d)| {
        //         let date = start_date + Duration::days(idx as i64);
        //         let acre_feet = idx as f32;
        //         DataPoint { date, acre_feet }
        //     })
        //     .collect();
        // self.data = data;
    }

    fn update_end_date(self, new_end_date: NaiveDate) {
        // self.update_start_date(new_end_date);
        todo!();
    }


    fn init_from_lzma() -> Self {
        let mut california_water_level_observations: WaterLevelObservations = WaterLevelObservations::new();
        let records = Observation::get_all_records();
        let observations = Observation::records_to_observations(records);
        for observation in observations {
            let k = {
                // TODO: https://github.com/afbase/california-water-reservoir-wasm/blob/04b9ee762aa4e8314846e33aae74995e399789bd/src/fetch.rs#L11
                // use Reservoir::parse_reservoir_csv()
                match observation.value {
                    DataRecording::Recording(v) => v as u32,
                    _ => 0,
                }
            };
            california_water_level_observations
            .entry(observation.date_observation)
            .and_modify(|e| *e += k)
            .or_insert(k);
        }
        california_water_level_observations
    }

    fn draw_wasm(self, canvas: HtmlCanvasElement, start_date: NaiveDate, end_date: NaiveDate) -> DrawResult<(), CanvasBackend> {
        // TODO: use the parameter dates and corresponding values for the chart
        let dates: Vec<NaiveDate> = self.keys().cloned().collect();
        // let start_date = dates.as_slice().first().unwrap();
        // let end_date = dates.as_slice().last().unwrap();
        //Goal get max and min value of btree:
        let date_range = Range {
            start: start_date,
            end: end_date,
        };
        let ranged_date: RangedDate<NaiveDate> = date_range.into();
        let values = self.values().cloned().collect::<Vec<u32>>();
        let y_max: f64 = (*values.iter().max().unwrap() as i64).cast();
        let y_min: f64 = (*values.iter().min().unwrap() as i64).cast();
        let _x_max = values.len() as f64;
        let x_labels_amount = (end_date.year() - start_date.year()) as usize;
        // setup chart
        // setup canvas drawing area
        let backend = CanvasBackend::with_canvas_object(canvas).unwrap();
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
                self.iter()
                    .map(|x| (*x.0, *x.1 as i32 as f64))
                    .collect::<Vec<_>>(),
                &RED,
            ))
            .unwrap()
            .label("water")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()
            .unwrap();
        backend_drawing_area.present().unwrap();
        Ok(())
    }
}