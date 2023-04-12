use chrono::{Local, NaiveDate};
use log::LevelFilter;
use std::{path::PathBuf, str::FromStr};
use utils::{error::date_error, run::Run};

const DEFAULT_OUTPUT_PATH: &str = "output.tar.xz";

pub struct Survey {
    // if there is already existing data to append to
    pub existing_data_input: Option<PathBuf>,
    // output of total reservoir capacity
    pub summation_output: Option<PathBuf>,
    // output of each reservoir's capacity
    pub reservoir_output: Option<PathBuf>,
    // date of earliest data to be collected
    pub start_date: Option<String>,
    // date of latest data to be collected
    pub end_date: Option<String>,
}

impl Run for Survey {
    async fn run(self) {
        // log::set_logger(&MY_LOGGER).unwrap();
        log::set_max_level(LevelFilter::Info);
        // dates
        let _start_date_final = match self.start_date {
            None => {
                //Oldest Reservoir Record is
                //LGT,Lagunitas,Lagunitas Lake,Lagunitas Creek,341,1925
                NaiveDate::from_ymd_opt(1925, 1, 1).unwrap()
            }
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

        let _end_date_final = match self.end_date {
            None => {
                // Get Today's Date
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
        // get files
        let _existing_data_input_path = match self.existing_data_input {
            None => {
                let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                file_path.unwrap()
            }
            Some(file_path) => file_path,
        };
        let _summation_output_path = match self.summation_output {
            None => {
                let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                file_path.unwrap()
            }
            Some(file_path) => file_path,
        };
        let _reservoir_output = match self.reservoir_output {
            None => {
                let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                file_path.unwrap()
            }
            Some(file_path) => file_path,
        };
        // 1. unzip reservoir input
        // let mut observations = File::open(_existing_data_input_path).unwrap();
        // let mut buffer: Vec<u8> = Vec::new();
        // observations.read_to_end(&mut buffer).unwrap();
        // let bytes = buffer.as_slice();
        // let compressed_string_vectors = Observation::get_all_records_from_bytes(bytes);
        // let mut observations = compressed_string_vectors.records_to_surveys();
        // let mut hash_map: HashMap<String, ReservoirObservations> = HashMap::new();
        // let reservoirs = Reservoir::get_reservoir_vector();
        // for reservoir in reservoirs {
        //     let station_id = reservoir.station_id;
        //     let mut surveys = observations
        //         .drain_filter(|survey| {
        //             let tap = survey.get_tap();
        //             let tap_station_id = tap.station_id.clone();
        //             tap_station_id == station_id
        //         })
        //         .collect::<Vec<_>>();
        //     surveys.sort();
        //     if surveys.is_empty() {
        //         continue;
        //     }
        //     let surveys_len = surveys.len();
        //     let start_date = surveys[0].get_tap().date_observation;
        //     let end_date = surveys[surveys_len - 1].get_tap().date_observation;

        //     // // okay this part below is a bit wonky and lazy
        //     // let mut observable_range = ObservableRange::new(start_date, end_date);
        //     // observable_range.observations = surveys;
        //     // let mut vec_observable_range = vec![observable_range];
        //     // vec_observable_range.interpolate_reservoir_observations();
        //     // let observable_range = &vec_observable_range[0];
        //     // let surveys = observable_range.observations.clone();
        //     // // okay this part above is a bit wonky and lazy

        //     let reservoir_observations = ReservoirObservations {
        //         observations: surveys,
        //         start_date,
        //         end_date,
        //     };
        //     hash_map.insert(station_id, reservoir_observations);
        // }
        // hash_map
        // Need to
    }
}
