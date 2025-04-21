use crate::model::震度;
use chrono::{DateTime, Utc};
use enum_map::EnumMap;
use renderer_types::codes::Area;
use renderer_types::{GeoDegree, Vertex};

#[derive(Debug)]
pub struct RenderingContextV0 {
    pub time: DateTime<Utc>,
    pub epicenter: Option<Vertex<GeoDegree>>,
    pub area_intensities: EnumMap<震度, Vec<Area>>,
}
