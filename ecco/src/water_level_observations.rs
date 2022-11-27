use cdec::{
    observable::{CompressedSurveyBuilder, ObservableRange},
    observation::{DataRecording, Observation},
    reservoir::Reservoir,
    survey::Survey,
    survey::{CompressedStringRecord, VectorCompressedStringRecord},
};
use chrono::NaiveDate;
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};

pub struct WaterLevelObservations {
    pub observations: BTreeMap<NaiveDate, u32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub min_date: NaiveDate,
    pub max_date: NaiveDate,
}

impl WaterLevelObservations {
    pub fn init_from_lzma() -> Self {
        let reservoirs: HashMap<String, Reservoir> = Reservoir::get_reservoir_vector()
            .iter()
            .map(|res| {
                let station = res.station_id.clone();
                let res_copy = res.clone();
                (station, res_copy)
            })
            .into_iter()
            .collect();
        let mut california_water_level_observations: BTreeMap<NaiveDate, u32> = BTreeMap::new();
        let mut observable_ranges_by_reservoir: BTreeMap<String, Vec<Survey>> = BTreeMap::new();
        let records: Vec<CompressedStringRecord> = Observation::get_all_records();
        let observations = records.records_to_surveys();
        // needs to build observable ranges for each reservoir and then interpolate
        for survey in observations {
            let survey_tap = survey.get_tap();
            let reservoir_id = survey_tap.station_id.clone();
            observable_ranges_by_reservoir
                .entry(reservoir_id)
                .and_modify(|vec| {
                    let survey_clone = survey.clone();
                    vec.push(survey_clone);
                })
                .or_insert(vec![survey.clone()]);
        }
        let sorted_interpolated_observations = observable_ranges_by_reservoir
            .iter_mut()
            .map(|(station_id, vec_survey)| {
                let station_id_clone = station_id.clone();
                let mut observations = vec_survey.clone();
                observations.sort();
                let start_date = observations.first().unwrap().get_tap().date_observation;
                let end_date = observations.last().unwrap().get_tap().date_observation;
                let mut observables = ObservableRange {
                    observations,
                    start_date,
                    end_date,
                };
                observables.retain();
                observables.finalize();
                observables.pad_end();
                (station_id_clone, observables)
            })
            .collect::<HashMap<String, ObservableRange>>();
        for (station_id, observables) in sorted_interpolated_observations {
            let res_capacity = match reservoirs.get(&station_id) {
                Some(r) => r.capacity as u32,
                None => 0u32,
            };
            for observation in observables.observations {
                let observed_value = {
                    let observation_value = observation.get_tap();
                    match observation_value.value {
                        DataRecording::Recording(v) => {
                            // sometimes the data is very noisy so
                            // simply choose the lesser of two values
                            // either the observed value of capacity
                            // of the reservoir
                            v.min(res_capacity)
                        }
                        _ => 0,
                    }
                };
                let tippy_tap = observation.get_tap();
                california_water_level_observations
                    .entry(tippy_tap.date_observation)
                    .and_modify(|e| *e += observed_value)
                    .or_insert(observed_value);
            }
        }
        // //build the data
        // for observation in observations {
        //     let res_capacity = match reservoirs.get(&observation.station_id) {
        //         Some(r) => r.capacity as u32,
        //         None => 0u32,
        //     };
        //     let observed_value = {
        //         match observation.value {
        //             DataRecording::Recording(v) => {
        //                 // sometimes the data is very noisy so
        //                 // simply choose the lesser of two values
        //                 // either the observed value of capacity
        //                 // of the reservoir
        //                 v.min(res_capacity)
        //             }
        //             _ => 0,
        //         }
        //     };
        //     california_water_level_observations
        //         .entry(observation.date_observation)
        //         .and_modify(|e| *e += observed_value)
        //         .or_insert(observed_value);
        // }
        let keys: Vec<NaiveDate> = california_water_level_observations
            .clone()
            .keys()
            .sorted()
            .copied()
            .collect();
        let start = keys.first().unwrap();
        let end = keys.last().unwrap();
        WaterLevelObservations {
            observations: california_water_level_observations,
            start_date: *start,
            min_date: *start,
            end_date: *end,
            max_date: *end,
        }
        // california_water_level_observations
    }
}
