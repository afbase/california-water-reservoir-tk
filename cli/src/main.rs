
use cmd::{
    Commands,
    query::Query,
    survey::Survey,
};
use clap::Parser;
use log::LevelFilter;
use my_log::MY_LOGGER;
use utils::Run;

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
