#![allow(clippy::type_complexity)]

use std::collections::HashMap;
use std::path::Path;

use shapefile::dbase::{FieldValue, Record};
use shapefile::{Shape, ShapeReader};

use crate::math::*;
use renderer_types::codes;

struct VertexBuffer {
    buffer: Vec<(Of32, Of32, u32)>,
    dict: HashMap<(Of32, Of32, u32), usize>,
}

impl VertexBuffer {
    fn new() -> Self {
        Self {
            buffer: Default::default(),
            dict: Default::default(),
        }
    }

    fn insert(&mut self, v: (Of32, Of32, u32)) -> usize {
        match self.dict.get(&v) {
            Some(index) => *index,
            None => {
                self.buffer.push(v);
                let index = self.buffer.len() - 1;
                self.dict.insert(v, index);
                index
            }
        }
    }

    fn into_buffer(self) -> Vec<(f32, f32, u32)> {
        self.buffer.into_iter().map(|(x, y, code)| (x.0, y.0, code)).collect()
    }
}

struct AreaLines {
    lines: Vec<Line>,
    tsunami_area_code: codes::TsunamiArea,
}

impl AreaLines {
    fn try_new(shape: Shape, record: Record) -> Option<Self> {
        let Shape::Polyline(polyline) = shape else {
            return None;
        };

        let tsunami_area_code: codes::TsunamiArea = match record.get("code").unwrap() {
            FieldValue::Character(Some(c)) => match c.parse() {
                Ok(c) => c,
                Err(_) => panic!("Failed to parse code"),
            },
            FieldValue::Character(None) => panic!("UNNUMBERED_AREA DETECTED!"),
            _ => panic!("Failed to get code"),
        };

        // 帰属未定 (極小の島など)
        if tsunami_area_code == 0 {
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

pub fn read() -> (
    Vec<(f32, f32, u32)>, // vertices
    Vec<u32>,        // indices
)
{
    let shapefile = Shapefile::new(
        "../assets/shapefile/tsunami_forecast/tsunami_forecast_simplified.shp",
        "../assets/shapefile/tsunami_forecast/tsunami_forecast_simplified.dbf",
    );

    let mut vertex_buffer = VertexBuffer::new();
    let mut lines = Vec::new();

    for e in shapefile.entries {
        for line in e.lines {
            let line: Vec<u32> = line.vertices.into_iter().map(|v|
                vertex_buffer.insert(
                    (Of32::from(v.latitude), Of32::from(v.longitude), e.tsunami_area_code)
                ) as u32
            ).collect();

            lines.extend_from_slice(&line);

            // append NUL vertex
            lines.push(0);
        }
    }

    (vertex_buffer.into_buffer(), lines)
}
