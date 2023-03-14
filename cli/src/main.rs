
use cmd::{
    query::{Query, QueryError},
    survey::{Survey}
};
use cdec::{
    observable::{
        CompressedSurveyBuilder, InterpolateObservableRanges, MonthDatum, ObservableRange,
    },
    reservoir::Reservoir,
    survey::CompressedStringRecord,
};
use chrono::{format::ParseError, DateTime, Local, NaiveDate, Utc};
use clap::{Parser, Subcommand};
use csv::{StringRecord, Writer};
use easy_cast::Cast;
use futures::future::join_all;
use reqwest::Client;
use std::{
    collections::HashSet,
    collections::{BTreeMap, HashMap},
    io::Write,
    path::PathBuf,
    process,
    str::FromStr,
};
use utils::error::date_error;


const DEFAULT_OUTPUT_PATH: &str = "output.tar.xz";

#[derive(Parser)]
#[command(name = "cdec-tk", author, version, about = "Query CA CDEC Water Reservoir API", long_about = None)]
struct Cli {
    /// Optional name to operate on
    name: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}





#[tokio::main]
async fn main() {
    log::set_logger(&MY_LOGGER).unwrap();
    log::set_max_level(LevelFilter::Info);
    let args = Cli::parse();

    match args.command {
        Some(Commands::Query {
            output,
            start_date,
            end_date,
            summation,
        }) => {
            let query = query::Query {
                output,
            start_date,
            end_date,
            summation
            };
            query.run();
        }, 
        Some(Commands::Survey { existing_data_input, summation_output, reservoir_output, start_date, end_date }) => {
            let survey = Survey {
                existing_data_input, summation_output, reservoir_output, start_date, end_date
            };
            survey.run();
        }, 
        None => panic!("must specify a subcommand!"),
    }
}
