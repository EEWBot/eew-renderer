use std::collections::HashMap;
use std::path::PathBuf;

use renderer_types::*;

use itertools::Itertools;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum StationLoadError {
    #[error("intensity_stations.json {path} を読めない")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("intensity_stations.jsonが不正")]
    Json(#[from] serde_json::Error),

    #[error("観測点[{index}]の{field}が不正: {value:?}")]
    Field {
        index: usize,
        field: &'static str,
        value: String,
    },

    #[error("観測点[{index}]の座標が有限でない")]
    NonFiniteCoordinate { index: usize },

    #[error("観測点[{index}]の座標が範囲外: (lat: {lat}, lon: {lon})")]
    CoordinateOutOfRange { index: usize, lat: f32, lon: f32 },

    #[error("stationCode {0} が重複している")]
    DuplicateStationCode(u32),

    #[error("地図上にあるarea {0} がintensity_stations.jsonに無い")]
    AreaWithoutStation(u32),

    #[error("intensity_stations.jsonは既に初期化されている")]
    AlreadyInitialized,

    #[error("intensity_stations.jsonがまだ初期化されていない")]
    NotInitialized,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct JsonEntry {
    #[serde(rename = "lat")]
    latitude: NumOrString,
    #[serde(rename = "lon")]
    longitude: NumOrString,
    name: String,
    pref: String,
    affi: String,
    area_code: String,
    city_code: String,
    station_code: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NumOrString {
    Number(f32),
    String(String),
}

impl NumOrString {
    fn to_f32(&self, index: usize, field: &'static str) -> Result<f32, StationLoadError> {
        match self {
            Self::Number(v) => Ok(*v),
            Self::String(v) => v.parse().map_err(|_| StationLoadError::Field {
                index,
                field,
                value: v.clone(),
            }),
        }
    }
}

fn parse_code(s: &str, index: usize, field: &'static str) -> Result<u32, StationLoadError> {
    s.parse().map_err(|_| StationLoadError::Field {
        index,
        field,
        value: s.to_owned(),
    })
}

#[derive(Debug)]
struct IntensityStationInternal {
    area_code: codes::地震情報細分区域,
    station_code: codes::震度観測点,
    pref_code: codes::地震情報都道府県等,
    position: (f32, f32),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct IntensityStationRange {
    pub start_i: usize,
    pub n: usize,
}

#[allow(dead_code)]
pub struct ParsedStations {
    pub positions: Vec<(f32, f32)>,
    pub area_ranges: HashMap<codes::地震情報細分区域, IntensityStationRange>,
    pub station_code_index: HashMap<u32, usize>,
    pub area_to_pref: HashMap<codes::地震情報細分区域, codes::地震情報都道府県等>,
}

pub fn parse(s: &str) -> Result<ParsedStations, StationLoadError> {
    let stations: Vec<JsonEntry> = serde_json::from_str(s)?;

    let intensity_station_internal: Vec<IntensityStationInternal> = stations
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let lat = v.latitude.to_f32(i, "lat")?;
            let lon = v.longitude.to_f32(i, "lon")?;

            if !lat.is_finite() || !lon.is_finite() {
                return Err(StationLoadError::NonFiniteCoordinate { index: i });
            }

            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                return Err(StationLoadError::CoordinateOutOfRange { index: i, lat, lon });
            }

            Ok(IntensityStationInternal {
                area_code: codes::地震情報細分区域(
                    parse_code(&v.area_code, i, "areaCode")?,
                ),
                station_code: codes::震度観測点(
                    parse_code(&v.station_code, i, "stationCode")?,
                ),
                pref_code: codes::地震情報都道府県等(parse_code(&v.pref, i, "pref")?),
                position: (lon, lat),
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sorted_by_key(|v| v.area_code)
        .collect();

    #[allow(non_snake_case)]
    let area_code__intensity_station_range: HashMap<_, _> = intensity_station_internal
        .iter()
        .map(|v| v.area_code)
        .dedup_with_count()
        .sorted_by_key(|(_len, area_code)| *area_code)
        .scan(0, |offset, (len, area_code)| {
            let internal = IntensityStationRange {
                start_i: *offset,
                n: len,
            };

            *offset += len;

            Some((area_code, internal))
        })
        .collect();

    #[allow(non_snake_case)]
    let area_code__pref_code: HashMap<_, _> = intensity_station_internal
        .iter()
        .map(|v| (v.area_code, v.pref_code))
        .collect();

    let mut station_code_index: HashMap<u32, usize> = HashMap::new();
    for (i, v) in intensity_station_internal.iter().enumerate() {
        if station_code_index.insert(v.station_code.0, i).is_some() {
            return Err(StationLoadError::DuplicateStationCode(v.station_code.0));
        }
    }

    let intensity_station_positions: Vec<_> = intensity_station_internal
        .into_iter()
        .map(|v| v.position)
        .collect();

    Ok(ParsedStations {
        positions: intensity_station_positions,
        area_ranges: area_code__intensity_station_range,
        station_code_index,
        area_to_pref: area_code__pref_code,
    })
}
