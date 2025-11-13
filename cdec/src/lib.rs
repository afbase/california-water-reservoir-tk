

//! # CDEC (California Data Exchange Center) Library
//!
//! This library provides tools for fetching, processing, and analyzing California reservoir data
//! from the California Data Exchange Center (CDEC).
//!
//! ## Features
//!
//! - Fetch real-time reservoir observations via HTTP API
//! - Process compressed historical data (LZMA/tar archives)
//! - Calculate water year statistics
//! - Normalize water year data for comparison
//! - Generate visualizations with plotters
//!
//! ## Example
//!
//! ```no_run
//! use cdec::reservoir::Reservoir;
//!
//! let reservoirs = Reservoir::get_reservoir_vector().unwrap();
//! println!("Found {} reservoirs", reservoirs.len());
//! ```

pub mod compression;
pub mod date_range;
pub mod error;
pub mod normalized_naive_date;
pub mod observable;
pub mod observation;
pub mod reservoir;
pub mod reservoir_observations;
pub mod survey;
pub mod water_year;

// Re-export commonly used types
pub use error::{CdecError, Result};
