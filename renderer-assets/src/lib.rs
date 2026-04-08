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
    pub vertices: &'static [(f32, f32, u16)],
    pub indices: &'static [u32],
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
            vertices: TSUNAMI_VERTICES,
            indices: TSUNAMI_INDICES,
        }
    }

    pub fn is_valid_地震情報細分区域(code: codes::地震情報細分区域) -> bool {
        AREAS.contains_key(&code.0)
    }

    pub fn is_valid_津波予報区(code: codes::津波予報区) -> bool {
        TSUNAMI_AREA_CODE_TO_INTERNAL_CODE.contains_key(&code.0)
    }

    pub fn 津波予報区_to_internal_tsunami_area_code(
        code: codes::津波予報区,
    ) -> Option<codes::InternalTsunamiAreaCode> {
        TSUNAMI_AREA_CODE_TO_INTERNAL_CODE
            .get(&code.0)
            .map(|code| codes::InternalTsunamiAreaCode(*code))
    }

    pub fn internal_tsunami_area_code_count() -> usize {
        TSUNAMI_AREA_CODE_TO_INTERNAL_CODE.len()
    }

    pub fn query_bounding_box_by_地震情報細分区域(code: codes::地震情報細分区域) -> Option<BoundingBox<GeoDegree>> {
        let tuple = AREAS.get(&code.0)?.1;
        let min = Vertex::new(tuple.0, tuple.1);
        let max = Vertex::new(tuple.2, tuple.3);
        Some(BoundingBox::new(min, max))
    }

    pub fn query_intensity_icon_center_by_地震情報細分区域(code: codes::地震情報細分区域) -> Option<Vertex<GeoDegree>> {
        Some(INTENSITY_STATION_POSITIONS[AREAS.get(&code.0)?.0].into())
    }

    pub fn query_position_by_station_code(
        code: codes::震度観測点,
    ) -> Option<Vertex<GeoDegree>> {
        Some(INTENSITY_STATION_POSITIONS[*STATION_CODES.get(&code.0)?].into())
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
