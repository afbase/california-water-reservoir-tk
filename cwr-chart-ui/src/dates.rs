//! Conversions between the two date spellings the apps juggle:
//! CDEC's compact `YYYYMMDD` (data keys) and ISO `YYYY-MM-DD` (HTML date inputs).

/// `YYYYMMDD` -> `YYYY-MM-DD`. Anything that is not 8 ASCII digits is returned unchanged.
pub fn compact_to_iso(date: &str) -> String {
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8])
    } else {
        date.to_string()
    }
}

/// `YYYY-MM-DD` -> `YYYYMMDD`.
pub fn iso_to_compact(date: &str) -> String {
    date.replace('-', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        assert_eq!(compact_to_iso("20230110"), "2023-01-10");
        assert_eq!(iso_to_compact("2023-01-10"), "20230110");
        assert_eq!(compact_to_iso(&iso_to_compact("1925-12-31")), "1925-12-31");
    }

    #[test]
    fn leaves_unexpected_input_alone() {
        assert_eq!(compact_to_iso("2023-01-10"), "2023-01-10");
        assert_eq!(compact_to_iso(""), "");
        assert_eq!(compact_to_iso("2023011é"), "2023011é");
    }
}
