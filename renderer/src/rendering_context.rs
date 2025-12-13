use crate::model::{津波情報, 震度};
use chrono::{DateTime, Utc};
use enum_map::EnumMap;
use renderer_types::codes::{Area, TsunamiArea};
use renderer_types::{GeoDegree, Vertex};

#[derive(Debug)]
pub enum RenderingContext {
    V0(V0),
    Tsunami(Tsunami)
}

impl RenderingContext {
    pub fn request_identity(&self) -> &str {
        match self {
            RenderingContext::Tsunami(tsunami) => &tsunami.request_identity,
            RenderingContext::V0(v0) => &v0.request_identity,
        }
    }
}

#[derive(Debug)]
pub struct V0 {
    pub time: DateTime<Utc>,
    pub epicenter: Option<Vertex<GeoDegree>>,
    pub area_intensities: EnumMap<震度, Vec<Area>>,
    pub request_identity: String,
}

impl HasTime for V0 {
    fn time(&self) -> DateTime<Utc> {
        self.time
    }
}

impl HasEpicenter for V0 {
    fn epicenter(&self) -> Option<Vertex<GeoDegree>> {
        self.epicenter.clone()
    }
}

impl HasRequestIdentity for V0 {
    fn request_identity(&self) -> String {
        self.request_identity.clone()
    }
}

#[derive(Debug)]
pub struct Tsunami {
    pub time: DateTime<Utc>,
    pub epicenter: Option<Vertex<GeoDegree>>,
    pub forecast_levels: EnumMap<津波情報, Vec<TsunamiArea>>,
    pub request_identity: String,
}

impl HasTime for Tsunami {
    fn time(&self) -> DateTime<Utc> {
        self.time
    }
}

impl HasEpicenter for Tsunami {
    fn epicenter(&self) -> Option<Vertex<GeoDegree>> {
        self.epicenter.clone()
    }
}

impl HasRequestIdentity for Tsunami {
    fn request_identity(&self) -> String {
        self.request_identity.clone()
    }
}

pub trait HasTime {
    fn time(&self) -> DateTime<Utc>;
}

pub trait HasEpicenter {
    fn epicenter(&self) -> Option<Vertex<GeoDegree>>;
}

pub trait HasRequestIdentity {
    fn request_identity(&self) -> String;
}
