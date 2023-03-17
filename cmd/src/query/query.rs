use chrono::{NaiveDate, Local};
use my_log::MY_LOGGER;
use log::{info, LevelFilter};
use utils::{TryFromError, Run};
use crate::Commands;
use crate::run::run::{run_csv_v2, run_csv};
use std::str::FromStr;
use std::{
    io::Write,
    path::PathBuf,
};
use utils::error::date_error;
const DEFAULT_OUTPUT_PATH: &str = "output.tar.xz";


pub struct Query {
    pub output: Option<PathBuf>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub summation: bool,
}


impl TryFrom<Commands> for Query {
    type Error = TryFromError;

    fn try_from(value: Commands) -> Result<Self, Self::Error> {
        match value {
            Commands::Query {
                output,
                start_date,
                end_date,
                summation,
            } => 
                Ok(Query{
                    output,
                    start_date,
                    end_date,
                    summation,
                }),
            _ => Err(TryFromError::QueryError)
            
        }
    }
}

impl Run for Query {
    fn run(self) {
        log::set_logger(&MY_LOGGER).unwrap();
        log::set_max_level(LevelFilter::Info);
        let file_path = match self.output {
            None => {
                let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                file_path.unwrap()
            }
            Some(file_path) => file_path,
        };
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
        let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
        let csv_out = if self.summation {
            rt.block_on(run_csv_v2(&start_date_final, &end_date_final))
        } else {
            rt.block_on(run_csv(&start_date_final, &end_date_final))
        };
        let mut fs = std::fs::File::create(file_path.as_path()).unwrap();
        if fs.write_all(csv_out.as_bytes()).is_err() {
            panic!("writing csv file failed");
        }
        info!("Observations Written to CSV");
    }
}