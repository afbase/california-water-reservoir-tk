use cdec::{observation::Observation, reservoir::Reservoir};
use chrono::{format::ParseError, Local, NaiveDate};
use clap::{Parser, Subcommand};
use csv::Writer;
use futures::future::join_all;
use reqwest::Client;
use std::{io::Write, path::PathBuf, process, str::FromStr};
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
    eprintln!("{} Date Error: {:?}", date_type, err_kind);
    eprintln!("Date must be of YYYY-MM-DD format");
    process::exit(1);
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Query {
        /// Sets a output file
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        start_date: Option<String>,
        end_date: Option<String>,
    },
}

async fn run_csv(start_date: &NaiveDate, end_date: &NaiveDate) -> String {
    // 1. get observations from date range
    let reservoirs = Reservoir::get_reservoir_vector();
    let client = Client::new();
    let all_reservoir_observations = join_all(reservoirs.iter().map(|reservoir| {
        let client_ref = &client;
        let start_date_ref = start_date;
        let end_date_ref = end_date;
        async move {
            Observation::get_string_records(
                client_ref,
                reservoir.station_id.as_str(),
                start_date_ref,
                end_date_ref,
            )
            .await
        }
    }))
    .await;
    let mut writer = Writer::from_writer(vec![]);
    for reservoir_records in all_reservoir_observations {
        let records = reservoir_records.unwrap();
        // writer.write_byte_record(records.iter());
        for record in records {
            if writer.write_byte_record(record.as_byte_record()).is_err() {
                panic!("Error: writiing record failed");
            }
        }
    }
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Query {
            output,
            start_date,
            end_date,
        }) => {
            let file_path = match output {
                None => {
                    let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                    file_path.unwrap()
                }
                Some(file_path) => file_path,
            };
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

            let csv_out = run_csv(&start_date_final, &end_date_final).await;
            let mut fs = std::fs::File::create(file_path.as_path()).unwrap();
            if fs.write_all(csv_out.as_bytes()).is_err() {
                panic!("writing csv file failed");
            }
        }
        None => panic!("must specify a subcommand!"),
    }
}
