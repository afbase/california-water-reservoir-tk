/// Reservoir data structures and survey fetching logic
use crate::{
    error::{CdecError, Result},
    observable::{MonthDatum, ObservableRange},
    observation::DataRecording,
    survey::Survey,
};
use chrono::NaiveDate;
use csv::ReaderBuilder;
use log::{info, warn};
use reqwest::{Client, StatusCode};
use std::{collections::HashSet, include_str, thread::sleep, time::Duration};

/// Maximum number of retry attempts for HTTP requests
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Initial sleep duration in milliseconds before retrying
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

pub static CSV_OBJECT: &str = include_str!("../../fixtures/capacity.csv");
pub static CSV_OBJECT_NO_POWELL_NO_MEAD: &str =
    include_str!("../../fixtures/capacity-no-powell-no-mead.csv");
/// Date format for API requests
const YEAR_FORMAT: &str = "%Y-%m-%d";

/// Represents a California reservoir with metadata
#[derive(Debug, PartialEq, Clone)]
pub struct Reservoir {
    /// Station identifier (e.g., "SHA" for Shasta)
    pub station_id: String,
    /// Dam name
    pub dam: String,
    /// Lake/reservoir name
    pub lake: String,
    /// Stream name
    pub stream: String,
    /// Total capacity in acre-feet
    pub capacity: i32,
    /// Year the reservoir was filled
    pub fill_year: i32,
}

/// Trait for converting HTTP response strings to survey data
trait StringRecordsToSurveys {
    /// Parses CSV response into an ObservableRange
    fn response_to_surveys(&self) -> Result<ObservableRange>;
}

impl StringRecordsToSurveys for String {
    fn response_to_surveys(&self) -> Result<ObservableRange> {
        let mut m: HashSet<MonthDatum> = HashSet::new();
        let mut observations = Vec::new();

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(self.as_bytes());

        for record_result in rdr.records() {
            let string_record = record_result?;
            let survey: Survey = string_record.try_into()?;
            let tap = survey.get_tap();

            if let DataRecording::Recording(_) = tap.value {
                let month_date = survey.as_month_datum();
                m.insert(month_date);
                observations.push(survey);
            }
        }

        observations.sort();

        if observations.is_empty() {
            return Err(CdecError::InvalidFormat("No valid observations found in response".to_string()));
        }

        // Extract dates before moving observations
        let start_date = observations
            .first()
            .map(|s| s.get_tap().date_observation)
            .ok_or_else(|| CdecError::InvalidFormat("Empty observations after filtering".to_string()))?;

        let end_date = observations
            .last()
            .map(|s| s.get_tap().date_observation)
            .ok_or_else(|| CdecError::InvalidFormat("Empty observations after filtering".to_string()))?;

        Ok(ObservableRange {
            observations,
            start_date,
            end_date,
            month_datum: m,
        })
    }
}

fn get_default_year<'life>() -> &'life str {
    "3000"
}
fn get_default_capacity<'life>() -> &'life str {
    "0"
}

impl Reservoir {
    /// Fetches survey data with retry logic and exponential backoff
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client (reuse for multiple requests)
    /// * `start_date` - Start date for data range
    /// * `end_date` - End date for data range (inclusive)
    /// * `duration_type` - Either "D" for daily or "M" for monthly
    ///
    /// # Returns
    ///
    /// `Ok(Some(ObservableRange))` if data was successfully fetched
    /// `Ok(None)` if all retry attempts failed (non-critical failure)
    async fn get_survey_general(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
        duration_type: &str,
    ) -> Option<ObservableRange> {
        let mut sleep_millis = INITIAL_RETRY_DELAY_MS;
        let start_date_str = start_date.format(YEAR_FORMAT);
        let end_date_str = end_date.format(YEAR_FORMAT);

        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            let url = format!(
                "http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}",
                self.station_id.as_str(), duration_type, start_date_str, end_date_str
            );

            match client.get(&url).send().await {
                Ok(response) => {
                    if response.status() != StatusCode::OK {
                        warn!(
                            "Attempt {}/{}: Bad response status for {}: {}",
                            attempt,
                            MAX_RETRY_ATTEMPTS,
                            self.dam,
                            response.status()
                        );
                    } else {
                        match response.text().await {
                            Ok(response_body) => {
                                if response_body.len() <= 2 {
                                    warn!(
                                        "Attempt {}/{}: Empty response for {}",
                                        attempt, MAX_RETRY_ATTEMPTS, self.dam
                                    );
                                } else {
                                    match response_body.response_to_surveys() {
                                        Ok(surveys) => return Some(surveys),
                                        Err(e) => {
                                            warn!(
                                                "Attempt {}/{}: Failed to parse response for {}: {}",
                                                attempt, MAX_RETRY_ATTEMPTS, self.dam, e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Attempt {}/{}: Failed to read response body for {}: {}",
                                    attempt, MAX_RETRY_ATTEMPTS, self.dam, e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Attempt {}/{}: Request failed for {}: {}",
                        attempt, MAX_RETRY_ATTEMPTS, self.dam, e
                    );
                }
            }

            if attempt < MAX_RETRY_ATTEMPTS {
                info!(
                    "Sleeping for {} milliseconds before retry for {}",
                    sleep_millis, self.dam
                );
                sleep(Duration::from_millis(sleep_millis));
                sleep_millis *= 2; // Exponential backoff
            }
        }

        warn!("All attempts failed for {}", self.dam);
        None
    }

    /// Fetches monthly survey data for this reservoir
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client (reuse for multiple requests)
    /// * `start_date` - Start date
    /// * `end_date` - End date (inclusive)
    ///
    /// # Returns
    ///
    /// `Some(ObservableRange)` with monthly observations, or `None` if fetching failed
    pub async fn get_monthly_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Option<ObservableRange> {
        self.get_survey_general(client, start_date, end_date, "M")
            .await
    }

    /// Fetches daily survey data for this reservoir
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client (reuse for multiple requests)
    /// * `start_date` - Start date
    /// * `end_date` - End date (inclusive)
    ///
    /// # Returns
    ///
    /// `Some(ObservableRange)` with daily observations, or `None` if fetching failed
    pub async fn get_daily_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Option<ObservableRange> {
        self.get_survey_general(client, start_date, end_date, "D")
            .await
    }

    /// Fetches both daily and monthly surveys, combining them intelligently
    ///
    /// This method fetches both daily and monthly data and merges them, preferring
    /// daily data when available and filling gaps with monthly data.
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client (reuse for multiple requests)
    /// * `start_date` - Start date
    /// * `end_date` - End date (inclusive)
    ///
    /// # Returns
    ///
    /// Combined ObservableRange, or `None` if both daily and monthly fetches failed
    pub async fn get_surveys_v2(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Option<ObservableRange> {
        let daily_observables = self.get_daily_surveys(client, start_date, end_date).await;
        let monthly_observables = self.get_monthly_surveys(client, start_date, end_date).await;
        match (daily_observables, monthly_observables) {
            (Some(mut daily), Some(monthly)) => {
                for survey in monthly.observations {
                    let monthly_datum = survey.as_month_datum();
                    let is_monthly_datum_in_dailies = daily.month_datum.contains(&monthly_datum);
                    if !is_monthly_datum_in_dailies {
                        daily.observations.push(survey);
                    }
                }
                Some(daily)
            }
            (Some(daily), None) => Some(daily),
            (None, Some(monthly)) => Some(monthly),
            (None, None) => None,
        }
    }

    /// Fetches and combines surveys (deprecated - use get_surveys_v2 instead)
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client
    /// * `start_date` - Start date
    /// * `end_date` - End date (inclusive)
    ///
    /// # Returns
    ///
    /// Combined vector of Survey objects, or empty vector if fetching failed
    ///
    /// # Panics
    ///
    /// This method may panic on network failures. Consider using `get_surveys_v2` instead.
    #[deprecated(since = "1.2.0", note = "Use get_surveys_v2 instead")]
    pub async fn get_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Vec<Survey> {
        match self.get_surveys_v2(client, start_date, end_date).await {
            Some(range) => range.observations,
            None => Vec::new(),
        }
    }
    /// Returns all California reservoirs from embedded CSV data
    ///
    /// Parses the embedded capacity.csv file containing metadata for 218 reservoirs.
    ///
    /// # Returns
    ///
    /// Vector of all reservoirs
    ///
    /// # Errors
    ///
    /// Returns `CdecError::CsvParse` if the embedded data is corrupted
    pub fn get_reservoir_vector() -> Result<Vec<Reservoir>> {
        Reservoir::parse_reservoir_csv(CSV_OBJECT)
    }

    /// Returns reservoirs from a custom CSV string
    ///
    /// Allows loading reservoir data from alternative sources (e.g., excluding
    /// Powell and Mead reservoirs).
    ///
    /// # Arguments
    ///
    /// * `reservoir_csv` - CSV string with reservoir data
    ///
    /// # Returns
    ///
    /// Vector of reservoirs parsed from the CSV
    ///
    /// # Errors
    ///
    /// Returns `CdecError::CsvParse` if the CSV is malformed
    pub fn get_reservoir_vector_v2(reservoir_csv: &str) -> Result<Vec<Reservoir>> {
        Reservoir::parse_reservoir_csv(reservoir_csv)
    }

    fn parse_int(ess: &str) -> i32 {
        let ess_lowered = ess.trim().to_lowercase();
        let ess_lowered_str = ess_lowered.as_str();
        match ess_lowered_str {
            "null" => 0i32,
            "" => 0i32,
            "n/a" => 0i32,
            "na" => 0i32,
            s => s.parse::<i32>().unwrap_or_default(),
        }
    }

    /// Parses reservoir CSV data into Reservoir structs
    ///
    /// # Arguments
    ///
    /// * `csv_object` - CSV string containing reservoir data
    ///
    /// # Returns
    ///
    /// Vector of parsed reservoirs
    ///
    /// # Errors
    ///
    /// Returns `CdecError::CsvParse` for CSV parsing errors
    /// Returns `CdecError::InvalidFormat` for missing required fields
    fn parse_reservoir_csv(csv_object: &str) -> Result<Vec<Reservoir>> {
        let mut reservoir_list: Vec<Reservoir> = Vec::new();
        let mut rdr = ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(true)
            .from_reader(csv_object.as_bytes());

        for row in rdr.records() {
            let rho = row?;

            let capacity = Reservoir::parse_int(rho.get(4).unwrap_or_else(get_default_capacity));
            let fill_year = Reservoir::parse_int(rho.get(5).unwrap_or_else(get_default_year));

            let reservoir = Reservoir {
                station_id: rho
                    .get(0)
                    .ok_or_else(|| CdecError::InvalidFormat("Missing station_id column".to_string()))?
                    .to_string(),
                dam: rho
                    .get(1)
                    .ok_or_else(|| CdecError::InvalidFormat("Missing dam column".to_string()))?
                    .to_string(),
                lake: rho
                    .get(2)
                    .ok_or_else(|| CdecError::InvalidFormat("Missing lake column".to_string()))?
                    .to_string(),
                stream: rho
                    .get(3)
                    .ok_or_else(|| CdecError::InvalidFormat("Missing stream column".to_string()))?
                    .to_string(),
                capacity,
                fill_year,
            };
            reservoir_list.push(reservoir);
        }
        Ok(reservoir_list)
    }
}

#[cfg(test)]
mod tests {
    use crate::reservoir::Reservoir;

    #[test]
    fn test_reservoir_vector() {
        let reservoirs = Reservoir::get_reservoir_vector()
            .expect("Failed to load reservoir vector");
        assert_eq!(reservoirs.len(), 218);
    }
}
