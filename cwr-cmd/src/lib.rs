//! Command implementations for CWR CLI.
//!
//! Provides subcommands for querying CDEC water and snow data,
//! with support for incremental data fetching.

use clap::Subcommand;

pub mod incremental;
pub mod query;
pub mod split;

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

        /// CSV to read per-station max-dates from (where to resume each station).
        /// Defaults to the value of -r/--reservoirs_csv when omitted.
        #[arg(long)]
        baseline: Option<String>,

        /// Total number of shards to divide the reservoir list into.
        #[arg(long, default_value_t = 1)]
        shards: usize,

        /// This shard's 0-based index (must be < --shards).
        #[arg(long, default_value_t = 0)]
        shard: usize,
    },

    /// Query CDEC for snow sensor data
    SnowQuery {
        /// Output path for per-station snow observations CSV
        #[arg(short = 't', long)]
        stations_csv: String,
    },

    /// Split a monolithic observations CSV into per-station, delta-encoded,
    /// brotli-compressed files (`observations_<ID>.csv.br`) plus a manifest.
    SplitObservations {
        /// Path to the monolithic observations CSV to read.
        #[arg(short = 'i', long)]
        input: String,

        /// Directory to write per-station `.csv.br` files and the manifest into.
        #[arg(short = 'o', long)]
        output_dir: String,

        /// Minimum fraction (0.0..=1.0) of the full reservoir list that must be
        /// written, or the command exits non-zero. Omitted = no coverage gate.
        #[arg(long)]
        min_coverage: Option<f64>,
    },
}

pub async fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Query {
            reservoirs_csv,
            california_only,
        } => query::run_query(&reservoirs_csv, california_only).await,
        Command::IncrementalQuery {
            reservoirs_csv,
            california_only,
            baseline,
            shards,
            shard,
        } => {
            incremental::run_incremental(
                &reservoirs_csv,
                california_only,
                baseline.as_deref(),
                shards,
                shard,
            )
            .await
        }
        Command::SnowQuery { stations_csv } => query::run_snow_query(&stations_csv).await,
        Command::SplitObservations {
            input,
            output_dir,
            min_coverage,
        } => split::run_split(&input, &output_dir, min_coverage),
    }
}
