use crate::run::get_surveys_of_reservoirs;
use crate::Commands;
use cdec::{
    observable::{InterpolateObservableRanges, ObservableRangeRunner},
    reservoir::Reservoir,
    reservoir_observations::{GetWaterYears, ReservoirObservations},
    water_year::WaterYearStatistics,
};

use chrono::{Local, NaiveDate};
use log::info;
use serde_cbor::to_writer;
use std::collections::HashMap;
use std::{io::Write, path::PathBuf};
use utils::error::date_error;
use utils::{error::TryFromError, run::Run};

pub struct Peruse {
    pub summation_output: Option<PathBuf>,
    pub reservoir_output: Option<PathBuf>,
    pub water_years_output: Option<PathBuf>,
    pub min_max_output: Option<PathBuf>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

impl TryFrom<Commands> for Peruse {
    type Error = TryFromError;

    fn try_from(value: Commands) -> Result<Self, Self::Error> {
        match value {
            Commands::Peruse {
                summation_output,
                reservoir_output,
                water_years_output,
                min_max_output,
                start_date,
                end_date,
            } => Ok(Peruse {
                summation_output,
                reservoir_output,
                water_years_output,
                min_max_output,
                start_date,
                end_date,
            }),
            _ => Err(TryFromError::PeruseError),
        }
    }
}

impl Run for Peruse {
    async fn run(self) {
        info!("cdec-tk!");
        let end_date_final = match self.end_date {
            None => {
                let now = Local::now();
                now.date_naive()
            }
            Some(end_date_string) => {
                match NaiveDate::parse_from_str(end_date_string.as_str(), "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(err) => {
                        date_error("Start".to_string(), err);
                        panic!();
                    }
                }
            }
        };
        info!("end date: {:?}", end_date_final);
        let start_date_final = match self.start_date {
            None => NaiveDate::from_ymd_opt(1925, 1, 1).unwrap(),
            Some(start_date_string) => {
                match NaiveDate::parse_from_str(start_date_string.as_str(), "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(err) => {
                        date_error("Start".to_string(), err);
                        panic!();
                    }
                }
            }
        };
        info!("start date: {:?}", start_date_final);
        let cdec_data = get_surveys_of_reservoirs(&start_date_final, &end_date_final).await;

        match self.summation_output {
            None => {}
            Some(file_path) => {
                info!("running summation now");
                let csv_out = cdec_data.run_csv_v2();
                let mut fs = std::fs::File::create(file_path.as_path()).unwrap();
                if fs.write_all(csv_out.as_bytes()).is_err() {
                    panic!("writing csv file failed");
                }
                info!("summation file path: {:?}", file_path);
            }
        };
        match self.reservoir_output {
            None => {}
            Some(file_path) => {
                info!("running summation now");
                let csv_out = cdec_data.run_csv();
                let mut fs = std::fs::File::create(file_path.as_path()).unwrap();
                if fs.write_all(csv_out.as_bytes()).is_err() {
                    panic!("writing csv file failed");
                }
                info!("reservoir file path: {:?}", file_path);
            }
        };

        match (self.water_years_output, self.min_max_output) {
            (None, None) => {}
            (Some(water_years_file_path), Some(min_max_file_path)) => {
                info!("calculating water years");
                let mut observation_ranges = cdec_data.clone();
                observation_ranges.interpolate_reservoir_observations();
                let mut observations = observation_ranges
                    .into_iter()
                    .flat_map(|observation_range| observation_range.observations)
                    .collect::<Vec<_>>();
                let mut hash_map: HashMap<String, ReservoirObservations> = HashMap::new();
                let reservoirs = Reservoir::get_reservoir_vector();

                for reservoir in reservoirs {
                    let station_id = reservoir.station_id;
                    let (surveys, remaining_observations): (Vec<_>, Vec<_>) =
                        observations.into_iter().partition(|survey| {
                            let tap = survey.get_tap();
                            let tap_station_id = tap.station_id.clone();
                            tap_station_id == station_id
                        });
                    observations = remaining_observations;

                    if surveys.is_empty() {
                        continue;
                    }

                    let mut surveys = surveys;
                    surveys.sort();
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

                let water_years_from_observable_ranges =
                    hash_map.get_water_years_from_reservoir_observations();

                let water_statistics = water_years_from_observable_ranges
                    .into_iter()
                    .map(|(station_id, water_years)| {
                        let water_statistics = water_years
                            .iter()
                            .map(|water_year| water_year.into())
                            .collect::<Vec<WaterYearStatistics>>();
                        (station_id, water_statistics)
                    })
                    .collect::<HashMap<String, Vec<WaterYearStatistics>>>();

                let water_years_fs =
                    std::fs::File::create(water_years_file_path.as_path()).unwrap();
                to_writer(water_years_fs, &hash_map).expect("failed to write water years file");

                let min_max_fs = std::fs::File::create(min_max_file_path.as_path()).unwrap();
                to_writer(min_max_fs, &water_statistics).expect("failed to write min_max file");
            }
            (Some(water_years_file_path), None) => {
                info!("calculating water years");
                let mut observation_ranges = cdec_data.clone();
                observation_ranges.interpolate_reservoir_observations();
                let mut observations = observation_ranges
                    .into_iter()
                    .flat_map(|observation_range| observation_range.observations)
                    .collect::<Vec<_>>();
                let mut hash_map: HashMap<String, ReservoirObservations> = HashMap::new();
                let reservoirs = Reservoir::get_reservoir_vector();

                for reservoir in reservoirs {
                    let station_id = reservoir.station_id;
                    let (surveys, remaining_observations): (Vec<_>, Vec<_>) =
                        observations.into_iter().partition(|survey| {
                            let tap = survey.get_tap();
                            let tap_station_id = tap.station_id.clone();
                            tap_station_id == station_id
                        });
                    observations = remaining_observations;

                    if surveys.is_empty() {
                        continue;
                    }

                    let mut surveys = surveys;
                    surveys.sort();
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

                let water_years_fs =
                    std::fs::File::create(water_years_file_path.as_path()).unwrap();
                to_writer(water_years_fs, &hash_map).expect("failed to write water years file");
            }
            (None, Some(min_max_file_path)) => {
                info!("calculating water years");
                let mut observation_ranges = cdec_data.clone();
                observation_ranges.interpolate_reservoir_observations();
                let mut observations = observation_ranges
                    .into_iter()
                    .flat_map(|observation_range| observation_range.observations)
                    .collect::<Vec<_>>();
                let mut hash_map: HashMap<String, ReservoirObservations> = HashMap::new();
                let reservoirs = Reservoir::get_reservoir_vector();

                for reservoir in reservoirs {
                    let station_id = reservoir.station_id;
                    let (surveys, remaining_observations): (Vec<_>, Vec<_>) =
                        observations.into_iter().partition(|survey| {
                            let tap = survey.get_tap();
                            let tap_station_id = tap.station_id.clone();
                            tap_station_id == station_id
                        });
                    observations = remaining_observations;

                    if surveys.is_empty() {
                        continue;
                    }

                    let mut surveys = surveys;
                    surveys.sort();
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

                let water_years_from_observable_ranges =
                    hash_map.get_water_years_from_reservoir_observations();

                let water_statistics = water_years_from_observable_ranges
                    .into_iter()
                    .map(|(station_id, water_years)| {
                        let water_statistics = water_years
                            .iter()
                            .map(|water_year| water_year.into())
                            .collect::<Vec<WaterYearStatistics>>();
                        (station_id, water_statistics)
                    })
                    .collect::<HashMap<String, Vec<WaterYearStatistics>>>();

                let min_max_fs = std::fs::File::create(min_max_file_path.as_path()).unwrap();
                to_writer(min_max_fs, &water_statistics).expect("failed to write min_max file");
            }
        };
    }
}
