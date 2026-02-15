//! Command implementations for CWR CLI.
//!
//! Provides subcommands for querying CDEC water and snow data,
//! with support for incremental data fetching.

use clap::Subcommand;

pub mod query;
pub mod incremental;

#[derive(Subcommand)]
pub enum Command {
    /// Query CDEC for water reservoir data
    Query {
        /// Output path for per-reservoir observations CSV
        #[arg(short = 'r', long)]
        reservoirs_csv: String,

        /// Only include California reservoirs (exclude Mead/Powell)
        #[arg(long)]
        california_only: bool,
    },

    /// Incrementally update existing CSV data (only fetch new data since last entry)
    IncrementalQuery {
        /// Path to existing per-reservoir observations CSV (will be updated in-place)
        #[arg(short = 'r', long)]
        reservoirs_csv: String,

        /// Only include California reservoirs (exclude Mead/Powell)
        #[arg(long)]
        california_only: bool,
    },

    /// Query CDEC for snow sensor data
    SnowQuery {
        /// Output path for per-station snow observations CSV
        #[arg(short = 't', long)]
        stations_csv: String,
    },
}

pub async fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Query {
            reservoirs_csv,
            california_only,
        } => {
            query::run_query(&reservoirs_csv, california_only).await
        }
        Command::IncrementalQuery {
            reservoirs_csv,
            california_only,
        } => {
            incremental::run_incremental(&reservoirs_csv, california_only).await
        }
        Command::SnowQuery {
            stations_csv,
        } => {
            query::run_snow_query(&stations_csv).await
        }
    }
}
