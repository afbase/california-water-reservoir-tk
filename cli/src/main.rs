use clap::Parser;
use cmd::{peruse::Peruse, query::Query, survey::Survey, Commands};
use log::{info, LevelFilter};
use my_log::MY_LOGGER;
use utils::run::Run;

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
            summation_output,
            reservoir_output,
            start_date,
            end_date,
        }) => {
            let query = Query {
                summation_output,
                reservoir_output,
                start_date,
                end_date,
            };
            info!("hello world");
            query.run().await;
        }
        Some(Commands::Survey {
            existing_data_input,
            summation_output,
            reservoir_output,
            start_date,
            end_date,
        }) => {
            let survey = Survey {
                existing_data_input,
                summation_output,
                reservoir_output,
                start_date,
                end_date,
            };
            survey.run().await;
        }
        Some(Commands::Peruse {
            summation_output,
            reservoir_output,
            water_years_output,
            min_max_output,
            start_date,
            end_date,
        }) => {
            let peruse = Peruse {
                summation_output,
                reservoir_output,
                water_years_output,
                min_max_output,
                start_date,
                end_date,
            };
            peruse.run().await;
        }
        None => panic!("must specify a subcommand!"),
    }
}
