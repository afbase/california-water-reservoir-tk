use crate::{
    observable::{CompressedSurveyBuilder, MonthDatum, ObservableRange},
    observation::DataRecording,
    survey::Survey,
};
use chrono::NaiveDate;
use csv::ReaderBuilder;
use reqwest::Client;
use std::{collections::HashSet, include_str};

static CSV_OBJECT: &str = include_str!("../../fixtures/capacity.csv");
const YEAR_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, PartialEq, Clone)]
pub struct Reservoir {
    pub station_id: String,
    pub dam: String,
    pub lake: String,
    pub stream: String,
    pub capacity: i32,
    pub fill_year: i32,
}

trait StringRecordsToSurveys {
    fn response_to_surveys(&self) -> Option<ObservableRange>;
}

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
    async fn get_survey_general(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
        duration_type: &str,
    ) -> Option<ObservableRange> {
        let start_date_str = start_date.format(YEAR_FORMAT);
        let end_date_str = end_date.format(YEAR_FORMAT);
        let url = format!("http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}", self.station_id.as_str(), duration_type, start_date_str, end_date_str);
        let response = client.get(url).send().await.unwrap();
        let response_body = response.text().await.unwrap();
        response_body.response_to_surveys()
    }
    pub async fn get_monthly_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Option<ObservableRange> {
        self.get_survey_general(client, start_date, end_date, "M")
            .await
    }
    pub async fn get_daily_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Option<ObservableRange> {
        self.get_survey_general(client, start_date, end_date, "D")
            .await
    }

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
        let monthly_url = format!("http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}", self.station_id.as_str(), monthly_rate, start_date_str, end_date_str);
        let monthly_response = client.get(monthly_url).send().await.unwrap();
        let monthly_response_body = monthly_response.text().await.unwrap();
        let daily_url = format!("http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}", self.station_id.as_str(), daily_rate, start_date_str, end_date_str);
        let daily_response = client.get(daily_url).send().await.unwrap();
        let daily_response_body = daily_response.text().await.unwrap();
        let mut daily_observation_range = daily_response_body.response_to_surveys().unwrap();
        let monthly_observation_range = monthly_response_body.response_to_surveys().unwrap();
        // insert the monthlys with update into daily
        for survey in monthly_observation_range.observations {
            daily_observation_range.update(survey);
        }
        daily_observation_range.retain();
        daily_observation_range.observations
    }
    // collects reservoir information from https://raw.githubusercontent.com/afbase/california-water/main/obj/capacity.csv
    pub fn get_reservoir_vector() -> Vec<Reservoir> {
        if let Ok(r) = Reservoir::parse_reservoir_csv() {
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
            s => {
                if let Ok(c) = s.parse::<i32>() {
                    c
                } else {
                    0i32
                }
            }
        }
    }

    fn parse_reservoir_csv() -> Result<Vec<Reservoir>, std::io::Error> {
        let mut reservoir_list: Vec<Reservoir> = Vec::new();
        let mut rdr = ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(true)
            .from_reader(CSV_OBJECT.as_bytes());
        for row in rdr.records() {
            let rho = row?;
            let capacity = Reservoir::parse_int(rho.get(4).unwrap_or_else(get_default_capacity));
            let fill_year = Reservoir::parse_int(rho.get(5).unwrap_or_else(get_default_year));
            let reservoir = Reservoir {
                station_id: String::from(rho.get(0).expect("station_id parse fail")),
                dam: String::from(rho.get(1).expect("damn parse fail")),
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
