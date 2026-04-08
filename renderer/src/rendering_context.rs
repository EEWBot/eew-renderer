use crate::model::{津波情報, 震度};
use crate::proto;
use chrono::{DateTime, Utc};
use enum_map::enum_map;
use enum_map::EnumMap;
use renderer_assets::QueryInterface;
use renderer_types::codes;
use renderer_types::{GeoDegree, Vertex};

#[derive(Debug)]
pub struct EarthquakePayload {
    pub time: DateTime<Utc>,
    pub epicenter: Vec<Vertex<GeoDegree>>,
    pub area_intensities: EnumMap<震度, Vec<codes::地震情報細分区域>>,
}

impl EarthquakePayload {
    pub fn into_frame_payload(self) -> crate::frame_context::FramePayload {
        crate::frame_context::FramePayload::Earthquake(crate::frame_context::EarthquakePayload {
            time: self.time,
            epicenter: self.epicenter,
            area_intensities: self.area_intensities,
        })
    }
}

#[derive(Debug)]
pub struct TsunamiPayload {
    pub time: DateTime<Utc>,
    pub epicenter: Vec<Vertex<GeoDegree>>,
    pub forecast_levels: EnumMap<津波情報, Vec<codes::津波予報区>>,
}

impl TsunamiPayload {
    pub fn into_frame_payloads(self) -> [crate::frame_context::FramePayload; 2] {
        [
            crate::frame_context::FramePayload::TsunamiFirst(
                crate::frame_context::TsunamiFirstPayload {
                    time: self.time,
                    forecast_levels: self.forecast_levels.clone(),
                },
            ),
            crate::frame_context::FramePayload::TsunamiSecond(
                crate::frame_context::TsunamiSecondPayload {
                    time: self.time,
                    forecast_levels: self.forecast_levels,
                    epicenter: self.epicenter,
                },
            ),
        ]
    }
}

#[derive(Debug)]
pub enum RenderingPayload {
    Earthquake(EarthquakePayload),
    Tsunami(TsunamiPayload),
}

#[derive(Debug)]
pub struct RenderingContext {
    pub payload: RenderingPayload,
    pub request_identity: String,
}

#[derive(thiserror::Error, Debug)]
pub enum PayloadError {
    #[error("Invalid AreaCode is provided")]
    InvalidAreaCodeIsProvided,

    #[error("AreaCode or epicenter were not provided")]
    AreaCodeOrEpicenterWereNotProvided,
}

impl TryFrom<proto::QuakePrefectureV0> for RenderingPayload {
    type Error = PayloadError;

    fn try_from(data: proto::QuakePrefectureV0) -> Result<Self, Self::Error> {
        let area_intensities = enum_map! {
            震度::震度1 => data.one.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
            震度::震度2 => data.two.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
            震度::震度3 => data.three.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
            震度::震度4 => data.four.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
            震度::震度5弱 => data.five_minus.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
            震度::震度5強 => data.five_plus.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
            震度::震度6弱 => data.six_minus.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
            震度::震度6強 => data.six_plus.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
            震度::震度7 => data.seven.clone().map(|v| v.codes.iter().map(|code| codes::地震情報細分区域(*code)).collect()).unwrap_or(vec![]),
        };

        if !area_intensities.iter().all(|(_, areas)| {
            areas
                .iter()
                .all(|area| QueryInterface::is_valid_地震情報細分区域(*area))
        }) {
            return Err(PayloadError::InvalidAreaCodeIsProvided);
        }

        if data.epicenter.is_none() && area_intensities.iter().all(|(_, areas)| areas.is_empty()) {
            return Err(PayloadError::AreaCodeOrEpicenterWereNotProvided);
        }

        Ok(Self::Earthquake(EarthquakePayload {
            time: DateTime::from_timestamp(data.time as i64, 0).unwrap(),
            epicenter: data
                .epicenter
                .into_iter()
                .map(|crate::proto::Epicenter { lat_x10, lon_x10 }| {
                    renderer_types::Vertex::new(lon_x10 as f32 / 10.0, lat_x10 as f32 / 10.0)
                })
                .collect(),
            area_intensities,
        }))
    }
}

impl TryFrom<proto::TsunamiForecastV0> for RenderingPayload {
    type Error = PayloadError;

    fn try_from(data: proto::TsunamiForecastV0) -> Result<Self, Self::Error> {
        let forecast_levels = enum_map! {
            津波情報::津波予報 => data.forecast.clone().map(|v| v.codes.iter().map(|code| codes::津波予報区(*code)).collect()).unwrap_or(vec![]),
            津波情報::津波注意報 => data.advisory.clone().map(|v| v.codes.iter().map(|code| codes::津波予報区(*code)).collect()).unwrap_or(vec![]),
            津波情報::津波警報 => data.warning.clone().map(|v| v.codes.iter().map(|code| codes::津波予報区(*code)).collect()).unwrap_or(vec![]),
            津波情報::大津波警報 => data.major_warning.clone().map(|v| v.codes.iter().map(|code| codes::津波予報区(*code)).collect()).unwrap_or(vec![]),
        };

        if !forecast_levels.iter().all(|(_, areas)| {
            areas
                .iter()
                .all(|area| QueryInterface::is_valid_津波予報区(*area))
        }) {
            return Err(PayloadError::InvalidAreaCodeIsProvided);
        }

        Ok(Self::Tsunami(TsunamiPayload {
            time: DateTime::from_timestamp(data.time as i64, 0).unwrap(),
            epicenter: data
                .epicenter
                .into_iter()
                .map(|crate::proto::Epicenter { lat_x10, lon_x10 }| {
                    renderer_types::Vertex::new(lon_x10 as f32 / 10.0, lat_x10 as f32 / 10.0)
                })
                .collect(),
            forecast_levels,
        }))
    }
}

impl TryFrom<proto::TsunamiForecastV1> for RenderingPayload {
    type Error = PayloadError;

    fn try_from(data: proto::TsunamiForecastV1) -> Result<Self, Self::Error> {
        let forecast_levels = enum_map! {
            津波情報::津波予報 => data.forecast.clone().map(|v| v.codes.iter().map(|code| codes::津波予報区(*code)).collect()).unwrap_or(vec![]),
            津波情報::津波注意報 => data.advisory.clone().map(|v| v.codes.iter().map(|code| codes::津波予報区(*code)).collect()).unwrap_or(vec![]),
            津波情報::津波警報 => data.warning.clone().map(|v| v.codes.iter().map(|code| codes::津波予報区(*code)).collect()).unwrap_or(vec![]),
            津波情報::大津波警報 => data.major_warning.clone().map(|v| v.codes.iter().map(|code| codes::津波予報区(*code)).collect()).unwrap_or(vec![]),
        };

        if !forecast_levels.iter().all(|(_, areas)| {
            areas
                .iter()
                .all(|area| QueryInterface::is_valid_津波予報区(*area))
        }) {
            return Err(PayloadError::InvalidAreaCodeIsProvided);
        }

        Ok(Self::Tsunami(TsunamiPayload {
            time: DateTime::from_timestamp(data.time as i64, 0).unwrap(),
            epicenter: data
                .epicenter
                .into_iter()
                .map(|crate::proto::Epicenter { lat_x10, lon_x10 }| {
                    renderer_types::Vertex::new(lon_x10 as f32 / 10.0, lat_x10 as f32 / 10.0)
                })
                .collect(),
            forecast_levels,
        }))
    }
}
