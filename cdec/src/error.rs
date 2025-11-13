/// Error types for the CDEC library
use thiserror::Error;

/// Main error type for CDEC operations
#[derive(Error, Debug)]
pub enum CdecError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// Failed to parse HTTP response
    #[error("Failed to parse HTTP response: {0}")]
    ResponseParse(String),

    /// Failed to parse CSV data
    #[error("Failed to parse CSV: {0}")]
    CsvParse(#[from] csv::Error),

    /// Failed to decompress data
    #[error("Failed to decompress data: {0}")]
    Decompression(String),

    /// Failed to extract tar archive
    #[error("Failed to extract tar archive: {0}")]
    TarExtraction(#[from] std::io::Error),

    /// Failed to parse observation data
    #[error("Failed to parse observation data")]
    ObservationParse,

    /// Failed to convert observation data
    #[error("Failed to convert observation: {0}")]
    ObservationConversion(String),

    /// Insufficient water year data
    #[error("Insufficient water years available (needed: {needed}, found: {found})")]
    InsufficientWaterYears { needed: usize, found: usize },

    /// Date parsing failed
    #[error("Failed to parse date: {0}")]
    DateParse(String),

    /// Invalid data format
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),

    /// Reservoir not found
    #[error("Reservoir not found: {0}")]
    ReservoirNotFound(String),
}

/// Type alias for Results using CdecError
pub type Result<T> = std::result::Result<T, CdecError>;
