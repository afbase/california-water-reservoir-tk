use std::{path::PathBuf, str::FromStr};
use utils::Run;
use chrono::{NaiveDate, Local};
use my_log::{MY_LOGGER};
use utils::error::date_error;
use log::LevelFilter;

const DEFAULT_OUTPUT_PATH: &str = "output.tar.xz";

pub struct Survey 
    {
        // if there is already existing data to append to
        existing_data_input: Option<PathBuf>,
        // output of total reservoir capacity
        summation_output: Option<PathBuf>,
        // output of each reservoir's capacity
        reservoir_output: Option<PathBuf>,
        // date of earliest data to be collected
        start_date: Option<String>,
        // date of latest data to be collected
        end_date: Option<String>,
    }

impl Run for Survey {
    fn run(self) {
        log::set_logger(&MY_LOGGER).unwrap();
        log::set_max_level(LevelFilter::Info);
        // dates
        let start_date_final = match self.start_date {
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

        let end_date_final = match self.end_date {
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
        let existing_data_input_path = match self.existing_data_input {
            None => {
                let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                file_path.unwrap()
            }
            Some(file_path) => file_path,
        };
        let summation_output_path = match self.summation_output {
            None => {
                let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                file_path.unwrap()
            }
            Some(file_path) => file_path,
        };
        let reservoir_output = match self.reservoir_output {
            None => {
                let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                file_path.unwrap()
            }
            Some(file_path) => file_path,
        };
    }
}