mod query;

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
use log::{info, Level, LevelFilter, Metadata, Record};
use reqwest::Client;
use std::{
    collections::HashSet,
    collections::{BTreeMap, HashMap},
    io::Write,
    path::PathBuf,
    process,
    str::FromStr,
};
static MY_LOGGER: MyLogger = MyLogger;

struct MyLogger;

impl log::Log for MyLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        let now: DateTime<Utc> = Utc::now();
        if self.enabled(record.metadata()) {
            println!(
                "[{}] {} - {}",
                now.to_rfc3339(),
                record.level(),
                record.args()
            );
        }
    }
    fn flush(&self) {}
}

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

fn date_error(date_type: String, err: ParseError) {
    let err_kind = err.kind();
    eprintln!("{date_type} Date Error: {err_kind:?}");
    eprintln!("Date must be of YYYY-MM-DD format");
    process::exit(1);
}

#[derive(Subcommand)]
enum Commands {
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
    }
    /// does testing things
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

async fn get_surveys_of_reservoirs(
    start_date: &NaiveDate,
    end_date: &NaiveDate,
) -> Vec<ObservableRange> {
    // 1. get observations from date range
    let reservoirs = Reservoir::get_reservoir_vector();
    let client = Client::new();
    let surveys = join_all(reservoirs.into_iter().map(|reservoir| {
        let client_ref = &client;
        let start_date_ref = start_date;
        let end_date_ref = end_date;
        async move {
            reservoir
                .get_surveys_v2(client_ref, start_date_ref, end_date_ref)
                .await
        }
    }))
    .await;
    surveys.into_iter().flatten().collect::<Vec<_>>()
}

async fn run_csv_v2(start_date: &NaiveDate, end_date: &NaiveDate) -> String {
    let reservoirs: HashMap<String, Reservoir> = Reservoir::get_reservoir_vector()
        .iter()
        .map(|res| {
            let station = res.station_id.clone();
            let res_copy = res.clone();
            (station, res_copy)
        })
        .collect();
    info!("{} Reservoirs Loaded", reservoirs.len());
    let mut all_reservoir_observations = get_surveys_of_reservoirs(start_date, end_date).await;
    info!("Surveyed Reseroirs: {}", all_reservoir_observations.len());
    info!("Observations Downloaded");
    all_reservoir_observations.interpolate_reservoir_observations();
    info!(
        "Interpolated Reseroirs: {}",
        all_reservoir_observations.len()
    );
    info!("Observations Interpolated and Sorted");
    let mut california_water_level_observations: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for observable_range in all_reservoir_observations {
        for survey in observable_range.observations {
            let tap = survey.get_tap();
            let date_observation = tap.date_observation;
            let station_id = tap.station_id.clone();
            let recording = survey.get_value();
            let reservoir = reservoirs.get(&station_id).unwrap();
            let reservoir_capacity: f64 = reservoir.capacity.cast();
            let observed_value = recording.min(reservoir_capacity);
            california_water_level_observations
                .entry(date_observation)
                .and_modify(|e| *e += observed_value)
                .or_insert(observed_value);
        }
    }
    info!("Observations Accumulated");
    let mut writer = Writer::from_writer(vec![]);
    for (date, observation) in california_water_level_observations {
        let date_string = date.format("%Y%m%d").to_string();
        let date_str = date_string.as_str();
        let observation_string = observation.to_string();
        let observation_str = observation_string.as_str();
        let string_record = StringRecord::from(vec![date_str, observation_str]);
        if writer
            .write_byte_record(string_record.as_byte_record())
            .is_err()
        {
            panic!("Error: writing record failed");
        }
    }
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
}

async fn run_csv(start_date: &NaiveDate, end_date: &NaiveDate) -> String {
    let mut all_reservoir_observations = get_surveys_of_reservoirs(start_date, end_date).await;
    let option_of_compressed_string_records = all_reservoir_observations
        .iter_mut()
        .map(|surveys| {
            surveys.observations.sort();
            let earliest_date = {
                if let Some(survey_first) = surveys.observations.first() {
                    let tap = survey_first.get_tap();
                    tap.date_observation
                } else {
                    return None;
                }
            };
            let last_survey = surveys.observations.last().unwrap();
            let last_tap = last_survey.get_tap();
            let most_recent_date = last_tap.date_observation;
            let month_datum: HashSet<MonthDatum> = HashSet::new();
            let mut observable_range = ObservableRange {
                observations: surveys.observations.clone(),
                start_date: earliest_date,
                end_date: most_recent_date,
                month_datum,
            };
            observable_range.retain();
            let records: Vec<CompressedStringRecord> = observable_range
                .observations
                .into_iter()
                .map(|survey| {
                    let record: CompressedStringRecord = survey.into();
                    record
                })
                .collect::<Vec<CompressedStringRecord>>();
            Some(records)
        })
        .collect::<Vec<_>>();
    //compressedstringrecords from hear on out
    let mut writer = Writer::from_writer(vec![]);
    let flattened_records = option_of_compressed_string_records.into_iter().flatten();
    for reservoir_records in flattened_records {
        for reservoir_record in reservoir_records {
            if writer
                .write_byte_record(reservoir_record.0.as_byte_record())
                .is_err()
            {
                panic!("Error: writing record failed");
            }
        }
    }
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
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
            let query = Query {
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
