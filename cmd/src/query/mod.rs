use crate::run::get_surveys_of_reservoirs_v2;
use crate::Commands;
//use cdec::observable::ObservableRange;
use cdec::observable::ObservableRangeRunner;
use cdec::reservoir::{CSV_OBJECT, CSV_OBJECT_NO_POWELL_NO_MEAD};

use chrono::{Local, NaiveDate};
use log::info;
use std::{io::Write, path::PathBuf};
use utils::error::date_error;
use utils::{error::TryFromError, run::Run};

pub struct Query {
    // output of total reservoir capacity
    pub summation_output: Option<PathBuf>,
    // output of each reservoir's capacity
    pub reservoir_output: Option<PathBuf>,
    // date of earliest data to be collected
    pub start_date: Option<String>,
    // date of latest data to be collected
    pub end_date: Option<String>,
    // flag to only include California Reservoirs,
    pub california_only: bool,
}

impl TryFrom<Commands> for Query {
    type Error = TryFromError;

    fn try_from(value: Commands) -> Result<Self, Self::Error> {
        match value {
            Commands::Query {
                summation_output,
                reservoir_output,
                start_date,
                end_date,
                california_only,
            } => Ok(Query {
                summation_output,
                reservoir_output,
                start_date,
                end_date,
                california_only,
            }),
            _ => Err(TryFromError::QueryError),
        }
    }
}

impl Run for Query {
    async fn run(self) {
        info!("cdec-tk!");
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
        info!("end date: {:?}", end_date_final);
        let start_date_final = match self.start_date {
            None => {
                //Oldest Reservoir Record is
                //LGT,Lagunitas,Lagunitas Lake,Lagunitas Creek,341,1925
                NaiveDate::from_ymd_opt(1924, 12, 30).unwrap()
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
        info!("start date: {:?}", start_date_final);
        let cdec_data = {
            if self.california_only {
                get_surveys_of_reservoirs_v2(
                    &start_date_final,
                    &end_date_final,
                    CSV_OBJECT_NO_POWELL_NO_MEAD,
                )
                .await
            } else {
                get_surveys_of_reservoirs_v2(&start_date_final, &end_date_final, CSV_OBJECT).await
            }
        };

        match self.summation_output {
            None => {}
            Some(file_path) => {
                info!("running summation now");
                let csv_out = cdec_data.run_csv_v2();
                info!("attempting to create file: {:?}", file_path);
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
    }
}
