#![feature(extract_if)]

pub mod peruse;
pub mod query;
pub mod run;
pub mod survey;
use clap::Subcommand;
use std::path::PathBuf;
#[derive(Subcommand)]
pub enum Commands {
    Survey {
        // if there is already existing data to append to
        #[arg(long, value_name = "COMPRESSED_TAR")]
        existing_data_input: Option<PathBuf>,
        // output of total reservoir capacity
        #[arg(long, value_name = "SUMMATION_FILE")]
        summation_output: Option<PathBuf>,
        // output of each reservoir's capacity
        #[arg(long, value_name = "RESERVOIR_FILE")]
        reservoir_output: Option<PathBuf>,
        // date of earliest data to be collected
        #[arg(long, value_name = "YYYY-MM-DD")]
        start_date: Option<String>,
        // date of latest data to be collected
        #[arg(long, value_name = "YYYY-MM-DD")]
        end_date: Option<String>,
    },
    Query {
        // output of total reservoir capacity
        #[arg(long, short, value_name = "SUMMATION_FILE")]
        summation_output: Option<PathBuf>,
        // output of each reservoir's capacity
        #[arg(long, short, value_name = "RESERVOIR_FILE")]
        reservoir_output: Option<PathBuf>,
        // date of earliest data to be collected
        #[arg(long, value_name = "YYYY-MM-DD")]
        start_date: Option<String>,
        // date of latest data to be collected
        #[arg(long, value_name = "YYYY-MM-DD")]
        end_date: Option<String>,
    },
    Peruse {
        // output of total reservoir capacity
        #[arg(long, short, value_name = "SUMMATION_FILE")]
        summation_output: Option<PathBuf>,
        // output of each reservoir's capacity
        #[arg(long, short, value_name = "RESERVOIR_FILE")]
        reservoir_output: Option<PathBuf>,
        // output of each reservoir's water years capacities
        #[arg(long, short, value_name = "WATER_YEARS_FILE")]
        water_years_output: Option<PathBuf>,
        // output of each reservoir's water years Min/Max
        #[arg(long, short, value_name = "MIN_MAX_FILE")]
        min_max_output: Option<PathBuf>,
        // date of earliest data to be collected
        #[arg(long, value_name = "YYYY-MM-DD")]
        start_date: Option<String>,
        // date of latest data to be collected
        #[arg(long, value_name = "YYYY-MM-DD")]
        end_date: Option<String>,
    },
}
