use cdec::{
    observable::{CompressedSurveyBuilder, InterpolateObservableRanges, ObservableRange},
    observation::Observation,
    reservoir::Reservoir,
    survey::Survey,
    survey::{CompressedStringRecord, VectorCompressedStringRecord},
    water_year::WaterYear,
};
use chrono::NaiveDate;
use std::collections::HashMap;
use std::vec;

#[derive(Debug, Clone)]
pub struct ReservoirObservations {
    pub observations: Vec<Survey>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

pub trait ReservoirObservationsLike {
    fn observations(&self, station_id: &str) -> Option<Vec<Survey>>;
    fn start_date(&self, station_id: &str) -> Option<NaiveDate>;
    fn end_date(&self, station_id: &str) -> Option<NaiveDate>;
}

impl ReservoirObservationsLike for HashMap<String, ReservoirObservations> {
    fn observations(&self, station_id: &str) -> Option<Vec<Survey>> {
        if let Some(reservoir_observations) = self.get(station_id) {
            return Some(reservoir_observations.observations.clone());
        }
        None
    }
    fn start_date(&self, station_id: &str) -> Option<NaiveDate> {
        if let Some(reservoir_observations) = self.get(station_id) {
            return Some(reservoir_observations.start_date);
        }
        None
    }
    fn end_date(&self, station_id: &str) -> Option<NaiveDate> {
        if let Some(reservoir_observations) = self.get(station_id) {
            return Some(reservoir_observations.end_date);
        }
        None
    }
}

impl ReservoirObservations {
    pub fn station_id(&self) -> String {
        let survey = &self.observations[0];
        survey.get_tap().station_id.clone()
    }

    pub fn init_from_lzma_without_interpolation() -> HashMap<String, Self> {
        let records: Vec<CompressedStringRecord> = Observation::get_all_records();
        let mut observations = records.records_to_surveys();
        let mut hash_map: HashMap<String, Self> = HashMap::new();
        let reservoirs = Reservoir::get_reservoir_vector();
        
        for reservoir in reservoirs {
            let station_id = reservoir.station_id;
            
            // Replace extract_if with partition
            let (matching_surveys, remaining_observations): (Vec<_>, Vec<_>) = observations
                .into_iter()
                .partition(|survey| {
                    let tap = survey.get_tap();
                    let tap_station_id = tap.station_id.clone();
                    tap_station_id == station_id
                });
            observations = remaining_observations;
            
            let mut surveys = matching_surveys;
            surveys.sort();
            
            if surveys.is_empty() {
                continue;
            }
            
            let surveys_len = surveys.len();
            let start_date = surveys[0].get_tap().date_observation;
            let end_date = surveys[surveys_len - 1].get_tap().date_observation;

            let reservoir_observations = ReservoirObservations {
                observations: surveys,
                start_date,
                end_date,
            };
            hash_map.insert(station_id, reservoir_observations);
        }
        hash_map
    }

    pub fn init_from_lzma() -> HashMap<String, Self> {
        let records: Vec<CompressedStringRecord> = Observation::get_all_records();
        let mut observations = records.records_to_surveys();
        let mut hash_map: HashMap<String, Self> = HashMap::new();
        let reservoirs = Reservoir::get_reservoir_vector();
        
        for reservoir in reservoirs {
            let station_id = reservoir.station_id;
            
            // Replace extract_if with partition
            let (matching_surveys, remaining_observations): (Vec<_>, Vec<_>) = observations
                .into_iter()
                .partition(|survey| {
                    let tap = survey.get_tap();
                    let tap_station_id = tap.station_id.clone();
                    tap_station_id == station_id
                });
            observations = remaining_observations;
            
            let mut surveys = matching_surveys;
            surveys.sort();
            
            if surveys.is_empty() {
                continue;
            }
            
            let surveys_len = surveys.len();
            let start_date = surveys[0].get_tap().date_observation;
            let end_date = surveys[surveys_len - 1].get_tap().date_observation;

            // okay this part below is a bit wonky and lazy
            let mut observable_range = ObservableRange::new(start_date, end_date);
            observable_range.observations = surveys;
            let mut vec_observable_range = vec![observable_range];
            vec_observable_range.interpolate_reservoir_observations();
            let observable_range = &vec_observable_range[0];
            let surveys = observable_range.observations.clone();
            // okay this part above is a bit wonky and lazy

            let reservoir_observations = ReservoirObservations {
                observations: surveys,
                start_date,
                end_date,
            };
            hash_map.insert(station_id, reservoir_observations);
        }
        hash_map
    }
}

/// TODO: finish this
pub trait GetWaterYears {
    fn get_water_years_from_reservoir_observations(&self) -> HashMap<String, Vec<WaterYear>>;
}

impl GetWaterYears for HashMap<String, ReservoirObservations> {
    fn get_water_years_from_reservoir_observations(&self) -> HashMap<String, Vec<WaterYear>> {
        let mut hash_map = HashMap::new();
        for (station_id, reservoir_observations) in self {
            let observable_range: ObservableRange =
                reservoir_observations.observations.clone().into();
            let water_years = WaterYear::water_years_from_observable_range(&observable_range);
            hash_map.insert(station_id.clone(), water_years);
        }
        hash_map
    }
}