use std::{path::PathBuf, str::FromStr, process};
use clap::{Parser, Subcommand};
use chrono::{NaiveDate, Local, DateTime, format::ParseError};

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
    eprintln!("{date_type} Date Error: {err_kind}");
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
            end_date: Option<String>
        },
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Query { 
            output,
            start_date,
            end_date
         } => {
            let file_path = match output {
                None => {
                    let file_path = PathBuf::from_str(DEFAULT_OUTPUT_PATH);
                    file_path.unwrap()
                }
                Some(file_path) => {
                    file_path
                }
            };
            let start_date_final = match start_date {
                None => {
                    //Oldest Reservoir Record is
                    //LGT,Lagunitas,Lagunitas Lake,Lagunitas Creek,341,1925
                    NaiveDate::from_ymd(1925, 1, 1)
                },
                Some(start_date_string) => {
                    NaiveDate::parse_from_str(start_date_string.as_str(), "%Y-%m-%d").unwrap_or_else(|err| date_error("Start".to_string(), err))
                }
            };

            let end_date_final = match end_date {
                None => {
                    // Get Today's Date
                    let now = Local::now();
                    now.date_naive()
                },
                Some(end_date_string) => {
                    NaiveDate::parse_from_str(end_date_string.as_str(), "%Y-%m-%d").unwrap_or_else(|err| date_error("End".to_string(), err))
                }
            };
        }
    }
}