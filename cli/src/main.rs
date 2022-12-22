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
    /// does testing things
    Query {
        /// Sets an output file for cumulative, total reservoir acrefeet for california
        #[arg(short, long, value_name = "FILE")]
        summation_output: Option<PathBuf>,
        /// Sets an output file for reservoir acrefeet per reservoir
        #[arg(short, long, value_name = "FILE")]
        reservoir_output: Option<PathBuf>,
        start_date: Option<String>,
        end_date: Option<String>,
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

async fn run_csv_v2(all_reservoir_observations: &mut Vec<ObservableRange>) -> String {
    let reservoirs: HashMap<String, Reservoir> = Reservoir::get_reservoir_vector()
        .iter()
        .map(|res| {
            let station = res.station_id.clone();
            let res_copy = res.clone();
            (station, res_copy)
        })
        .into_iter()
        .collect();
    info!("{} Reservoirs Loaded", reservoirs.len());
    // let mut all_reservoir_observations = get_surveys_of_reservoirs(start_date, end_date).await;
    info!("Surveyed Reseroirs: {}", all_reservoir_observations.len());
    all_reservoir_observations.interpolate_reservoir_observations();
    info!(
        "Interpolated Reseroirs: {}",
        all_reservoir_observations.len()
    );
    info!("Observations Interpolated and Sorted");
    let mut california_water_level_observations: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for observable_range in all_reservoir_observations.clone() {
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

async fn run_csv(all_reservoir_observations: &mut [ObservableRange]) -> String {
    // let mut all_reservoir_observations = get_surveys_of_reservoirs(start_date, end_date).await;
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
                panic!("Error: writiing record failed");
            }
        }
    }
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
}

#[tokio::main]
async fn main() {
    log::set_logger(&MY_LOGGER).unwrap();
    log::set_max_level(LevelFilter::Trace);
    let args = Cli::parse();
    let mut file_written = false;
    match args.command {
        Some(Commands::Query {
            summation_output,
            reservoir_output,
            start_date,
            end_date,
        }) => {
            let start_date_final = match start_date {
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

            let end_date_final = match end_date {
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
            let mut all_reservoir_observations =
                get_surveys_of_reservoirs(&start_date_final, &end_date_final).await;
            info!("Observations Downloaded");
            if let Some(file_path) = summation_output {
                let csv_out = run_csv_v2(&mut all_reservoir_observations).await;
                let mut fs = std::fs::File::create(file_path.as_path()).unwrap();
                if fs.write_all(csv_out.as_bytes()).is_err() {
                    panic!("writing csv file failed");
                }
                info!("Observations Written to CSV");
                file_written = true;
            };
            if let Some(file_path) = reservoir_output {
                let csv_out = run_csv(&mut all_reservoir_observations).await;
                let mut fs = std::fs::File::create(file_path.as_path()).unwrap();
                if fs.write_all(csv_out.as_bytes()).is_err() {
                    panic!("writing csv file failed");
                }
                info!("Observations Written to CSV");
                file_written = true;
            };
            if !file_written {
                panic!("use -s or -r to output reservoir details");
            }
        }
        None => panic!("must specify a subcommand! Try query."),
    }
}
