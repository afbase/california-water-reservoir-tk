use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

#[cfg(feature = "api")]
use crate::{
    observable::{CompressedSurveyBuilder, MonthDatum, ObservableRange},
    observation::DataRecording,
    survey::Survey,
};
#[cfg(feature = "api")]
use chrono::NaiveDate;
#[cfg(feature = "api")]
use log::{info, warn};
#[cfg(feature = "api")]
use reqwest::{Client, StatusCode};
#[cfg(feature = "api")]
use std::collections::HashSet;
#[cfg(feature = "api")]
use std::{thread::sleep, time::Duration};

/// Embedded CSV data for all reservoirs (including Lake Powell and Lake Mead).
pub static CSV_OBJECT: &str = include_str!("../../fixtures/capacity.csv");

/// Embedded CSV data for California-only reservoirs (excluding Powell and Mead).
pub static CSV_OBJECT_NO_POWELL_NO_MEAD: &str =
    include_str!("../../fixtures/capacity-no-powell-no-mead.csv");

#[cfg(feature = "api")]
const YEAR_FORMAT: &str = "%Y-%m-%d";

/// Represents a California water reservoir with its CDEC station metadata.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Reservoir {
    pub station_id: String,
    pub dam: String,
    pub lake: String,
    pub stream: String,
    /// Capacity in acre-feet (AF)
    pub capacity: i32,
    /// Year the reservoir was filled
    pub fill_year: i32,
}

#[cfg(feature = "api")]
trait StringRecordsToSurveys {
    fn response_to_surveys(&self) -> Option<ObservableRange>;
}

#[cfg(feature = "api")]
impl StringRecordsToSurveys for String {
    fn response_to_surveys(&self) -> Option<ObservableRange> {
        let mut m: HashSet<MonthDatum> = HashSet::new();
        let mut observations = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(self.as_bytes())
            .records()
            .filter_map(|x| {
                let string_record = x.expect("failed record parse");
                let survey: Survey = string_record.try_into().unwrap();
                let tap = survey.get_tap();
                match tap.value {
                    DataRecording::Recording(_) => {
                        let month_date = survey.as_month_datum();
                        let _yep = m.insert(month_date);
                        Some(survey)
                    }
                    _ => None,
                }
            })
            .collect::<Vec<Survey>>();
        observations.sort();
        let (earliest_date, most_recent_date) = {
            if !observations.is_empty() {
                let first_survey = observations.first().unwrap();
                let first_tap = first_survey.get_tap();
                let last_survey = observations.last().unwrap();
                let last_tap = last_survey.get_tap();
                (first_tap.date_observation, last_tap.date_observation)
            } else {
                return None;
            }
        };
        Some(ObservableRange {
            observations,
            start_date: earliest_date,
            end_date: most_recent_date,
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
    /// Fetch daily and monthly surveys from CDEC, merging monthly data where
    /// daily data is missing, with retry and exponential backoff.
    #[cfg(feature = "api")]
    async fn get_survey_general(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
        duration_type: &str,
    ) -> Option<ObservableRange> {
        let max_tries = 3;
        let mut sleep_millis: u64 = 1000;
        let start_date_str = start_date.format(YEAR_FORMAT);
        let end_date_str = end_date.format(YEAR_FORMAT);

        for attempt in 1..=max_tries {
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
                            max_tries,
                            self.dam,
                            response.status()
                        );
                    } else {
                        match response.text().await {
                            Ok(response_body) => {
                                if response_body.len() <= 2 {
                                    warn!(
                                        "Attempt {}/{}: Empty response for {}",
                                        attempt, max_tries, self.dam
                                    );
                                } else {
                                    return response_body.response_to_surveys();
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Attempt {}/{}: Failed to read response body for {}: {}",
                                    attempt, max_tries, self.dam, e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Attempt {}/{}: Request failed for {}: {}",
                        attempt, max_tries, self.dam, e
                    );
                }
            }

            if attempt < max_tries {
                info!(
                    "Sleeping for {} milliseconds before retry for {}",
                    sleep_millis, self.dam
                );
                sleep(Duration::from_millis(sleep_millis));
                sleep_millis *= 2;
            }
        }

        warn!("All attempts failed for {}", self.dam);
        None
    }

    /// Get monthly surveys from CDEC for this reservoir.
    #[cfg(feature = "api")]
    pub async fn get_monthly_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Option<ObservableRange> {
        self.get_survey_general(client, start_date, end_date, "M")
            .await
    }

    /// Get daily surveys from CDEC for this reservoir.
    #[cfg(feature = "api")]
    pub async fn get_daily_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Option<ObservableRange> {
        self.get_survey_general(client, start_date, end_date, "D")
            .await
    }

    /// Get surveys (daily + monthly merged) from CDEC, v2.
    /// Daily data takes priority; monthly data fills in gaps.
    #[cfg(feature = "api")]
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

    /// Get surveys (daily + monthly merged) from CDEC.
    #[cfg(feature = "api")]
    pub async fn get_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Vec<Survey> {
        let daily_rate = "D";
        let monthly_rate = "M";
        let start_date_str = start_date.format(YEAR_FORMAT);
        let end_date_str = end_date.format(YEAR_FORMAT);
        let monthly_url = format!(
            "http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}",
            self.station_id.as_str(), monthly_rate, start_date_str, end_date_str
        );
        let monthly_response = client.get(monthly_url).send().await.unwrap();
        let monthly_response_body = monthly_response.text().await.unwrap();
        let daily_url = format!(
            "http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}",
            self.station_id.as_str(), daily_rate, start_date_str, end_date_str
        );
        let daily_response = client.get(daily_url).send().await.unwrap();
        let daily_response_body = daily_response.text().await.unwrap();
        let mut daily_observation_range = daily_response_body.response_to_surveys().unwrap();
        let monthly_observation_range = monthly_response_body.response_to_surveys().unwrap();
        for survey in monthly_observation_range.observations {
            daily_observation_range.update(survey);
        }
        daily_observation_range.retain();
        daily_observation_range.observations
    }

    /// Get reservoir vector from the embedded full CSV (including Powell and Mead).
    pub fn get_reservoir_vector() -> Vec<Reservoir> {
        if let Ok(r) = Reservoir::parse_reservoir_csv(CSV_OBJECT) {
            r
        } else {
            panic!("failed to parse csv file")
        }
    }

    /// Get reservoir vector excluding Colorado River reservoirs (Powell and Mead).
    pub fn get_reservoir_vector_no_colorado() -> Vec<Reservoir> {
        if let Ok(r) = Reservoir::parse_reservoir_csv(CSV_OBJECT_NO_POWELL_NO_MEAD) {
            r
        } else {
            panic!("failed to parse csv file (no colorado)")
        }
    }

    /// Get reservoir vector from a custom CSV string.
    pub fn get_reservoir_vector_v2(reservoir: &str) -> Vec<Reservoir> {
        if let Ok(r) = Reservoir::parse_reservoir_csv(reservoir) {
            r
        } else {
            panic!("failed to parse csv file")
        }
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

    /// Parse a CSV string of reservoir data into a vector of Reservoirs.
    ///
    /// Expected CSV columns: station_id, dam, lake, stream, capacity, fill_year
    pub fn parse_reservoir_csv(csv_object: &str) -> Result<Vec<Reservoir>, std::io::Error> {
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
                station_id: String::from(rho.get(0).expect("station_id parse fail")),
                dam: String::from(rho.get(1).expect("dam parse fail")),
                lake: String::from(rho.get(2).expect("lake parse fail")),
                stream: String::from(rho.get(3).expect("stream parse fail")),
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
        let reservoirs: Vec<Reservoir> = Reservoir::get_reservoir_vector();
        assert_eq!(reservoirs.len(), 218);
    }
}
