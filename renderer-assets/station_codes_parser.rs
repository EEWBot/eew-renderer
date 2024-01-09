use std::collections::HashMap;

use renderer_types::*;

use serde::Deserialize;
use itertools::Itertools;

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
    pub area_code: codes::Area,
    pub station_code: codes::IntensityStation,
    pub pref_code: codes::Pref,
    pub position: (f32, f32),
}

#[derive(Debug)]
pub struct IntensityStationRange {
    pub start_i: usize,
    pub n: usize,
}

pub fn read(s: &str) -> (
    Vec<(f32, f32)>,
    HashMap<codes::Area, IntensityStationRange>,
    HashMap<codes::IntensityStation, usize>,
    HashMap<codes::Area, codes::Pref>
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
                area_code: v.area_code.parse().unwrap(),
                station_code: v.station_code.parse().unwrap(),
                pref_code: v.pref.parse().unwrap(),
                position: (lon, lat),
            }
        })
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

    #[allow(non_snake_case)]
    let station_code__index: HashMap<_, _> = intensity_station_internal
        .iter()
        .enumerate()
        .map(|(i, v)| (v.station_code, i))
        .collect();

    let intensity_station_positions: Vec<_> = intensity_station_internal
        .into_iter()
        .map(|v| v.position).collect();

    (
        intensity_station_positions,
        area_code__intensity_station_range,
        station_code__index,
        area_code__pref_code,
    )
}
