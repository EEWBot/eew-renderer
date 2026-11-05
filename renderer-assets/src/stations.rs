use std::collections::HashMap;

use renderer_types::*;

use chrono::{DateTime, FixedOffset};
use itertools::Itertools;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum StationLoadError {
    #[error("intensity_stations.jsonが不正")]
    Json(#[from] serde_json::Error),

    #[error("観測点[{index}]の{field}が不正: {value:?}")]
    Field {
        index: usize,
        field: &'static str,
        value: String,
    },

    #[error("観測点[{index}]の座標が有限でない")]
    NonFiniteCoordinate { index: usize },

    #[error("観測点[{index}]の座標が範囲外: (lat: {lat}, lon: {lon})")]
    CoordinateOutOfRange { index: usize, lat: f32, lon: f32 },

    #[error("観測点code {0} が重複している")]
    DuplicateStationCode(u32),

    #[error("未対応の観測点マスター: schema_version={version}, source_kind={source_kind}")]
    UnsupportedMaster { version: u32, source_kind: String },

    #[error("観測点マスターのリリース情報が不正")]
    InvalidRelease,

    #[error("{field}の日時が不正: {value:?}")]
    InvalidTimestamp { field: &'static str, value: String },

    #[error("観測点[{index}]に有効なmetadataが無い")]
    MissingMetadata { index: usize },

    #[error("区域{area_code}に異なる都道府県コードが含まれる")]
    InconsistentPrefecture { area_code: u32 },

    #[error("地図上にあるarea {0} がintensity_stations.jsonに無い")]
    AreaWithoutStation(u32),

    #[error("intensity_stations.jsonは既に初期化されている")]
    AlreadyInitialized,

    #[error("intensity_stations.jsonがまだ初期化されていない")]
    NotInitialized,
}

#[derive(Deserialize)]
struct StationMaster {
    schema_version: u32,
    source_kind: String,
    releases: Vec<MasterRelease>,
    stations: Vec<MasterStation>,
}

#[derive(Deserialize)]
struct MasterRelease {
    effective_from: String,
    index_count: usize,
}

#[derive(Deserialize)]
struct MasterStation {
    code: String,
    lifecycle: Vec<LifecycleEvent>,
    scope_events: Vec<ScopeEvent>,
    metadata: Vec<StationMetadata>,
}

#[derive(Deserialize)]
struct LifecycleEvent {
    effective_from: String,
    active: bool,
}

#[derive(Deserialize)]
struct ScopeEvent {
    effective_from: String,
    scope: String,
    enabled: bool,
}

#[derive(Deserialize)]
struct StationMetadata {
    effective_from: String,
    region: MasterCode,
    city: MasterCode,
    location: Option<MasterLocation>,
}

#[derive(Deserialize)]
struct MasterCode {
    code: String,
}

#[derive(Deserialize)]
struct MasterLocation {
    latitude: f32,
    longitude: f32,
}

fn parse_code(s: &str, index: usize, field: &'static str) -> Result<u32, StationLoadError> {
    s.parse().map_err(|_| StationLoadError::Field {
        index,
        field,
        value: s.to_owned(),
    })
}

#[derive(Debug)]
struct IntensityStationInternal {
    area_code: codes::地震情報細分区域,
    station_code: codes::震度観測点,
    pref_code: codes::地震情報都道府県等,
    position: (f32, f32),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct IntensityStationRange {
    pub start_i: usize,
    pub n: usize,
}

#[allow(dead_code)]
pub struct ParsedStations {
    pub positions: Vec<(f32, f32)>,
    pub area_ranges: HashMap<codes::地震情報細分区域, IntensityStationRange>,
    pub station_code_index: HashMap<u32, usize>,
    pub area_to_pref: HashMap<codes::地震情報細分区域, codes::地震情報都道府県等>,
}

fn validate_coordinates(index: usize, lat: f32, lon: f32) -> Result<(), StationLoadError> {
    if !lat.is_finite() || !lon.is_finite() {
        return Err(StationLoadError::NonFiniteCoordinate { index });
    }
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(StationLoadError::CoordinateOutOfRange { index, lat, lon });
    }
    Ok(())
}

fn parse_timestamp(
    value: &str,
    field: &'static str,
) -> Result<DateTime<FixedOffset>, StationLoadError> {
    DateTime::parse_from_rfc3339(value).map_err(|_| StationLoadError::InvalidTimestamp {
        field,
        value: value.to_owned(),
    })
}

fn latest_event<'a, T>(
    events: &'a [T],
    release_time: DateTime<FixedOffset>,
    field: &'static str,
    timestamp: impl Fn(&T) -> &str,
) -> Result<Option<&'a T>, StationLoadError> {
    let mut latest = None;
    for event in events {
        let time = parse_timestamp(timestamp(event), field)?;
        if time <= release_time
            && latest
                .as_ref()
                .is_none_or(|(previous, _)| time >= *previous)
        {
            latest = Some((time, event));
        }
    }
    Ok(latest.map(|(_, event)| event))
}

fn parse_master(master: StationMaster) -> Result<Vec<IntensityStationInternal>, StationLoadError> {
    if master.schema_version != 1 || master.source_kind != "jma_public" {
        return Err(StationLoadError::UnsupportedMaster {
            version: master.schema_version,
            source_kind: master.source_kind,
        });
    }
    let release = master
        .releases
        .last()
        .ok_or(StationLoadError::InvalidRelease)?;
    if release.index_count == 0 || release.index_count > master.stations.len() {
        return Err(StationLoadError::InvalidRelease);
    }
    let release_time = parse_timestamp(&release.effective_from, "releases.effective_from")?;
    let mut stations = Vec::with_capacity(release.index_count);

    for (i, station) in master
        .stations
        .into_iter()
        .take(release.index_count)
        .enumerate()
    {
        let active = latest_event(
            &station.lifecycle,
            release_time,
            "lifecycle.effective_from",
            |e| &e.effective_from,
        )?
        .is_some_and(|event| event.active);
        let point_events: Vec<_> = station
            .scope_events
            .iter()
            .filter(|event| event.scope == "point_seismic_intensity")
            .collect();
        let enabled = latest_event(
            &point_events,
            release_time,
            "scope_events.effective_from",
            |e| &e.effective_from,
        )?
        .is_some_and(|event| event.enabled);
        if !active || !enabled {
            continue;
        }

        let metadata = latest_event(
            &station.metadata,
            release_time,
            "metadata.effective_from",
            |e| &e.effective_from,
        )?
        .ok_or(StationLoadError::MissingMetadata { index: i })?;

        let Some(location) = &metadata.location else {
            continue;
        };
        let lat = location.latitude;
        let lon = location.longitude;
        validate_coordinates(i, lat, lon)?;

        let city_code = &metadata.city.code;
        if city_code.len() != 7 || !city_code.bytes().all(|b| b.is_ascii_digit()) {
            return Err(StationLoadError::Field {
                index: i,
                field: "metadata.city.code",
                value: city_code.clone(),
            });
        }
        let pref_code = parse_code(&city_code[..2], i, "metadata.city.code")?;
        stations.push(IntensityStationInternal {
            area_code: codes::地震情報細分区域(parse_code(
                &metadata.region.code,
                i,
                "metadata.region.code",
            )?),
            station_code: codes::震度観測点(parse_code(&station.code, i, "code")?),
            pref_code: codes::地震情報都道府県等(pref_code),
            position: (lon, lat),
        });
    }
    Ok(stations)
}

pub fn parse(data: &[u8]) -> Result<ParsedStations, StationLoadError> {
    let master: StationMaster = serde_json::from_slice(data)?;
    let stations = parse_master(master)?;

    let intensity_station_internal: Vec<IntensityStationInternal> = stations
        .into_iter()
        .sorted_by_key(|v| v.area_code)
        .collect();

    #[allow(non_snake_case)]
    let area_code__intensity_station_range: HashMap<_, _> = intensity_station_internal
        .iter()
        .map(|v| v.area_code)
        .dedup_with_count()
        .sorted_by_key(|(_len, area_code)| *area_code)
        .scan(0, |offset, (len, area_code)| {
            let internal = IntensityStationRange {
                start_i: *offset,
                n: len,
            };

            *offset += len;

            Some((area_code, internal))
        })
        .collect();

    #[allow(non_snake_case)]
    let mut area_code__pref_code = HashMap::new();
    for station in &intensity_station_internal {
        if let Some(previous) = area_code__pref_code.insert(station.area_code, station.pref_code) {
            if previous != station.pref_code {
                return Err(StationLoadError::InconsistentPrefecture {
                    area_code: station.area_code.0,
                });
            }
        }
    }

    let mut station_code_index: HashMap<u32, usize> = HashMap::new();
    for (i, v) in intensity_station_internal.iter().enumerate() {
        if station_code_index.insert(v.station_code.0, i).is_some() {
            return Err(StationLoadError::DuplicateStationCode(v.station_code.0));
        }
    }

    let intensity_station_positions: Vec<_> = intensity_station_internal
        .into_iter()
        .map(|v| v.position)
        .collect();

    Ok(ParsedStations {
        positions: intensity_station_positions,
        area_ranges: area_code__intensity_station_range,
        station_code_index,
        area_to_pref: area_code__pref_code,
    })
}
