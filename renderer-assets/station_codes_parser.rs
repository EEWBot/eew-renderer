use std::collections::HashMap;

use renderer_types::*;

use itertools::Itertools;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct JsonEntry {
    #[serde(rename = "lat")]
    latitude: String,
    #[serde(rename = "lon")]
    longitude: Lon,
    name: String,
    pref: String,
    affi: String,
    area_code: String,
    city_code: String,
    station_code: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Lon {
    Number(f32),
    String(String),
}

#[derive(Debug)]
pub struct IntensityStationInternal {
    pub 地震情報細分区域: codes::地震情報細分区域,
    pub 震度観測点: codes::震度観測点,
    pub 地震情報都道府県等: codes::地震情報都道府県等,
    pub position: (f32, f32),
}

#[derive(Debug)]
pub struct IntensityStationRange {
    pub start_i: usize,
    pub n: usize,
}

pub fn read(
    s: &str,
) -> (
    Vec<(f32, f32)>,
    HashMap<codes::地震情報細分区域, IntensityStationRange>,
    HashMap<u32, usize>,
    HashMap<codes::地震情報細分区域, codes::地震情報都道府県等>,
) {
    let stations: Vec<JsonEntry> = serde_json::from_str(s).unwrap();

    let intensity_station_internal: Vec<IntensityStationInternal> = stations
        .into_iter()
        .map(|v| {
            let lat: f32 = v.latitude.parse().unwrap();

            let lon: f32 = match v.longitude {
                Lon::Number(v) => v,
                Lon::String(v) => v.parse().unwrap(),
            };

            IntensityStationInternal {
                地震情報細分区域: codes::地震情報細分区域(v.area_code.parse().unwrap()),
                震度観測点: codes::震度観測点(v.station_code.parse().unwrap()),
                地震情報都道府県等: codes::地震情報都道府県等(v.pref.parse().unwrap()),
                position: (lon, lat),
            }
        })
        .sorted_by_key(|v| v.地震情報細分区域)
        .collect();

    #[allow(non_snake_case)]
    let area_code__intensity_station_range: HashMap<_, _> = intensity_station_internal
        .iter()
        .map(|v| v.地震情報細分区域)
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
        .map(|v| (v.地震情報細分区域, v.地震情報都道府県等))
        .collect();

    #[allow(non_snake_case)]
    let station_code__index: HashMap<_, _> = intensity_station_internal
        .iter()
        .enumerate()
        .map(|(i, v)| (v.震度観測点.0, i))
        .collect();

    let intensity_station_positions: Vec<_> = intensity_station_internal
        .into_iter()
        .map(|v| v.position)
        .collect();

    (
        intensity_station_positions,
        area_code__intensity_station_range,
        station_code__index,
        area_code__pref_code,
    )
}
