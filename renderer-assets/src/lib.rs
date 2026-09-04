#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::type_complexity)]

include!(concat!(env!("OUT_DIR"), "/data.rs"));

mod stations;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use arc_swap::{ArcSwap, Guard};
use renderer_types::*;

use stations::ParsedStations;
pub use stations::StationLoadError;

const EMBEDDED_INTENSITY_STATIONS: &str =
    include_str!(concat!(env!("OUT_DIR"), "/intensity_stations.min.json"));

static INTENSITY_STATIONS: OnceLock<StationStore> = OnceLock::new();

struct IntensityStations {
    positions: Vec<(f32, f32)>,
    station_code_index: HashMap<u32, usize>,
    area_nearest_station: HashMap<u32, usize>,
}

fn resolve(parsed: ParsedStations) -> Result<IntensityStations, StationLoadError> {
    let mut area_nearest_station = HashMap::with_capacity(AREA_BBOXES.len());

    for (area_code, _bbox) in AREA_BBOXES.entries() {
        let center = AREA_CENTERS
            .get(area_code)
            .expect("AREA_BBOXES and AREA_CENTERS must have identical keys");
        let center = Vertex::<GeoDegree>::new(center.0, center.1);

        let range = parsed
            .area_ranges
            .get(&codes::地震情報細分区域(*area_code))
            .filter(|range| range.n >= 1)
            .ok_or(StationLoadError::AreaWithoutStation(*area_code))?;

        let nearest = parsed.positions[range.start_i..range.start_i + range.n]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let a = center.euclidean_distance(&Vertex::<GeoDegree>::new(a.0, a.1));
                let b = center.euclidean_distance(&Vertex::<GeoDegree>::new(b.0, b.1));
                a.partial_cmp(&b).expect("観測点距離が何故かNaN")
            })
            .map(|(offset, _)| range.start_i + offset)
            .ok_or(StationLoadError::AreaWithoutStation(*area_code))?;

        area_nearest_station.insert(*area_code, nearest);
    }

    Ok(IntensityStations {
        positions: parsed.positions,
        station_code_index: parsed.station_code_index,
        area_nearest_station,
    })
}

pub struct StationStore(ArcSwap<IntensityStations>);

impl StationStore {
    fn build(s: &str) -> Result<IntensityStations, StationLoadError> {
        resolve(stations::parse(s)?)
    }

    fn build_from_path(path: &Path) -> Result<IntensityStations, StationLoadError> {
        let s = std::fs::read_to_string(path).map_err(|source| StationLoadError::Io {
            path: path.to_owned(),
            source,
        })?;

        Self::build(&s)
    }

    pub fn load_embedded() -> Result<Self, StationLoadError> {
        Ok(Self(ArcSwap::from_pointee(Self::build(
            EMBEDDED_INTENSITY_STATIONS,
        )?)))
    }

    pub fn load_from_path(path: &Path) -> Result<Self, StationLoadError> {
        Ok(Self(ArcSwap::from_pointee(Self::build_from_path(path)?)))
    }

    pub fn reload_from_path(&self, path: &Path) -> Result<(), StationLoadError> {
        let new = Self::build_from_path(path)?;
        self.0.store(Arc::new(new));

        Ok(())
    }

    fn snapshot(&self) -> Guard<Arc<IntensityStations>> {
        self.0.load()
    }

    pub fn rendering_center_by_area(
        &self,
        area_code: codes::地震情報細分区域,
    ) -> Option<Vertex<GeoDegree>> {
        let stations = self.snapshot();
        Some(stations.positions[*stations.area_nearest_station.get(&area_code.0)?].into())
    }

    pub fn position_by_station_code(
        &self,
        intensity_station_code: codes::震度観測点,
    ) -> Option<Vertex<GeoDegree>> {
        let stations = self.snapshot();
        Some(
            stations.positions[*stations.station_code_index.get(&intensity_station_code.0)?].into(),
        )
    }
}

fn stations() -> &'static StationStore {
    INTENSITY_STATIONS
        .get_or_init(|| StationStore::load_embedded().expect("embedded data must be valid"))
}

pub struct QueryInterface;

pub struct Geometries {
    pub vertices: &'static [(f32, f32)],
    pub map_triangles: [&'static [u32]; InsetRegion::COUNT],
    pub area_lines: &'static [&'static [u32]],
    pub pref_lines: &'static [&'static [u32]],
}

pub struct LakeGeometries {
    pub vertices: &'static [(f32, f32)],
    pub indices: &'static [u32],
}

pub struct WorldGeometries {
    pub vertices: &'static [(f32, f32)],
    pub indices: &'static [u32],
}

pub struct TsunamiGeometries {
    pub vertices: &'static [(f32, f32, u16)],
    pub indices: [&'static [u32]; InsetRegion::COUNT],
}

impl QueryInterface {
    pub fn init_intensity_stations(path: Option<&Path>) -> Result<(), StationLoadError> {
        if INTENSITY_STATIONS.get().is_some() {
            return Err(StationLoadError::AlreadyInitialized);
        }

        let store = match path {
            None => StationStore::load_embedded()?,
            Some(path) => StationStore::load_from_path(path)?,
        };

        INTENSITY_STATIONS
            .set(store)
            .map_err(|_| StationLoadError::AlreadyInitialized)
    }

    pub fn reload_intensity_stations(path: &Path) -> Result<(), StationLoadError> {
        INTENSITY_STATIONS
            .get()
            .ok_or(StationLoadError::NotInitialized)?
            .reload_from_path(path)
    }

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

    pub fn world_geometries() -> WorldGeometries {
        WorldGeometries {
            vertices: WORLD_VERTICES,
            indices: WORLD_INDICES,
        }
    }

    pub fn tsunami_geometries() -> TsunamiGeometries {
        TsunamiGeometries {
            vertices: TSUNAMI_VERTICES,
            indices: TSUNAMI_INDICES,
        }
    }

    pub fn is_valid_earthquake_area_code(area_code: codes::地震情報細分区域) -> bool {
        AREA_BBOXES.contains_key(&area_code.0)
    }

    pub fn is_valid_tsunami_area_code(area_code: codes::津波予報区) -> bool {
        TSUNAMI_AREA_CODE_TO_INTERNAL_CODE.contains_key(&area_code.0)
    }

    pub fn tsunami_area_code_to_internal_code(area_code: codes::津波予報区) -> Option<u16> {
        TSUNAMI_AREA_CODE_TO_INTERNAL_CODE
            .get(&area_code.0)
            .copied()
    }

    pub fn tsunami_area_code_count() -> usize {
        TSUNAMI_AREA_CODE_TO_INTERNAL_CODE.len()
    }

    pub fn query_bounding_box_by_area(
        area_code: codes::地震情報細分区域,
    ) -> Option<BoundingBox<GeoDegree>> {
        let tuple = AREA_BBOXES.get(&area_code.0)?;
        let min = Vertex::new(tuple.0, tuple.1);
        let max = Vertex::new(tuple.2, tuple.3);
        Some(BoundingBox::new(min, max))
    }

    pub fn query_rendering_center_by_area(
        area_code: codes::地震情報細分区域,
    ) -> Option<Vertex<GeoDegree>> {
        stations().rendering_center_by_area(area_code)
    }

    pub fn query_position_by_station_code(
        intensity_station_code: codes::震度観測点,
    ) -> Option<Vertex<GeoDegree>> {
        stations().position_by_station_code(intensity_station_code)
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
