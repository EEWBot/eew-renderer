#![allow(clippy::type_complexity)]

use std::collections::HashMap;

use shapefile::{Shape, ShapeReader};

use crate::math::*;

const WEB_MERCATOR_MAX_LATITUD: f32 = 85.051_13;

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

/// 極点での発散を避けるため、球面Web Mercatorで一般的に使用される打ち切り緯度 ±85.05113° を、レンダラーの描画上限として採用する。
/// 経度はそのまま返す。
fn clamp_mercator_latitude(point: Point) -> Point {
    let latitude = point
        .latitude
        .0
        .clamp(-WEB_MERCATOR_MAX_LATITUD, WEB_MERCATOR_MAX_LATITUD);

    Point::new(Of32::from(latitude), point.longitude)
}

struct WorldRings {
    rings: Vec<Ring>,
}

impl WorldRings {
    fn from_shape(shape: Shape) -> Self {
        let Shape::Polygon(polygon) = shape else {
            panic!("world shapefile contains a non-Polygon shape");
        };

        let rings: Vec<_> = polygon
            .rings()
            .iter()
            .map(|ring| Ring::from(ring.points().to_vec()))
            .collect();

        Self { rings }
    }
}

struct Shapefile {
    entries: Vec<WorldRings>,
}

impl Shapefile {
    fn new() -> Self {
        let shp_file = std::fs::File::open("../assets/shapefile/world/world.shp");

        let Ok(shp_file) = shp_file else {
            panic!(
                r#"EEWBot Renderer requirements is not satisfied.

World shape file is not found.
 - assets/shapefile/world/world.shp

Please follow:
  https://github.com/EEWBot/eew-renderer/wiki#shapefile-%E5%85%A5%E6%89%8B%E5%85%88"#
            )
        };

        let mut shape_reader = ShapeReader::new(shp_file).unwrap();

        let entries = shape_reader
            .iter_shapes()
            .map(|shape| shape.expect("Failed to read a shape from world shapefile"))
            .map(WorldRings::from_shape)
            .collect();

        Self { entries }
    }
}

pub fn read() -> (
    Vec<(f32, f32)>, // vertices
    Vec<u32>,        // indices
) {
    let shapefile = Shapefile::new();

    let mut vertex_buffer = VertexBuffer::new();

    let world_indices: Vec<u32> = shapefile
        .entries
        .iter()
        .flat_map(|world_rings| &world_rings.rings)
        .flat_map(|ring| ring.triangulate())
        .map(clamp_mercator_latitude)
        .map(|point| {
            u32::try_from(vertex_buffer.insert(point.into()))
                .expect("world shapefile has too many vertices")
        })
        .collect();

    let world_vertices = vertex_buffer.into_buffer();

    assert!(
        !world_vertices.is_empty(),
        "world shapefile produced no vertices"
    );
    assert!(
        !world_indices.is_empty(),
        "world shapefile produced no indices"
    );

    (world_vertices, world_indices)
}
