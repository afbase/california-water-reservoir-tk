use std::process;

use chrono::format::ParseError;
pub enum TryFromError {
    PeruseError,
    QueryError,
    SurveyError,
    NoneError,
}

pub fn date_error(date_type: String, err: ParseError) {
    let err_kind = err.kind();
    eprintln!("{date_type} Date Error: {err_kind:?}");
    eprintln!("Date must be of YYYY-MM-DD format");
    process::exit(1);
}
