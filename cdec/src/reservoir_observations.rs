use crate::{observable::ObservableRange, survey::Survey, water_year::WaterYear};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirObservations {
    pub observations: Vec<Survey>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaterObservationsByReservoir {
    #[serde(with = "vectorize")]
    pub map: HashMap<String, ReservoirObservations>,
}

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
