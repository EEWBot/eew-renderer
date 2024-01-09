include!("data.rs");

use renderer_types::*;

pub struct QueryInterface;

pub struct Geometries {
    pub vertices: &'static [(f32, f32)],
    pub map_triangles: &'static [u32],
    pub area_lines: &'static [u32],
    pub pref_lines: &'static [u32],
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

    pub fn query_bounding_box_by_area(
        area_code: codes::Area,
    ) -> Option<BoundingBox<GeoDegree>> {
        Some(BoundingBox::from_tuple::<GeoDegree>(AREAS.get(&area_code)?.1))
    }

    pub fn query_rendering_center_by_area(
        area_code: codes::Area,
    ) -> Option<Vertex<GeoDegree>> {
        Some(INTENSITY_STATION_POSITIONS[AREAS.get(&area_code)?.0].into())
    }

    pub fn query_position_by_station_code(
        intensity_station_code: codes::IntensityStation,
    ) -> Option<Vertex<GeoDegree>> {
        Some(INTENSITY_STATION_POSITIONS[*STATION_CODES.get(&intensity_station_code)?].into())
    }
}
