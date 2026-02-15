//! Query result model structs for water and snow data.
//!
//! All structs derive `Serialize` so they can be passed to D3.js as JSON
//! from the Dioxus WASM frontend.

use serde::Serialize;

/// A single (date, value) pair used for line chart data points.
///
/// The `value` field represents acre-feet (AF) for water data
/// or snow water equivalent (SWE) in inches for snow data.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DateValue {
    pub date: String,
    pub value: f64,
}

/// A (station_id, date, value) triple for multi-line reservoir charts.
///
/// Each point identifies which station the observation belongs to,
/// enabling the chart to draw one line per reservoir.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StationDateValue {
    pub station_id: String,
    pub date: String,
    pub value: f64,
}

/// A single data point within a water year overlay chart.
///
/// Water years run from October 1 to September 30. The `day_of_year`
/// field normalizes dates so that October 1 = day 0 and September 30 = day 364,
/// allowing multiple years to be overlaid on the same x-axis.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WaterYearData {
    /// The water year (e.g., 2023 means Oct 2022 - Sep 2023).
    pub year: i32,
    /// Day within the water year (0 = Oct 1, 364 = Sep 30).
    pub day_of_year: i32,
    /// The original calendar date (YYYYMMDD format).
    pub date: String,
    /// Storage value in acre-feet (AF) or SWE in inches.
    pub value: f64,
}

/// Per-year min/max statistics for the water years chart.
///
/// The `is_driest` and `is_wettest` flags are computed dynamically
/// based on which year had the lowest minimum or highest maximum
/// across all years for that station.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WaterYearStats {
    /// The water year.
    pub year: i32,
    /// Calendar date of the lowest observation in this water year.
    pub date_lowest: String,
    /// The lowest storage value observed during this water year (AF or SWE).
    pub lowest_value: f64,
    /// Calendar date of the highest observation in this water year.
    pub date_highest: String,
    /// The highest storage value observed during this water year (AF or SWE).
    pub highest_value: f64,
    /// True if this year had the overall lowest minimum across all years.
    pub is_driest: bool,
    /// True if this year had the overall highest maximum across all years.
    pub is_wettest: bool,
}

/// Reservoir metadata for selection lists and chart labels.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReservoirInfo {
    /// CDEC station identifier (e.g. "SHA" for Shasta).
    pub station_id: String,
    /// Dam name.
    pub dam: String,
    /// Lake name.
    pub lake: String,
    /// Storage capacity in acre-feet (AF).
    pub capacity: i32,
}

/// Snow station metadata for selection lists and chart labels.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SnowStationInfo {
    /// CDEC station identifier.
    pub station_id: String,
    /// Station name.
    pub name: String,
    /// Elevation in feet.
    pub elevation: i32,
    /// River basin name.
    pub river_basin: String,
}
