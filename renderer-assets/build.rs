#![allow(clippy::type_complexity)]

use std::collections::HashMap;

use const_gen::*;
use ordered_float::NotNan;

mod station_codes_parser;
use asset_preprocessor::{parse_lake_shapefile, parse_shapefile};

fn main() {
    let (lake_vertices, lake_indices) = parse_lake_shapefile::read();

    let s = std::fs::read_to_string("../assets/intensity_stations.json").unwrap();

    #[allow(non_snake_case)]
    let (
        intensity_station_minimized,
        area_code__intensity_station_range,
        station_code__index,
        area_code__pref_code,
    ) = station_codes_parser::read(&s);

    #[allow(non_snake_case)]
    let (area_code__bbox, area_code__centers, vertices, indices, area_lines, pref_lines, scale_level_map) =
        parse_shapefile::read(&area_code__pref_code);

    // <AreaCode, (StationIndex, (BBox))>
    let areas: HashMap<u32, (usize, (f32, f32, f32, f32))> = area_code__bbox
        .iter()
        .map(|(code, bbox)| {
            let area = area_code__intensity_station_range
                .get(code)
                .expect("地図上にあるareaだがintensity_stations.json上に無い");

            let stations = &intensity_station_minimized[area.start_i..area.start_i + area.n];

            let area_center = area_code__centers.get(code).unwrap();

            let nearest_intensity_station_index = stations
                .iter()
                .enumerate()
                .min_by_key(|(_i, &station)| {
                    NotNan::new(area_center.euclidean_distance(station))
                        .expect("なぁん…観測点距離が何故かNaN")
                })
                .map(|(offset, _station)| area.start_i + offset)
                .expect("エリア内に一つも観測点がない");

            (*code, (nearest_intensity_station_index, bbox.to_tuple()))
        })
        .collect();

    let const_declarations = [
        const_declaration!(INTENSITY_STATION_POSITIONS = intensity_station_minimized),
        const_declaration!(AREAS = areas),
        const_declaration!(STATION_CODES = station_code__index),
        const_declaration!(VERTICES = vertices),
        const_declaration!(MAP_TRIANGLES = indices),
        const_declaration!(AREA_LINES = area_lines),
        const_declaration!(PREF_LINES = pref_lines),
        const_declaration!(SCALE_LEVEL_MAP = scale_level_map),
        const_declaration!(LAKE_VERTICES = lake_vertices),
        const_declaration!(LAKE_INDICES = lake_indices),
    ]
    .join("\n");

    std::fs::write(
        format!("{}/data.rs", std::env::var("OUT_DIR").unwrap()),
        const_declarations,
    )
    .unwrap();
}
