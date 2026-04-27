#![allow(clippy::type_complexity)]

use std::collections::{HashMap, HashSet};
use std::path::Path;

use geo::Simplify;
use itertools::Itertools;
use shapefile::dbase::{FieldValue, Record};
use shapefile::{Shape, ShapeReader};

use renderer_types::*;

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

struct Shapefile {
    entries: Vec<細分区域Rings>,
}

impl Shapefile {
    fn new<P: AsRef<Path>>(shp_file: P, dbf_file: P) -> Self {
        let shp_file = std::fs::File::open(shp_file);
        let dbf_file = std::fs::File::open(dbf_file);

        let (Ok(shp_file), Ok(dbf_file)) = (shp_file, dbf_file) else {
            panic!(
                r#"EEWBot Renderer requirements is not satisfied.

Simplified shape files are not found.
 - assets/shapefile/earthquake_detailed/earthquake_detailed_simplified.shp
 - assets/shapefile/earthquake_detailed/earthquake_detailed_simplified.dbf

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
            .flat_map(|shape_record| 細分区域Rings::try_new(shape_record.0, shape_record.1))
            .collect();

        Self { entries }
    }
}

struct 細分区域Rings {
    細分区域: codes::地震情報細分区域,
    bounding_box: BoundingBox<GeoDegree>,
    rings: Vec<Ring>,
}

impl 細分区域Rings {
    fn try_new(shape: Shape, record: Record) -> Option<Self> {
        let Shape::Polygon(polygon) = shape else {
            return None;
        };

        let 細分区域: codes::地震情報細分区域 = match record.get("code").unwrap() {
            FieldValue::Character(Some(c)) => match c.parse() {
                Ok(c) => codes::地震情報細分区域(c),
                Err(_) => panic!("ｺﾜｯ…ｺﾜれたshapefileきた！"),
            },

            FieldValue::Character(None) => codes::地震情報細分区域::UNNUMBERED, // 北方領土・諸外国等がNoneになる
            _ => panic!("知らないshapefileきた？🤔"),
        };

        let bounding_box = (*polygon.bbox()).into();
        let rings = polygon
            .rings()
            .iter()
            .map(|ring| Ring::from(ring.points().to_vec()))
            .collect();

        Some(Self {
            細分区域,
            bounding_box,
            rings,
        })
    }
}

pub(crate) struct PointReferences<'a> {
    map: HashMap<Point, PointReference<'a>>,
}

impl<'a> PointReferences<'a> {
    fn tally_of(
        shapefile: &'a Shapefile,
        細分区域_to_都道府県等: &'a HashMap<codes::地震情報細分区域, codes::地震情報都道府県等>,
    ) -> Self {
        let mut map: HashMap<Point, PointReference> = HashMap::new();

        shapefile.entries.iter().for_each(|rings| {
            let 細分区域 = rings.細分区域;

            rings.rings.iter().for_each(|ring| {
                ring.iter_adjacent_points().for_each(|point_set| {
                    let reference = map
                        .entry(point_set.current)
                        .or_insert(PointReference::new(細分区域_to_都道府県等));

                    reference.mark_細分区域(細分区域);
                    reference.mark_point(point_set.previous);
                    reference.mark_point(point_set.next);
                });
            });
        });

        Self { map }
    }

    fn cut_points(&self) -> Vec<Point> {
        self.map
            .iter()
            .filter(|(_, r)| r.adjacent_points_count() >= 3)
            .map(|(p, _)| *p)
            .collect()
    }

    fn 都道府県等_reference_count(&self, line: &Line) -> usize {
        let first = self.map.get(line.vertices.first().unwrap()).unwrap();
        let last = self.map.get(line.vertices.last().unwrap()).unwrap();

        first
            .都道府県等_references()
            .intersection(&last.都道府県等_references())
            .count()
    }
}

pub(crate) struct PointReference<'a> {
    細分区域_to_都道府県等: &'a HashMap<codes::地震情報細分区域, codes::地震情報都道府県等>,
    細分区域s: HashSet<codes::地震情報細分区域>,
    adjacent_points: HashSet<Point>,
}

impl<'a> PointReference<'a> {
    fn new(
        細分区域_to_都道府県等: &'a HashMap<codes::地震情報細分区域, codes::地震情報都道府県等>
    ) -> Self {
        Self {
            細分区域_to_都道府県等,
            細分区域s: HashSet::new(),
            adjacent_points: HashSet::new(),
        }
    }

    fn mark_細分区域(&mut self, 細分区域: codes::地震情報細分区域) {
        self.細分区域s.insert(細分区域);
    }

    fn mark_point(&mut self, point: Point) {
        self.adjacent_points.insert(point);
    }

    fn 細分区域_references(&self) -> &HashSet<codes::地震情報細分区域> {
        &self.細分区域s
    }

    pub(crate) fn 都道府県等_references(&self) -> HashSet<codes::地震情報都道府県等> {
        let 都道府県等s = self
            .細分区域_references()
            .iter()
            .filter(|a| **a != codes::地震情報細分区域::UNNUMBERED)
            .map(|a| *self.細分区域_to_都道府県等.get(a).unwrap());

        HashSet::from_iter(都道府県等s)
    }

    fn adjacent_points_count(&self) -> usize {
        self.adjacent_points.len()
    }
}

pub fn read(
    #[allow(non_snake_case)] 細分区域_to_都道府県等: &HashMap<
        codes::地震情報細分区域,
        codes::地震情報都道府県等,
    >,
) -> (
    HashMap<codes::地震情報細分区域, BoundingBox<GeoDegree>>, // area_bounding_box
    HashMap<codes::地震情報細分区域, Vertex<GeoDegree>>,      // area_centers
    Vec<(f32, f32)>,                                          // vertex_buffer
    Vec<u32>,                                                 // map_indices
    Vec<Vec<u32>>,                                            // area_lines
    Vec<Vec<u32>>,                                            // pref_lines
    Vec<(f32, usize)>,                                        // scale_level_map
) {
    let shapefile = Shapefile::new(
        "../assets/shapefile/earthquake_detailed/earthquake_detailed_simplified.shp",
        "../assets/shapefile/earthquake_detailed/earthquake_detailed_simplified.dbf",
    );
    let mut vertex_buffer = VertexBuffer::new();

    // @Siro_256 にゃ～っ…！ (ΦωΦ）

    let 細分区域_centers = calculate_細分区域_centers(&shapefile);

    let 細分区域_bounding_box: HashMap<codes::地震情報細分区域, BoundingBox<GeoDegree>> =
        shapefile
            .entries
            .iter()
            .filter(|rings| {
                rings.細分区域 != codes::地震情報細分区域::UNNUMBERED
            })
            .map(|rings| (rings.細分区域, rings.bounding_box))
            .collect();

    let map_indices = shapefile
        .entries
        .iter()
        .flat_map(|rings| &rings.rings)
        .flat_map(|ring| ring.triangulate())
        .map(|point| vertex_buffer.insert(point.into()) as u32)
        .collect();

    let references = PointReferences::tally_of(&shapefile, 細分区域_to_都道府県等);

    let rings = shapefile
        .entries
        .iter()
        .flat_map(|area_rings| &area_rings.rings)
        .collect_vec();

    let cut_points = references.cut_points();
    let lines = cut_rings(&rings, &cut_points);

    let lines: Vec<_> = lines
        .into_iter()
        .counts()
        .into_iter()
        .filter_map(|(l, c)| if c > 1 { Some(l) } else { None })
        .collect();

    let 細分区域_lines = lines
        .iter()
        .filter(|l| references.都道府県等_reference_count(l) == 1)
        .collect_vec();

    let 都道府県等_lines = lines
        .iter()
        .filter(|l| references.都道府県等_reference_count(l) >= 2)
        .collect_vec();

    let lod_details = [
        (100.0_f32.powf(1.00), 0.000),
        (100.0_f32.powf(0.96), 0.003),
        (100.0_f32.powf(0.92), 0.006),
        (100.0_f32.powf(0.88), 0.009),
        (100.0_f32.powf(0.84), 0.012),
        (100.0_f32.powf(0.80), 0.015),
        (100.0_f32.powf(0.76), 0.018),
        (100.0_f32.powf(0.72), 0.021),
        (100.0_f32.powf(0.68), 0.024),
        (100.0_f32.powf(0.64), 0.027),
        (100.0_f32.powf(0.60), 0.030),
        (100.0_f32.powf(0.56), 0.033),
        (100.0_f32.powf(0.52), 0.036),
        (100.0_f32.powf(0.48), 0.039),
        (100.0_f32.powf(0.44), 0.042),
        (100.0_f32.powf(0.40), 0.045),
        (100.0_f32.powf(0.36), 0.048),
        (100.0_f32.powf(0.32), 0.051),
        (100.0_f32.powf(0.28), 0.054),
        (100.0_f32.powf(0.24), 0.057),
        (100.0_f32.powf(0.60), 0.060),
        (100.0_f32.powf(0.56), 0.063),
        (100.0_f32.powf(0.52), 0.066),
        (100.0_f32.powf(0.48), 0.069),
        (100.0_f32.powf(0.44), 0.072),
        (100.0_f32.powf(0.40), 0.075),
        (100.0_f32.powf(0.36), 0.078),
        (100.0_f32.powf(0.32), 0.081),
        (100.0_f32.powf(0.28), 0.084),
        (100.0_f32.powf(0.24), 0.087),
        (100.0_f32.powf(0.60), 0.090),
        (100.0_f32.powf(0.56), 0.093),
        (100.0_f32.powf(0.52), 0.096),
        (100.0_f32.powf(0.48), 0.099),
        (100.0_f32.powf(0.44), 0.102),
        (100.0_f32.powf(0.40), 0.105),
        (100.0_f32.powf(0.36), 0.108),
        (100.0_f32.powf(0.32), 0.111),
        (100.0_f32.powf(0.28), 0.114),
        (100.0_f32.powf(0.24), 0.117),
    ];

    let 細分区域_lines = gen_lod(&mut vertex_buffer, &lod_details, &細分区域_lines);
    let 都道府県等_lines = gen_lod(&mut vertex_buffer, &lod_details, &都道府県等_lines);

    let scale_level_map = lod_details
        .into_iter()
        .enumerate()
        .map(|(i, (s, _))| (s, i))
        .collect();

    // ฅ•ω•ฅ Meow

    // (ΦωΦ) < Meow !
    {
        (
            細分区域_bounding_box,
            細分区域_centers,
            vertex_buffer.into_buffer(),
            map_indices,
            細分区域_lines,
            都道府県等_lines,
            scale_level_map,
        )
    }
}

fn cut_rings(rings: &[&Ring], cut_points: &[Point]) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();

    rings.iter().for_each(|ring| {
        let points = ring.points();
        let mut ring_lines: Vec<Line> = Vec::new();
        let mut start_index: usize = 0;

        for (i, p) in points.iter().enumerate() {
            if i == 0 {
                continue;
            }
            if i == points.len() - 1 {
                continue;
            }
            if !cut_points.contains(p) {
                continue;
            }
            ring_lines.push(Line::new(&points[start_index..=i]));
            start_index = i;
        }
        ring_lines.push(Line::new(&points[start_index..points.len()]));

        if ring_lines.len() >= 2 {
            let first_point = ring_lines.first().unwrap().vertices.first().unwrap();

            if !cut_points.contains(first_point) {
                let last_line = ring_lines.pop().unwrap();
                ring_lines.first_mut().unwrap().join_first(last_line);
            }
        }

        lines.append(&mut ring_lines);
    });

    lines
}

fn gen_lod(
    vertex_buffer: &mut VertexBuffer,
    lod_details: &[(f32, f64)],
    base_lines: &[&Line],
) -> Vec<Vec<u32>> {
    let geo_lines: Vec<geo::LineString> = base_lines
        .iter()
        .map(|l| geo::LineString::from(*l))
        .collect();

    lod_details
        .iter()
        .map(|(_, e)| geo_lines.iter().map(|l| l.simplify(*e)).collect_vec())
        .map(|l| {
            let mut v = Vec::new();
            for l in l {
                let l =
                    l.0.iter()
                        .map(|c| (Of32::from(c.x as f32), Of32::from(c.y as f32)))
                        .map(|v| vertex_buffer.insert(v) as u32);
                v.extend(l);
                v.push(0);
            }
            v.pop();
            v
        })
        .collect()
}

fn calculate_細分区域_centers(
    shapefile: &Shapefile,
) -> HashMap<codes::地震情報細分区域, Vertex<GeoDegree>> {
    use geo::{
        algorithm::{Area, Centroid},
        LineString, Polygon,
    };

    let weighted_vectors: HashMap<codes::地震情報細分区域, Vec<(f64, geo::Point)>> = shapefile
        .entries
        .iter()
        .filter(|rings| rings.細分区域 != codes::地震情報細分区域::UNNUMBERED)
        .map(|rings| {
            let polygons: Vec<(f64, geo::Point)> = rings
                .rings
                .iter()
                .map(|ring| LineString::new(ring.points().iter().map(|p| p.into()).collect_vec()))
                .map(|geo_line_string| Polygon::new(geo_line_string, vec![]))
                .map(|geo_polygon| (geo_polygon.unsigned_area(), geo_polygon.centroid().unwrap()))
                .collect();

            (rings.細分区域, polygons)
        })
        .collect();

    let centers: HashMap<codes::地震情報細分区域, Point> = weighted_vectors
        .into_iter()
        .map(|(細分区域, weighted_vectors)| {
            let weight: f64 = weighted_vectors
                .iter()
                .map(|(weight, _vector)| weight)
                .sum();

            let vector: Point = weighted_vectors
                .iter()
                .map(|(weight, vector)| {
                    let vector: Point = vector.into();
                    vector.multiply_by(*weight as f32)
                })
                .fold(Point::new(0.0.into(), 0.0.into()), |a, b| a + b);

            (細分区域, vector.divide_by(weight as f32))
        })
        .collect();

    let centers: HashMap<codes::地震情報細分区域, Vertex<GeoDegree>> = centers
        .into_iter()
        .map(|(code, center)| {
            (
                code,
                Vertex::new(center.latitude.into(), center.longitude.into()),
            )
        })
        .collect();

    centers
}
