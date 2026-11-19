#![allow(clippy::type_complexity)]

use std::collections::HashMap;
use std::path::Path;

use shapefile::dbase::{FieldValue, Record};
use shapefile::{Shape, ShapeReader};

use crate::math::*;
use renderer_types::{classify_tsunami_area, codes, InsetRegion};

struct TsunamiAreaCodeBuffer {
    area_code_to_internal_code: HashMap<u32, u16>,
}

impl TsunamiAreaCodeBuffer {
    fn new() -> Self {
        Self {
            area_code_to_internal_code: Default::default(),
        }
    }

    fn insert(&mut self, area_code: u32) -> u16 {
        match self.area_code_to_internal_code.get(&area_code) {
            Some(index) => *index,
            None => {
                let index = self.area_code_to_internal_code.len();
                self.area_code_to_internal_code
                    .insert(area_code, index as u16);
                index as u16
            }
        }
    }

    fn into_buffer(self) -> HashMap<u32, u16> {
        self.area_code_to_internal_code
    }
}

struct AreaLines {
    lines: Vec<Line>,
    tsunami_area_code: codes::津波予報区,
}

impl AreaLines {
    fn try_new(shape: Shape, record: Record) -> Option<Self> {
        let Shape::Polyline(polyline) = shape else {
            return None;
        };

        let tsunami_area_code: codes::津波予報区 = match record.get("code").unwrap() {
            FieldValue::Character(Some(c)) => match c.parse() {
                Ok(c) => codes::津波予報区(c),
                Err(_) => panic!("Failed to parse code"),
            },
            FieldValue::Character(None) => panic!("UNNUMBERED_AREA DETECTED!"),
            _ => panic!("Failed to get code"),
        };

        // 帰属未定 (極小の島など)
        if tsunami_area_code.0 == 0 {
            return None;
        }

        let lines: Vec<_> = polyline
            .parts()
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect();

        Some(Self {
            lines,
            tsunami_area_code,
        })
    }
}

struct Shapefile {
    entries: Vec<AreaLines>,
}

impl Shapefile {
    fn new<P: AsRef<Path>>(shp_file: P, dbf_file: P) -> Self {
        let shp_file = std::fs::File::open(shp_file);
        let dbf_file = std::fs::File::open(dbf_file);

        let (Ok(shp_file), Ok(dbf_file)) = (shp_file, dbf_file) else {
            panic!(
                r#"EEWBot Renderer requirements is not satisfied.

Simplified shape files are not found.
 - assets/shapefile/tsunami_forecast/tsunami_forecast_simplified.shp
 - assets/shapefile/tsunami_forecast/tsunami_forecast_simplified.dbf

Please follow:
  https://github.com/EEWBot/eew-renderer/wiki#shapefile-%E5%85%A5%E6%89%8B%E5%85%88"#
            )
        };

        let shape_reader = ShapeReader::new(shp_file).unwrap();
        let dbf_reader = shapefile::dbase::Reader::new(dbf_file).unwrap();
        let mut reader = shapefile::reader::Reader::new(shape_reader, dbf_reader);

        let entries = reader
            .iter_shapes_and_records()
            .flatten()
            .flat_map(|shape_record| AreaLines::try_new(shape_record.0, shape_record.1))
            .collect();

        Self { entries }
    }
}

pub type TsunamiAreaRange = (u16, u32, u32);

pub fn read() -> (
    // 線分 (mercator) と予報区の内部コード (main / okinawa / ogasawara)
    [Vec<(f32, f32, f32, f32, u16)>; InsetRegion::COUNT],
    [Vec<TsunamiAreaRange>; InsetRegion::COUNT],
    HashMap<u32, u16>, // tsunami_area_code_to_internal_code
) {
    let shapefile = Shapefile::new(
        "../assets/shapefile/tsunami_forecast/tsunami_forecast_simplified.shp",
        "../assets/shapefile/tsunami_forecast/tsunami_forecast_simplified.dbf",
    );

    let mut segments: [Vec<(f32, f32, f32, f32, u16)>; InsetRegion::COUNT] = Default::default();
    let mut tsunami_area_code_buffer = TsunamiAreaCodeBuffer::new();

    for e in shapefile.entries {
        let region = classify_tsunami_area(e.tsunami_area_code.0);
        let internal_code = tsunami_area_code_buffer.insert(e.tsunami_area_code.0);

        for line in e.lines {
            let line = line
                .vertices
                .into_iter()
                .map(|v| (f32::from(v.longitude), f32::from(v.latitude)));

            segments[region.index()].extend(
                crate::projection::to_mercator_segments(line)
                    .into_iter()
                    .map(|(x0, y0, x1, y1)| (x0, y0, x1, y1, internal_code)),
            );
        }
    }

    for region_segments in &mut segments {
        region_segments.sort_by_key(|&(_, _, _, _, code)| code);
    }

    let ranges = segments.each_ref().map(|region_segments| {
        let mut start = 0;
        region_segments
            .chunk_by(|a, b| a.4 == b.4)
            .map(|chunk| {
                let end = start + chunk.len() as u32;
                let range: TsunamiAreaRange = (chunk[0].4, start, end);
                start = end;
                range
            })
            .collect::<Vec<_>>()
    });

    (segments, ranges, tsunami_area_code_buffer.into_buffer())
}
