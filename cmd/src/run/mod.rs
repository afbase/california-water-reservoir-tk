use cdec::{
    observable::{
        CompressedSurveyBuilder, InterpolateObservableRanges, MonthDatum, ObservableRange,
    },
    reservoir::Reservoir,
    survey::CompressedStringRecord,
};
use chrono::NaiveDate;
use csv::{StringRecord, Writer};
use easy_cast::Cast;
use futures::future::join_all;
use log::info;
use reqwest::Client;
use std::{
    collections::HashSet,
    collections::{BTreeMap, HashMap},
};

pub async fn get_surveys_of_reservoirs(
    start_date: &NaiveDate,
    end_date: &NaiveDate,
) -> Vec<ObservableRange> {
    // 1. get observations from date range
    let reservoirs = Reservoir::get_reservoir_vector();
    let client = Client::new();
    let surveys = join_all(reservoirs.into_iter().map(|reservoir| {
        let client_ref = &client;
        let start_date_ref = start_date;
        let end_date_ref = end_date;
        async move {
            reservoir
                .get_surveys_v2(client_ref, start_date_ref, end_date_ref)
                .await
        }
    }))
    .await;
    surveys.into_iter().flatten().collect::<Vec<_>>()
}

pub async fn run_csv_v2(start_date: &NaiveDate, end_date: &NaiveDate) -> String {
    let reservoirs: HashMap<String, Reservoir> = Reservoir::get_reservoir_vector()
        .iter()
        .map(|res| {
            let station = res.station_id.clone();
            let res_copy = res.clone();
            (station, res_copy)
        })
        .collect();
    info!("{} Reservoirs Loaded", reservoirs.len());
    let mut all_reservoir_observations = get_surveys_of_reservoirs(start_date, end_date).await;
    info!("Surveyed Reseroirs: {}", all_reservoir_observations.len());
    info!("Observations Downloaded");
    all_reservoir_observations.interpolate_reservoir_observations();
    info!(
        "Interpolated Reseroirs: {}",
        all_reservoir_observations.len()
    );
    info!("Observations Interpolated and Sorted");
    let mut california_water_level_observations: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for observable_range in all_reservoir_observations {
        for survey in observable_range.observations {
            let tap = survey.get_tap();
            let date_observation = tap.date_observation;
            let station_id = tap.station_id.clone();
            let recording = survey.get_value();
            let reservoir = reservoirs.get(&station_id).unwrap();
            let reservoir_capacity: f64 = reservoir.capacity.cast();
            let observed_value = recording.min(reservoir_capacity);
            california_water_level_observations
                .entry(date_observation)
                .and_modify(|e| *e += observed_value)
                .or_insert(observed_value);
        }
    }
    info!("Observations Accumulated");
    let mut writer = Writer::from_writer(vec![]);
    for (date, observation) in california_water_level_observations {
        let date_string = date.format("%Y%m%d").to_string();
        let date_str = date_string.as_str();
        let observation_string = observation.to_string();
        let observation_str = observation_string.as_str();
        let string_record = StringRecord::from(vec![date_str, observation_str]);
        if writer
            .write_byte_record(string_record.as_byte_record())
            .is_err()
        {
            panic!("Error: writing record failed");
        }
    }
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
}

pub async fn run_csv(start_date: &NaiveDate, end_date: &NaiveDate) -> String {
    info!("run_csv");
    let mut all_reservoir_observations = get_surveys_of_reservoirs(start_date, end_date).await;
    info!("ran all surveys!");
    let option_of_compressed_string_records = all_reservoir_observations
        .iter_mut()
        .map(|surveys| {
            surveys.observations.sort();
            let earliest_date = {
                if let Some(survey_first) = surveys.observations.first() {
                    let tap = survey_first.get_tap();
                    tap.date_observation
                } else {
                    return None;
                }
            };
            let last_survey = surveys.observations.last().unwrap();
            let last_tap = last_survey.get_tap();
            let most_recent_date = last_tap.date_observation;
            let month_datum: HashSet<MonthDatum> = HashSet::new();
            let mut observable_range = ObservableRange {
                observations: surveys.observations.clone(),
                start_date: earliest_date,
                end_date: most_recent_date,
                month_datum,
            };
            observable_range.retain();
            let records: Vec<CompressedStringRecord> = observable_range
                .observations
                .into_iter()
                .map(|survey| {
                    let record: CompressedStringRecord = survey.into();
                    record
                })
                .collect::<Vec<CompressedStringRecord>>();
            Some(records)
        })
        .collect::<Vec<_>>();
    //compressedstringrecords from hear on out
    let mut writer = Writer::from_writer(vec![]);
    let flattened_records = option_of_compressed_string_records.into_iter().flatten();
    for reservoir_records in flattened_records {
        for reservoir_record in reservoir_records {
            if writer
                .write_byte_record(reservoir_record.0.as_byte_record())
                .is_err()
            {
                panic!("Error: writing record failed");
            }
        }
    }
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
}
