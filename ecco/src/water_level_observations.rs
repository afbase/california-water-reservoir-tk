use cdec::{observation::{Observation, DataRecording}, reservoir::Reservoir};
use chrono::NaiveDate;
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};

pub struct  WaterLevelObservations {
    pub observations: BTreeMap<NaiveDate, u32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
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
        let records = Observation::get_all_records();
        let observations = Observation::records_to_observations(records);
        for observation in observations {
            let res_capacity = match reservoirs.get(&observation.station_id) {
                Some(r) => r.capacity as u32,
                None => 0u32,
            };
            let observed_value = {
                match observation.value {
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
            california_water_level_observations
                .entry(observation.date_observation)
                .and_modify(|e| *e += observed_value)
                .or_insert(observed_value);
        }
        let keys: Vec<NaiveDate> = california_water_level_observations
        .clone()
        .keys()
        .sorted()
        .map(|m| {
            *m
        })
        .collect();
        let start = keys.first().unwrap();
        let end = keys.last().unwrap();
        WaterLevelObservations {
            observations: california_water_level_observations,
            start_date: *start,
            end_date: *end,
        }
        // california_water_level_observations
    }
}