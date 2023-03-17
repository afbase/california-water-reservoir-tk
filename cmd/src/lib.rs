pub mod query;
pub mod survey;
pub mod run;
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
        /// Sets a output file
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        start_date: Option<String>,
        end_date: Option<String>,
        #[arg(short, long)]
        summation: bool,
    },
}