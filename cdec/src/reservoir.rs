use crate::{
    observable::{CompressedSurveyBuilder, ObservableRange},
    survey::Survey,
};
use chrono::NaiveDate;
use csv::ReaderBuilder;
use reqwest::Client;
use std::include_str;

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
    fn response_to_surveys(&self, start_date: &NaiveDate, end_date: &NaiveDate) -> ObservableRange;
}

impl StringRecordsToSurveys for String {
    fn response_to_surveys(&self, start_date: &NaiveDate, end_date: &NaiveDate) -> ObservableRange {
        let observations = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(self.as_bytes())
            .records()
            .map(|x| {
                let string_record = x.expect("failed record parse");
                let survey: Survey = string_record.try_into().unwrap();
                survey
            })
            .collect::<Vec<Survey>>();
        ObservableRange {
            observations,
            start_date: *start_date,
            end_date: *end_date,
        }
    }
}

fn get_default_year<'life>() -> &'life str {
    "3000"
}
fn get_default_capacity<'life>() -> &'life str {
    "0"
}

impl Reservoir {
    pub async fn get_surveys(
        &self,
        client: &Client,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Vec<Survey> {
        let start_date_str = start_date.format(YEAR_FORMAT);
        let end_date_str = end_date.format(YEAR_FORMAT);
        let daily_rate = "D";
        let monthly_rate = "M";
        let daily_url = format!("http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}", self.station_id.as_str(), daily_rate, start_date_str, end_date_str);
        let daily_response = client.get(daily_url).send().await.unwrap();
        let daily_response_body = daily_response.text().await.unwrap();
        let monthly_url = format!("http://cdec.water.ca.gov/dynamicapp/req/CSVDataServlet?Stations={}&SensorNums=15&dur_code={}&Start={}&End={}", self.station_id.as_str(), monthly_rate, start_date_str, end_date_str);
        let monthly_response = client.get(monthly_url).send().await.unwrap();
        let monthly_response_body = monthly_response.text().await.unwrap();
        let mut daily_observation_range =
            daily_response_body.response_to_surveys(start_date, end_date);
        let monthly_observation_range =
            monthly_response_body.response_to_surveys(start_date, end_date);
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
            // println!("{}", rho.as_slice());
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
