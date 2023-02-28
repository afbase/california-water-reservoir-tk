// mod errors;
pub mod run;
pub mod log;
pub mod error;
use run::Run;

use std::{
    path::PathBuf,
};

struct Query {
    output: Option<PathBuf>,
    start_date: Option<String>,
    end_date: Option<String>,
    summation: bool,
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
        let csv_out = if self.summation {
            run_csv_v2(&start_date_final, &end_date_final).await
        } else {
            run_csv(&start_date_final, &end_date_final).await
        };
        let mut fs = std::fs::File::create(file_path.as_path()).unwrap();
        if fs.write_all(csv_out.as_bytes()).is_err() {
            panic!("writing csv file failed");
        }
        info!("Observations Written to CSV");
    }
}