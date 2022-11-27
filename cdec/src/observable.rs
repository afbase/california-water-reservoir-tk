use crate::{
    observation::DataRecording,
    survey::{Interpolate, Survey, Tap},
};
use chrono::{Datelike, Duration, NaiveDate};

// to group survey and observable types
pub trait Observable: Clone {
    fn into_survey(self) -> Survey;
}

pub trait CompressedSurveyBuilder {
    fn new(start_date: NaiveDate, end_date: NaiveDate) -> Self;
    fn update(&mut self, item: impl Observable);
    fn retain(&mut self);
    fn finalize(&mut self);
    fn pad_end(&mut self);
}

pub struct ObservableRange {
    pub observations: Vec<Survey>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

impl CompressedSurveyBuilder for ObservableRange {
    fn new(start_date: NaiveDate, end_date: NaiveDate) -> Self {
        if end_date < start_date {
            panic!("CompressedSurveyBuilder<> Error: End Date precedes Start Date");
        }
        let capacity = ((end_date - start_date).num_days() + 1) as usize;
        let v: Vec<Survey> = Vec::with_capacity(capacity);
        ObservableRange {
            observations: v,
            start_date,
            end_date,
        }
    }

    fn update(&mut self, item: impl Observable) {
        let item_clone = item.clone();
        let observations_clone = self.observations.clone();
        let t = item.into_survey();
        match t {
            Survey::Daily(tap) => {
                let has_record = observations_clone.into_iter().any(|obs| {
                    let survey = obs.into_survey();
                    let survey_tap = survey.get_tap();
                    tap.date_observation == survey_tap.date_observation
                });
                if !has_record {
                    self.observations.push(item_clone.into_survey());
                }
            }
            Survey::Monthly(tap) => {
                // needs to see if there is a daily observation for that month,
                // if there is, then do not insert
                // otherwise insert
                let year = tap.date_observation.year();
                let month = tap.date_observation.month();
                let has_an_observation_for_the_month = observations_clone.into_iter().any(|obs| {
                    let obs_year;
                    let obs_month;
                    let survey = obs.into_survey();
                    match survey {
                        Survey::Daily(daily_tap) => {
                            obs_year = daily_tap.date_observation.year();
                            obs_month = daily_tap.date_observation.month();
                        }
                        Survey::Monthly(monthly_tap) => {
                            obs_year = monthly_tap.date_observation.year();
                            obs_month = monthly_tap.date_observation.month();
                        }
                    }
                    obs_year == year && obs_month == month
                });
                if !has_an_observation_for_the_month {
                    // if there are no observations for that month
                    // then insert the observation
                    self.observations.push(item_clone.into_survey());
                }
            }
        }
    }

    fn retain(&mut self) {
        // remove anything that isn't a recording
        self.observations.retain(|observable| {
            let survey = observable.clone().into_survey();
            let tap = survey.get_tap();
            matches!(tap.value, DataRecording::Recording(_))
        });
    }

    fn finalize(&mut self) {
        // remove anything that isn't a recording
        self.observations.retain(|observable| {
            let survey = observable.clone().into_survey();
            let tap = survey.get_tap();
            matches!(tap.value, DataRecording::Recording(_))
        });
        // this smooths and pads observations
        self.observations.sort();
        let observation_clone = self.observations.clone();
        let surveys_slice = observation_clone.as_slice();
        let windows = surveys_slice.windows(2);
        for survey_window in windows {
            let survey_0 = survey_window[0].clone();
            let survey_1 = survey_window[1].clone();
            let survey_tuple = (survey_0, survey_1);
            let interpolation = survey_tuple.interpolate_pair();
            if let Some(vec_survey) = interpolation {
                for item in vec_survey {
                    self.update(item);
                }
            }
        }
    }

    fn pad_end(&mut self) {
        let mut tmp_date;
        let mut tmp_survey;
        self.observations.sort();
        let observations_clone = self.observations.clone();
        let most_recent = observations_clone.last().unwrap().get_tap();
        let most_recent_date = most_recent.date_observation;
        if most_recent_date < self.end_date {
            let days = (self.end_date - most_recent_date).num_days();
            for num_of_days in 1..days {
                tmp_date = most_recent_date + Duration::days(num_of_days);
                tmp_survey = Survey::Daily(Tap {
                    station_id: most_recent.station_id.clone(),
                    date_observation: tmp_date,
                    date_recording: tmp_date,
                    value: most_recent.value,
                });
                self.update(tmp_survey);
            }
        }
    }
}

// impl Ord for dyn Observable {
//     fn cmp(&self, other: &Self) -> Ordering {
//         let self_survey = self.into_survey();
//         let other_survey = other.into_survey();
//         self_survey.cmp(&other_survey)
//     }
// }

// impl Eq for dyn Observable {}

// impl PartialEq for dyn Observable {
//     fn eq(&self, other: &Self) -> bool {
//         let self_survey = self.clone().into_survey();
//         let self_tap = self_survey.get_tap();
//         let other_survey = other.clone().into_survey();
//         let other_tap = other_survey.get_tap();
//         self_tap.date_observation == other_tap.date_observation && self_tap.station_id == other_tap.station_id
//     }
// }

// impl PartialOrd for dyn Observable {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }
