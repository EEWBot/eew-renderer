use std::collections::HashMap;

use renderer_types::*;

use itertools::Itertools;
use serde::Deserialize;

#[derive(Deserialize)]
struct Stations {
    items: Vec<JsonEntry>,
}

#[derive(Deserialize)]
struct CodeOnly {
    code: String,
}

#[derive(Deserialize)]
struct JsonEntry {
    region: CodeOnly,
    city: CodeOnly,
    code: String,
    status: String,
    latitude: String,
    longitude: String,
}

#[derive(Debug)]
pub struct IntensityStationInternal {
    pub area_code: codes::地震情報細分区域,
    pub station_code: codes::震度観測点,
    pub pref_code: codes::地震情報都道府県等,
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
    let stations: Stations = serde_json::from_str(s).unwrap();

    let intensity_station_internal: Vec<IntensityStationInternal> = stations
        .items
        .into_iter()
        .filter(|v| v.status != "廃止")
        .map(|v| {
            let lat: f32 = v.latitude.parse().unwrap();
            let lon: f32 = v.longitude.parse().unwrap();
            let pref: u32 = v.city.code[0..2].parse().unwrap();

            IntensityStationInternal {
                area_code: codes::地震情報細分区域(v.region.code.parse().unwrap()),
                station_code: codes::震度観測点(v.code.parse().unwrap()),
                pref_code: codes::地震情報都道府県等(pref),
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
        .map(|(i, v)| (v.station_code.0, i))
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
