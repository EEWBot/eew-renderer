#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::type_complexity)]

include!(concat!(env!("OUT_DIR"), "/data.rs"));

use renderer_types::*;

pub struct QueryInterface;

pub struct Geometries {
    pub vertices: &'static [(f32, f32)],
    pub map_triangles: &'static [u32],
    pub area_lines: &'static [&'static [u32]],
    pub pref_lines: &'static [&'static [u32]],
}

pub struct LakeGeometries {
    pub vertices: &'static [(f32, f32)],
    pub indices: &'static [u32],
}

pub struct TsunamiGeometries {
    pub line_map: &'static phf::Map<codes::TsunamiArea, &'static [&'static [(f32, f32)]]>,
}

impl QueryInterface {
    pub fn geometries() -> Geometries {
        Geometries {
            vertices: VERTICES,
            map_triangles: MAP_TRIANGLES,
            area_lines: AREA_LINES,
            pref_lines: PREF_LINES,
        }
    }

    pub fn lake_geometries() -> LakeGeometries {
        LakeGeometries {
            vertices: LAKE_VERTICES,
            indices: LAKE_INDICES,
        }
    }

    pub fn tsunami_geometries() -> TsunamiGeometries {
        TsunamiGeometries {
            line_map: &TSUNAMI_LINE_MAP,
        }
    }

    pub fn query_bounding_box_by_area(area_code: codes::Area) -> Option<BoundingBox<GeoDegree>> {
        Some(BoundingBox::from_tuple::<GeoDegree>(
            AREAS.get(&area_code)?.1,
        ))
    }

    pub fn query_rendering_center_by_area(area_code: codes::Area) -> Option<Vertex<GeoDegree>> {
        Some(INTENSITY_STATION_POSITIONS[AREAS.get(&area_code)?.0].into())
    }

    pub fn query_position_by_station_code(
        intensity_station_code: codes::IntensityStation,
    ) -> Option<Vertex<GeoDegree>> {
        Some(INTENSITY_STATION_POSITIONS[*STATION_CODES.get(&intensity_station_code)?].into())
    }

    pub fn query_lod_level_by_scale(scale: f32) -> Option<usize> {
        SCALE_LEVEL_MAP
            .iter()
            .find_map(|(s, l)| if *s <= scale { Some(*l) } else { None })
    }

    pub fn query_lod_level_count() -> usize {
        AREA_LINES.len()
    }
}
