use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Index;
use std::path::Path;
use geo::{coord, Coord, LineString, MultiLineString, Simplify};
use itertools::Itertools;
use shapefile::dbase::{FieldValue, Record};
use shapefile::{Shape, ShapeReader};


use renderer_types::*;

type Of32 = ordered_float::OrderedFloat<f32>;

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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct Point {
    latitude: Of32,
    longitude: Of32,
}

impl Point {
    fn new(latitude: Of32, longitude: Of32) -> Self {
        Self {
            latitude,
            longitude
        }
    }
}

impl From<shapefile::Point> for Point {
    fn from(value: shapefile::Point) -> Self {
        Self::new(Of32::from(value.y as f32), Of32::from(value.x as f32))
    }
}

impl From<Vertex<GeoDegree>> for Point {
    fn from(value: Vertex<GeoDegree>) -> Self {
        Self::new(Of32::from(value.y), Of32::from(value.x))
    }
}

impl Into<(Of32, Of32)> for Point {
    fn into(self) -> (Of32, Of32) {
        (self.longitude, self.latitude)
    }
}

impl Into<Coord> for Point {
    fn into(self) -> Coord {
        coord! { x: self.longitude.0 as f64, y: self.latitude.0 as f64 }
    }
}

struct Shapefile {
    entries: Vec<AreaRings>,
}

impl Shapefile {
    fn new<P: AsRef<Path>>(
        shp_file: P,
        dbf_file: P,
    ) -> Self {
        let shp_file = std::fs::File::open(shp_file);
        let dbf_file = std::fs::File::open(dbf_file);

        let ((Ok(shp_file), Ok(dbf_file))) = (shp_file, dbf_file) else {
            panic!(r#"EEWBot Renderer requirements is not satisfied.

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
            .filter(|shape_record| shape_record.is_ok())
            .map(|shape_record| shape_record.unwrap())
            .map(|shape_record| AreaRings::try_new(shape_record.0, shape_record.1))
            .filter(|area_rings| area_rings.is_some())
            .map(|area_rings| area_rings.unwrap())
            .collect_vec();

        Self {
            entries,
        }
    }
}

struct AreaRings {
    area_code: codes::Area,
    bounding_box: BoundingBox<GeoDegree>,
    rings: Vec<Ring>,
}

impl AreaRings {
    fn try_new(shape: Shape, record: Record) -> Option<Self> {
        let Shape::Polygon(polygon) = shape else {
            return None
        };
        let area_code: codes::Area = match record.get("code").unwrap() {
            FieldValue::Character(Some(c)) => {
                match c.parse() {
                    Ok(c) => c,
                    Err(_) => panic!("ｺﾜｯ…ｺﾜれたshapefileきた！"),
                }
            },
            FieldValue::Character(None) => codes::UNNUMBERED_AREA, // 北方領土・諸外国等がNoneになる
            _ => panic!("知らないshapefileきた？🤔"),
        };
        let bounding_box = (*polygon.bbox()).into();
        let rings = polygon.rings().iter().map(|ring| Ring::new(ring.points())).collect_vec();

        Some(
            Self {
                area_code,
                bounding_box,
                rings,
            }
        )
    }
}

struct Ring {
    points: Vec<Point>,
}

impl Ring {
    fn new(points: &[shapefile::Point]) -> Self {
        let points = points.iter().map(|p| (*p).into()).collect_vec();
        Self {
            points,
        }
    }

    fn iter_adjacent_points(&self) -> AdjacentPointsIter {
        AdjacentPointsIter::new(&self.points)
    }

    fn triangulate(&self) -> Vec<&Point> {
        earcutr::earcut(
            self.points.iter().flat_map(|p| vec![p.longitude.0, p.latitude.0]).collect_vec().as_slice(),
            &[],
            2,
        )
            .unwrap()
            .iter()
            .map(|i| &self.points[*i])
            .collect()
    }
}

struct AdjacentPointsIter<'a> {
    points: &'a Vec<Point>,
    index: usize,
}

impl <'a> AdjacentPointsIter<'a> {
    fn new(points: &'a Vec<Point>) -> Self {
        Self {
            points,
            index: 0,
        }
    }
}

impl <'a> Iterator for AdjacentPointsIter<'a> {
    type Item = AdjacentPointsIterItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.points.len() {
            None
        } else {
            let previous_index = if self.index == 0 {
                self.points.len() - 1
            } else {
                self.index - 1
            };
            let current_index = self.index;
            let next_index = if self.index == self.points.len() - 1 {
                0
            } else {
                self.index + 1
            };

            self.index += 1;

            Some(Self::Item::new(
                self.points.get(previous_index).unwrap(),
                self.points.get(current_index).unwrap(),
                self.points.get(next_index).unwrap(),
            ))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.points.len() - self.index;
        (size, Some(size))
    }
}

impl ExactSizeIterator for AdjacentPointsIter<'_> {}

struct AdjacentPointsIterItem<'a> {
    previous: &'a Point,
    current: &'a Point,
    next: &'a Point,
}

impl<'a> AdjacentPointsIterItem<'a> {
    fn new(previous: &'a Point, current: &'a Point, next: &'a Point) -> Self {
        Self {
            previous,
            current,
            next,
        }
    }
}

struct PointReferences<'a> {
    shapefile: &'a Shapefile,
    map: HashMap<&'a Point, PointReference<'a>>,
}

impl <'a> PointReferences<'a> {
    fn tally_of(shapefile: &'a Shapefile, area_to_pref: &'a HashMap<codes::Area, codes::Pref>) -> Self {
        let mut map: HashMap<&Point, PointReference> = HashMap::new();

        shapefile
            .entries
            .iter()
            .for_each(|area_rings| {
                let area_code = area_rings.area_code;

                area_rings
                    .rings
                    .iter()
                    .for_each(|ring| {
                        ring
                            .iter_adjacent_points()
                            .for_each(|point_set| {
                                let reference = map
                                    .entry(point_set.current)
                                    .or_insert(PointReference::new(area_to_pref));

                                reference.mark_area(area_code);
                                reference.mark_point(point_set.previous);
                                reference.mark_point(point_set.next);
                            });
                    });
            });

        Self {
            shapefile,
            map,
        }
    }

    fn cut_points(&self) -> Vec<&'a Point> {
        self
            .map
            .iter()
            .filter(|(_, r)| r.adjacent_points_count() >= 3)
            .map(|(p, _)| *p)
            .collect()
    }
}

struct PointReference<'a> {
    area_to_pref: &'a HashMap<codes::Area, codes::Pref>,
    areas: HashSet<codes::Area>,
    adjacent_points: HashSet<&'a Point>,
}

impl <'a> PointReference<'a> {
    fn new(area_to_pref: &'a HashMap<codes::Area, codes::Pref>) -> Self {
        Self {
            area_to_pref,
            areas: HashSet::new(),
            adjacent_points: HashSet::new(),
        }
    }

    fn mark_area(&mut self, area: codes::Area) {
        self.areas.insert(area);
    }

    fn mark_point(&mut self, point: &'a Point) {
        self.adjacent_points.insert(point);
    }

    fn area_references(&self) -> &HashSet<codes::Area> {
        &self.areas
    }

    fn pref_references(&self) -> HashSet<codes::Pref> {
        let areas = self
            .area_references()
            .iter()
            .filter(|a| **a != codes::UNNUMBERED_AREA)
            .map(|a| *self.area_to_pref.get(a).unwrap());
        HashSet::from_iter(areas)
    }

    fn adjacent_points_count(&self) -> usize {
        self.adjacent_points.len()
    }
}

#[derive(Debug)]
struct Line<'a> {
    vertices: Vec<&'a Point>,
}

impl <'a> Line<'a> {
    fn new(vertices: Vec<&'a Point>) -> Self {
        Self {
            vertices,
        }
    }

    fn join_first(&mut self, mut other: Self) {
        if other.vertices.last() == self.vertices.first() {
            other.vertices.pop();
        }
        other.vertices.append(&mut self.vertices);
        self.vertices = other.vertices;
    }

    fn area_reference_count(&self, references: &PointReferences) -> usize {
        let first = self.vertices.first().unwrap();
        let last = self.vertices.last().unwrap();

        let first = references.map.get(first).unwrap();
        let last = references.map.get(last).unwrap();

        first.area_references().intersection(last.area_references()).count()
    }

    fn pref_reference_count(&self, references: &PointReferences) -> usize {
        let first = self.vertices.first().unwrap();
        let last = self.vertices.last().unwrap();

        let first = references.map.get(first).unwrap();
        let last = references.map.get(last).unwrap();

        first.pref_references().intersection(&last.pref_references()).count()
    }
}

impl <'a> From<&'a [Point]> for Line<'a> {
    fn from(value: &'a [Point]) -> Self {
        Self::new(value.iter().collect())
    }
}

impl Into<LineString> for &Line<'_> {
    fn into(self) -> LineString {
        LineString::new(self.vertices.iter().map(|v| (**v).into()).collect_vec())
    }
}

impl PartialEq for Line<'_> {
    fn eq(&self, other: &Self) -> bool {
        let is_self_ordered = self.vertices.first().unwrap() < self.vertices.last().unwrap();
        let is_other_hand_ordered = other.vertices.first().unwrap() < other.vertices.last().unwrap();

        if is_self_ordered == is_other_hand_ordered {
            itertools::equal(&self.vertices, &other.vertices)
        } else {
            itertools::equal(self.vertices.iter().rev(), &other.vertices)
        }
    }
}

impl Eq for Line<'_> {}

impl Hash for Line<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let first = self.vertices.first().unwrap();
        let last = self.vertices.last().unwrap();

        if first < last {
            self.vertices.iter().for_each(|v| v.hash(state));
        } else {
            self.vertices.iter().rev().for_each(|v| v.hash(state));
        }
    }
}

pub fn read(
    #[allow(non_snake_case)] area_code__pref_code: &HashMap<codes::Area, codes::Pref>,
) -> (
    HashMap<codes::Area, BoundingBox<GeoDegree>>, // area_bounding_box
    Vec<(f32, f32)>, // vertex_buffer
    Vec<u32>, // map_indices
    Vec<Vec<u32>>, // area_lines
    Vec<Vec<u32>>, // pref_lines
    Vec<(f32, usize)> // scale_level_map
) {
    let shapefile = Shapefile::new(
        "../assets/shapefile/earthquake_detailed/earthquake_detailed_simplified.shp",
        "../assets/shapefile/earthquake_detailed/earthquake_detailed_simplified.dbf"
    );
    let mut vertex_buffer = VertexBuffer::new();

    // @Siro_256 にゃ～っ…！ (ΦωΦ）

    let area_bounding_box: HashMap<codes::Area, BoundingBox<GeoDegree>> = shapefile
        .entries
        .iter()
        .filter(|area_rings| area_rings.area_code != codes::UNNUMBERED_AREA)
        .map(|area_rings| (area_rings.area_code, area_rings.bounding_box))
        .collect();

    let map_indices = shapefile
        .entries
        .iter()
        .flat_map(|area_rings| &area_rings.rings)
        .flat_map(|r| r.triangulate())
        .map(|p| vertex_buffer.insert((*p).into()) as u32)
        .collect_vec();

    let references = PointReferences::tally_of(&shapefile, area_code__pref_code);

    let rings = shapefile.entries.iter().flat_map(|area_rings| &area_rings.rings).collect_vec();
    let cut_points = references.cut_points();
    let lines = cut_rings(&rings, &cut_points);
    let lines = lines.into_iter().counts().into_iter().filter_map(|(l, c)| if c > 1 { Some(l) } else { None }).collect_vec();

    let area_lines = lines.iter().filter(|l| l.pref_reference_count(&references) == 1).collect_vec();
    let pref_lines = lines.iter().filter(|l| l.pref_reference_count(&references) >= 2).collect_vec();

    // let lod_details = [
    //     (100.0_f32.powf(1.00), 0.010),
    //     (100.0_f32.powf(0.96), 0.014),
    //     (100.0_f32.powf(0.92), 0.018),
    //     (100.0_f32.powf(0.88), 0.022),
    //     (100.0_f32.powf(0.84), 0.026),
    //     (100.0_f32.powf(0.80), 0.030),
    //     (100.0_f32.powf(0.76), 0.034),
    //     (100.0_f32.powf(0.72), 0.038),
    //     (100.0_f32.powf(0.68), 0.042),
    //     (100.0_f32.powf(0.64), 0.046),
    //     (100.0_f32.powf(0.60), 0.050),
    //     (100.0_f32.powf(0.56), 0.054),
    //     (100.0_f32.powf(0.52), 0.058),
    //     (100.0_f32.powf(0.48), 0.062),
    //     (100.0_f32.powf(0.44), 0.066),
    //     (100.0_f32.powf(0.40), 0.070),
    //     (100.0_f32.powf(0.36), 0.074),
    //     (100.0_f32.powf(0.32), 0.078),
    //     (100.0_f32.powf(0.28), 0.082),
    //     (100.0_f32.powf(0.24), 0.086),
    //     (100.0_f32.powf(0.20), 0.090),
    //     (100.0_f32.powf(0.16), 0.094),
    //     (100.0_f32.powf(0.12), 0.098),
    //     (0.0, 0.12),
    // ];

    let lod_details = [
        (100.0_f32.powf(1.00), 0.000),
        (100.0_f32.powf(0.96), 0.001),
        (100.0_f32.powf(0.92), 0.002),
        (100.0_f32.powf(0.88), 0.003),
        (100.0_f32.powf(0.84), 0.004),
        (100.0_f32.powf(0.80), 0.005),
        (100.0_f32.powf(0.76), 0.006),
        (100.0_f32.powf(0.72), 0.007),
        (100.0_f32.powf(0.68), 0.008),
        (100.0_f32.powf(0.64), 0.009),
        (100.0_f32.powf(0.60), 0.010),
        (100.0_f32.powf(0.56), 0.011),
        (100.0_f32.powf(0.52), 0.012),
        (100.0_f32.powf(0.48), 0.013),
        (100.0_f32.powf(0.44), 0.014),
        (100.0_f32.powf(0.40), 0.015),
        (100.0_f32.powf(0.36), 0.016),
        (100.0_f32.powf(0.32), 0.017),
        (100.0_f32.powf(0.28), 0.018),
        (100.0_f32.powf(0.24), 0.019),
        (100.0_f32.powf(0.60), 0.020),
        (100.0_f32.powf(0.56), 0.021),
        (100.0_f32.powf(0.52), 0.022),
        (100.0_f32.powf(0.48), 0.023),
        (100.0_f32.powf(0.44), 0.024),
        (100.0_f32.powf(0.40), 0.025),
        (100.0_f32.powf(0.36), 0.026),
        (100.0_f32.powf(0.32), 0.027),
        (100.0_f32.powf(0.28), 0.028),
        (100.0_f32.powf(0.24), 0.029),
        (100.0_f32.powf(0.60), 0.030),
        (100.0_f32.powf(0.56), 0.031),
        (100.0_f32.powf(0.52), 0.032),
        (100.0_f32.powf(0.48), 0.033),
        (100.0_f32.powf(0.44), 0.034),
        (100.0_f32.powf(0.40), 0.035),
        (100.0_f32.powf(0.36), 0.036),
        (100.0_f32.powf(0.32), 0.037),
        (100.0_f32.powf(0.28), 0.038),
        (100.0_f32.powf(0.24), 0.039),
    ];

    let area_lines = gen_lod(&mut vertex_buffer, &lod_details, area_lines);
    let pref_lines = gen_lod(&mut vertex_buffer, &lod_details, pref_lines);

    let scale_level_map = lod_details
        .into_iter()
        .enumerate()
        .map(|(i, (s, _))| (s, i))
        .collect_vec();

    // ฅ•ω•ฅ Meow

    // (ΦωΦ) < Meow !
    {(
        area_bounding_box,
        vertex_buffer.into_buffer(),
        map_indices,
        area_lines,
        pref_lines,
        scale_level_map,
    )}
}

fn cut_rings<'a>(rings: &'a Vec<&Ring>, cut_points: &Vec<&'a Point>) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    rings
        .iter()
        .for_each(|ring| {
            let points = &ring.points;
            let mut ring_lines: Vec<Line> = Vec::new();
            let mut start_index: usize = 0;

            for (i, p) in points.iter().enumerate() {
                if i == 0 { continue }
                if i == points.len() - 1 { continue }
                if !cut_points.contains(&p) { continue }
                ring_lines.push((&points[start_index..=i]).into());
                start_index = i;
            }
            ring_lines.push((&points[start_index..points.len()]).into());

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
    base_lines: Vec<&Line>
) -> Vec<Vec<u32>> {
    let geo_lines: Vec<LineString> = base_lines.into_iter().map_into().collect();
    lod_details
        .iter()
        .map(|(_, e)| geo_lines.iter().map(|l| l.simplify(e)).collect_vec())
        .map(|l| {
            let mut v = Vec::new();
            for l in l {
                let l = l
                    .0
                    .iter()
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

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use crate::parse_shapefile::{Line, Of32, Point};

    #[test]
    fn line_eq() {
        let l1 = vec![
            Point::new(Of32::from(0.0), Of32::from(0.0)),
            Point::new(Of32::from(1.0), Of32::from(1.0)),
            Point::new(Of32::from(2.0), Of32::from(2.0)),
            Point::new(Of32::from(3.0), Of32::from(3.0)),
        ];
        let l2 = vec![
            Point::new(Of32::from(3.0), Of32::from(3.0)),
            Point::new(Of32::from(2.0), Of32::from(2.0)),
            Point::new(Of32::from(1.0), Of32::from(1.0)),
            Point::new(Of32::from(0.0), Of32::from(0.0)),
        ];
        let l1 = Line::new(l1.iter().map(|p| p).collect_vec());
        let l2 = Line::new(l2.iter().map(|p| p).collect_vec());

        assert_eq!(l1, l1);
        assert_eq!(l1, l2);
    }
}
