use crate::model::{TimeKind, 津波情報, 震度};
use chrono::{DateTime, Utc};
use enum_map::EnumMap;
use renderer_types::codes;
use renderer_types::{GeoDegree, Vertex};


pub trait HasEpicenter {
    fn epicenter(&self) -> &[Vertex<GeoDegree>];
}

pub trait HasTime {
    fn time(&self) -> DateTime<Utc>;
    fn time_kind(&self) -> TimeKind;
}

pub trait HasTsunamiForecastLevels {
    fn forecast_levels(&self) -> &EnumMap<津波情報, Vec<codes::津波予報区>>;
}


#[derive(Debug)]
pub struct EarthquakePayload {
    pub time: DateTime<Utc>,
    pub epicenter: Vec<Vertex<GeoDegree>>,
    pub area_intensities: EnumMap<震度, Vec<codes::地震情報細分区域>>,
}

impl HasTime for EarthquakePayload {
    fn time(&self) -> DateTime<Utc> {
        self.time
    }

    fn time_kind(&self) -> TimeKind {
        TimeKind::発生
    }
}

impl HasEpicenter for EarthquakePayload {
    fn epicenter(&self) -> &[Vertex<GeoDegree>] {
        &self.epicenter
    }
}


#[derive(Debug)]
pub struct TsunamiFirstPayload {
    pub time: DateTime<Utc>,
    pub forecast_levels: EnumMap<津波情報, Vec<codes::津波予報区>>,
}

impl HasTime for TsunamiFirstPayload {
    fn time(&self) -> DateTime<Utc> {
        self.time
    }

    fn time_kind(&self) -> TimeKind {
        TimeKind::発表
    }
}

impl HasTsunamiForecastLevels for TsunamiFirstPayload {
    fn forecast_levels(&self) -> &EnumMap<津波情報, Vec<codes::津波予報区>> {
        &self.forecast_levels
    }
}


#[derive(Debug)]
pub struct TsunamiSecondPayload {
    pub time: DateTime<Utc>,
    pub epicenter: Vec<Vertex<GeoDegree>>,
    pub forecast_levels: EnumMap<津波情報, Vec<codes::津波予報区>>,
}

impl HasTime for TsunamiSecondPayload {
    fn time(&self) -> DateTime<Utc> {
        self.time
    }

    fn time_kind(&self) -> TimeKind {
        TimeKind::発表
    }
}

impl HasEpicenter for TsunamiSecondPayload {
    fn epicenter(&self) -> &[Vertex<GeoDegree>] {
        &self.epicenter
    }
}

impl HasTsunamiForecastLevels for TsunamiSecondPayload {
    fn forecast_levels(&self) -> &EnumMap<津波情報, Vec<codes::津波予報区>> {
        &self.forecast_levels
    }
}


#[derive(Debug)]
pub enum FramePayload {
    Earthquake(EarthquakePayload),
    TsunamiFirst(TsunamiFirstPayload),
    TsunamiSecond(TsunamiSecondPayload),
}


#[derive(Debug)]
pub struct FrameContext {
    pub payload: FramePayload,
    pub request_identity: String,
}
