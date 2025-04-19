use crate::intensity::震度;
use enum_map::EnumMap;
use renderer_types::codes::Area;
use renderer_types::{GeoDegree, Vertex};

pub struct RenderingContextV0 {
    pub epicenter: Option<Vertex<GeoDegree>>,
    pub area_intensities: EnumMap<震度, Vec<Area>>,
}
