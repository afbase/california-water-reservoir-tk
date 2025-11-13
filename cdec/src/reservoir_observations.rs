/// Reservoir observations and water year conversions
use crate::{
    error::Result, observable::ObservableRange, survey::Survey, water_year::WaterYear,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Observations for a single reservoir
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirObservations {
    /// All survey observations
    pub observations: Vec<Survey>,
    /// First observation date
    pub start_date: NaiveDate,
    /// Last observation date
    pub end_date: NaiveDate,
}

/// Map of reservoir observations by station ID
#[derive(Debug, Serialize, Deserialize)]
pub struct WaterObservationsByReservoir {
    #[serde(with = "vectorize")]
    pub map: HashMap<String, ReservoirObservations>,
}

/// Trait for converting reservoir observations to water years
pub trait GetWaterYears {
    /// Converts all reservoir observations to water years
    ///
    /// # Errors
    ///
    /// Returns error if date parsing fails
    fn get_water_years_from_reservoir_observations(&self) -> Result<HashMap<String, Vec<WaterYear>>>;
}

impl GetWaterYears for HashMap<String, ReservoirObservations> {
    fn get_water_years_from_reservoir_observations(&self) -> Result<HashMap<String, Vec<WaterYear>>> {
        let mut hash_map = HashMap::new();
        for (station_id, reservoir_observations) in self {
            let observable_range: ObservableRange =
                reservoir_observations.observations.clone().into();
            let water_years = WaterYear::water_years_from_observable_range(&observable_range)?;
            hash_map.insert(station_id.clone(), water_years);
        }
        Ok(hash_map)
    }
}
