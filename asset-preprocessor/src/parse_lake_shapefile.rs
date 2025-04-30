#![allow(clippy::type_complexity)]

use std::collections::HashMap;
use std::path::Path;

use shapefile::dbase::Record;
use shapefile::{Shape, ShapeReader};

use crate::math::*;

struct VertexBuffer {
    buffer: Vec<(Of32, Of32)>,
    dict: HashMap<(Of32, Of32), usize>,
}

impl VertexBuffer {
    fn new() -> Self {
        Self {
            buffer: Default::default(),
            dict: Default::default(),
        }
    }

    fn insert(&mut self, v: (Of32, Of32)) -> usize {
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

    fn into_buffer(self) -> Vec<(f32, f32)> {
        self.buffer.into_iter().map(|(x, y)| (x.0, y.0)).collect()
    }
}

struct AreaRings {
    rings: Vec<Ring>,
}

impl AreaRings {
    fn try_new(shape: Shape, _record: Record) -> Option<Self> {
        let Shape::Polygon(polygon) = shape else {
            return None;
        };

        let rings: Vec<_> = polygon
            .rings()
            .iter()
            .map(|ring| Ring::from(ring.points().to_vec()))
            .collect();

        Some(Self { rings })
    }
}

struct Shapefile {
    entries: Vec<AreaRings>,
}

impl Shapefile {
    fn new<P: AsRef<Path>>(shp_file: P, dbf_file: P) -> Self {
        let shp_file = std::fs::File::open(shp_file);
        let dbf_file = std::fs::File::open(dbf_file);

        let (Ok(shp_file), Ok(dbf_file)) = (shp_file, dbf_file) else {
            panic!(
                r#"EEWBot Renderer requirements is not satisfied.

Simplified shape files are not found.
 - assets/shapefile/lake_reduced/lake_reduced_simplified.shp
 - assets/shapefile/lake_reduced/lake_reduced_simplified.dbf

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
            .flat_map(|shape_record| AreaRings::try_new(shape_record.0, shape_record.1))
            .collect();

        Self { entries }
    }
}

pub fn read() -> (
    Vec<(f32, f32)>, // vertices
    Vec<u32>,        // indices
) {
    let shapefile = Shapefile::new(
        "../assets/shapefile/lake_reduced/lake_reduced_simplified.shp",
        "../assets/shapefile/lake_reduced/lake_reduced_simplified.dbf",
    );

    let mut vertex_buffer = VertexBuffer::new();

    // @Siro_256 にゃ～っ…！ (ΦωΦ）

    let map_indices = shapefile
        .entries
        .iter()
        .flat_map(|area_rings| &area_rings.rings)
        .flat_map(|r| r.triangulate())
        .map(|p| vertex_buffer.insert(p.into()) as u32)
        .collect();

    // ฅ•ω•ฅ Meow

    // (ΦωΦ) < Meow !
    (vertex_buffer.into_buffer(), map_indices)
}
