use dioxus_logger::tracing::info;
use serde::{Deserialize, Serialize};

const COMPRESSED_DATA: &[u8] = include_bytes!("../data/reservoir_data.json.zst");

#[derive(Serialize, Deserialize)]
struct DataFile {
    observations: Vec<(String, u32)>,
}

#[derive(Clone, PartialEq)]
pub struct Database {
    data: Vec<(String, u32)>,
}

impl Database {
    pub async fn new() -> Result<Self, String> {
        info!("Decompressing data...");

        // Decompress the data
        let decompressed = zstd::decode_all(COMPRESSED_DATA)
            .map_err(|e| format!("Failed to decompress data: {}", e))?;

        info!("Data decompressed, size: {} bytes", decompressed.len());

        // Parse JSON
        let data_file: DataFile = serde_json::from_slice(&decompressed)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        info!("Loaded {} observations", data_file.observations.len());

        Ok(Database {
            data: data_file.observations,
        })
    }

    pub async fn get_date_range(&self) -> Result<(String, String), String> {
        if self.data.is_empty() {
            return Err("No data available".to_string());
        }

        let min_date = self.data.first()
            .map(|(date, _)| date.clone())
            .ok_or("Failed to get min date")?;

        let max_date = self.data.last()
            .map(|(date, _)| date.clone())
            .ok_or("Failed to get max date")?;

        Ok((min_date, max_date))
    }

    pub async fn get_data(&self, start_date: &str, end_date: &str) -> Result<Vec<(String, u32)>, String> {
        let filtered: Vec<(String, u32)> = self.data
            .iter()
            .filter(|(date, _)| date.as_str() >= start_date && date.as_str() <= end_date)
            .cloned()
            .collect();

        info!("Retrieved {} data points for range {} to {}", filtered.len(), start_date, end_date);
        Ok(filtered)
    }
}
