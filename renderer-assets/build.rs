#![allow(clippy::type_complexity)]

use std::collections::HashMap;

use const_gen::*;

#[path = "src/stations.rs"]
mod stations;
use asset_preprocessor::{
    parse_lake_shapefile, parse_shapefile, parse_tsunami_shapefile, parse_world_shapefile,
};
use renderer_types::{BoundingBox, GeoDegree};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=src/stations.rs");
    println!("cargo::rerun-if-changed=../assets/intensity_stations.json");
    println!("cargo::rerun-if-changed=../assets/shapefile");

    let (tsunami_vertices, tsunami_indices, tsunami_area_code_to_internal_code) =
        parse_tsunami_shapefile::read();

    let (lake_vertices, lake_indices) = parse_lake_shapefile::read();

    let (world_vertices, world_indices) = parse_world_shapefile::read();

    let raw = std::fs::read("../assets/intensity_stations.json").unwrap();

    let minified: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    std::fs::write(
        format!(
            "{}/intensity_stations.min.json",
            std::env::var("OUT_DIR").unwrap()
        ),
        serde_json::to_string(&minified).unwrap(),
    )
    .unwrap();

    let parsed = stations::parse(&raw).expect("埋め込みのintensity_stations.jsonが不正");

    #[allow(non_snake_case)]
    let (
        area_code__bbox,
        area_code__centers,
        vertices,
        indices,
        area_lines,
        pref_lines,
        scale_level_map,
    ) = parse_shapefile::read(&parsed.area_to_pref);

    // 壊れたintensity_stations.jsonをでbuildを落とす用
    let area_bboxes: HashMap<u32, (f32, f32, f32, f32)> = area_code__bbox
        .iter()
        .map(|(code, bbox)| {
            let range = parsed
                .area_ranges
                .get(code)
                .expect("地図上にあるareaだがintensity_stations.json上に無い");

            assert!(range.n >= 1, "エリア内に一つも観測点がない");

            (code.0, bbox_to_tuple(bbox))
        })
        .collect();

    let area_centers: HashMap<u32, (f32, f32)> = area_code__centers
        .iter()
        .filter(|(code, _center)| area_bboxes.contains_key(&code.0))
        .map(|(code, center)| (code.0, (center.x(), center.y())))
        .collect();

    assert_eq!(
        area_bboxes.len(),
        area_centers.len(),
        "AREA_BBOXES と AREA_CENTERS のキー集合が一致しない"
    );

    let const_declarations = [
        const_declaration!(AREA_BBOXES = area_bboxes),
        const_declaration!(AREA_CENTERS = area_centers),
        const_declaration!(VERTICES = vertices),
        const_declaration!(MAP_TRIANGLES = indices),
        const_declaration!(AREA_LINES = area_lines),
        const_declaration!(PREF_LINES = pref_lines),
        const_declaration!(SCALE_LEVEL_MAP = scale_level_map),
        const_declaration!(LAKE_VERTICES = lake_vertices),
        const_declaration!(LAKE_INDICES = lake_indices),
        const_declaration!(WORLD_VERTICES = world_vertices),
        const_declaration!(WORLD_INDICES = world_indices),
        const_declaration!(TSUNAMI_VERTICES = tsunami_vertices),
        const_declaration!(TSUNAMI_INDICES = tsunami_indices),
        const_declaration!(TSUNAMI_AREA_CODE_TO_INTERNAL_CODE = tsunami_area_code_to_internal_code),
    ]
    .join("\n");

    std::fs::write(
        format!("{}/data.rs", std::env::var("OUT_DIR").unwrap()),
        const_declarations,
    )
    .unwrap();
}

fn bbox_to_tuple(bb: &BoundingBox<GeoDegree>) -> (f32, f32, f32, f32) {
    (bb.min.x(), bb.min.y(), bb.max.x(), bb.max.y())
}
