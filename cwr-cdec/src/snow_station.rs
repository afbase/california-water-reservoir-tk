use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

/// Represents a CDEC snow measurement station.
///
/// Parallel to `Reservoir`, this struct holds metadata for stations that
/// report snow water content (SWC) and snow depth measurements.
///
/// See: <https://cdec.water.ca.gov/reportapp/javareports?name=SnowSensors>
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SnowStation {
    /// CDEC station identifier (e.g., "GRZ" for Grizzly Peak)
    pub station_id: String,
    /// Human-readable name of the station
    pub name: String,
    /// Elevation of the station in feet
    pub elevation: i32,
    /// River basin where the station is located
    pub river_basin: String,
    /// County where the station is located
    pub county: String,
    /// Latitude in decimal degrees
    pub latitude: f64,
    /// Longitude in decimal degrees
    pub longitude: f64,
}

impl SnowStation {
    /// Parse a CSV string of snow station data into a vector of SnowStations.
    ///
    /// Expected CSV columns: station_id, name, elevation, river_basin, county, latitude, longitude
    pub fn parse_snow_station_csv(csv_object: &str) -> Result<Vec<SnowStation>, std::io::Error> {
        let mut station_list: Vec<SnowStation> = Vec::new();
        let mut rdr = ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(true)
            .from_reader(csv_object.as_bytes());
        for row in rdr.records() {
            let record = row?;
            let station_id = String::from(
                record
                    .get(0)
                    .expect("station_id parse fail"),
            );
            let name = String::from(
                record
                    .get(1)
                    .expect("name parse fail"),
            );
            let elevation = record
                .get(2)
                .unwrap_or("0")
                .trim()
                .parse::<i32>()
                .unwrap_or(0);
            let river_basin = String::from(
                record
                    .get(3)
                    .expect("river_basin parse fail"),
            );
            let county = String::from(
                record
                    .get(4)
                    .expect("county parse fail"),
            );
            let latitude = record
                .get(5)
                .unwrap_or("0.0")
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0);
            let longitude = record
                .get(6)
                .unwrap_or("0.0")
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0);
            let station = SnowStation {
                station_id,
                name,
                elevation,
                river_basin,
                county,
                latitude,
                longitude,
            };
            station_list.push(station);
        }
        Ok(station_list)
    }
}

#[cfg(test)]
mod tests {
    use super::SnowStation;

    #[test]
    fn test_parse_snow_station_csv() {
        let csv_data = "\
station_id,name,elevation,river_basin,county,latitude,longitude
GRZ,Grizzly Peak,6800,American,El Dorado,38.75,-120.25
SDW,Slide Mountain,7900,Truckee,Washoe,39.32,-119.87
";
        let stations = SnowStation::parse_snow_station_csv(csv_data).unwrap();
        assert_eq!(stations.len(), 2);
        assert_eq!(stations[0].station_id, "GRZ");
        assert_eq!(stations[0].name, "Grizzly Peak");
        assert_eq!(stations[0].elevation, 6800);
        assert_eq!(stations[0].river_basin, "American");
        assert_eq!(stations[0].county, "El Dorado");
        assert!((stations[0].latitude - 38.75).abs() < f64::EPSILON);
        assert!((stations[0].longitude - (-120.25)).abs() < f64::EPSILON);
        assert_eq!(stations[1].station_id, "SDW");
    }

    #[test]
    fn test_parse_empty_csv() {
        let csv_data = "station_id,name,elevation,river_basin,county,latitude,longitude\n";
        let stations = SnowStation::parse_snow_station_csv(csv_data).unwrap();
        assert_eq!(stations.len(), 0);
    }
}
