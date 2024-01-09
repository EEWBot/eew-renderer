use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use shapefile::dbase::FieldValue;
use shapefile::{Shape, ShapeReader};

use renderer_types::*;

#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
struct Line {
    a: usize,
    b: usize,
}

impl Line {
    pub fn new(a: usize, b: usize) -> Self {
        Self {
            a: a.min(b),
            b: a.max(b),
        }
    }

    pub fn to_tuple(&self) -> (u32, u32) {
        (self.a as u32, self.b as u32)
    }
}

impl From<Line> for (u32, u32) {
    fn from(item: Line) -> Self {
        item.to_tuple()
    }
}

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

pub fn read(
    #[allow(non_snake_case)] area_code__pref_code: &HashMap<codes::Area, codes::Pref>,
) -> (
    HashMap<codes::Area, BoundingBox<GeoDegree>>,
    Vec<(f32, f32)>,
    Vec<u32>,
    Vec<u32>,
    Vec<u32>,
) {
    let shp_file = std::fs::File::open(
        "../assets/shapefile/earthquake_detailed/earthquake_detailed_simplified.shp",
    )
    .unwrap();

    let dbf_file = std::fs::File::open(
        "../assets/shapefile/earthquake_detailed/earthquake_detailed_simplified.dbf",
    )
    .unwrap();

    let shape_reader = ShapeReader::new(shp_file).unwrap();
    let dbf_reader = shapefile::dbase::Reader::new(dbf_file).unwrap();
    let mut reader = shapefile::reader::Reader::new(shape_reader, dbf_reader);

    let mut code_to_center = HashMap::new();
    let mut vertex_buffer = VertexBuffer::new();
    let mut indices = Vec::<u32>::new();

    #[allow(non_snake_case)]
    let mut pref_code__lines: HashMap<codes::Pref, Vec<Line>> = HashMap::new();

    #[allow(non_snake_case)]
    let mut area_code__lines: HashMap<codes::Area, Vec<Line>> = HashMap::new();

    for shape_record in reader.iter_shapes_and_records() {
        let (shape, record) = shape_record.unwrap();

        let Shape::Polygon(polygon) = shape else {
            continue;
        };

        let area_code: u32 = match record.get("code").unwrap() {
            FieldValue::Character(Some(s)) => s.parse().expect("Failed to parse 'code' into u16."),
            FieldValue::Character(None) => codes::UNNUMBERED_AREA, // 北方領土・諸外国等がNoneになる
            _ => panic!("💩"),
        };

        for ring in polygon.rings() {
            let points = ring.points();

            let point_index_to_vertex_index: Vec<_> = points
                .iter()
                .map(|v| vertex_buffer.insert((Of32::from(v.x as f32), Of32::from(v.y as f32))))
                .collect();

            let triangles: Vec<_> = earcutr::earcut(
                &points
                    .iter()
                    .flat_map(|vertex| [vertex.x, vertex.y])
                    .collect::<Vec<_>>(),
                &[],
                2,
            )
            .unwrap();

            indices.extend(
                triangles
                    .iter()
                    .map(|n| point_index_to_vertex_index[*n] as u32),
            );

            // 線を引くべき箇所
            if area_code != codes::UNNUMBERED_AREA {
                let area_entry = area_code__lines
                    .entry(area_code)
                    .or_insert_with(|| Default::default());

                let pref_code = area_code__pref_code.get(&area_code).unwrap();

                let pref_entry = pref_code__lines
                    .entry(*pref_code)
                    .or_insert_with(|| Default::default());

                for (n, _) in points.iter().enumerate() {
                    let is_last = n + 1 == points.len();

                    // Line
                    let point = (n, if is_last { 0 } else { n + 1 });

                    // Apply Offset
                    let point = Line::new(
                        point_index_to_vertex_index[point.0],
                        point_index_to_vertex_index[point.1],
                    );

                    area_entry.push(point);
                    pref_entry.push(point);
                }
            }
        }

        if area_code != codes::UNNUMBERED_AREA {
            let bounding_box: BoundingBox<GeoDegree> = (*polygon.bbox()).into();
            code_to_center.insert(area_code, bounding_box);
        }
    }

    use itertools::Itertools;

    fn remove_internal_lines(lines: Vec<Line>) -> Vec<Line> {
        lines
            .into_iter()
            .counts()
            .into_iter()
            .filter(|(line, count)| match count {
                0 => unreachable!(),
                1 => true,
                2 => false,
                _ => panic!("空間が壊れています (internal) {:?}: {count}", line),
            })
            .map(|(line, _count)| line)
            .collect()
    }

    fn remove_outlines(v: &HashMap<u32, Vec<Line>>) -> HashMap<u32, Vec<Line>> {
        let outlines: Vec<Line> = v
            .values()
            .flatten()
            .counts()
            .into_iter()
            .filter(|(line, count)| match count {
                0 => unreachable!(),
                1 => true,
                2 => false,
                _ => panic!("空間が壊れています (outline) {:?}: {count}", line),
            })
            .map(|(line, _count)| *line)
            .collect();

        v.iter()
            .map(|(code, line)| {
                (
                    *code,
                    line.into_iter()
                        .filter(|v| !outlines.contains(&v))
                        .copied()
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    #[allow(non_snake_case)]
    let pref_code__lines: HashMap<codes::Pref, Vec<Line>> = pref_code__lines
        .into_iter()
        .map(|(code, lines)| (code, remove_internal_lines(lines)))
        .collect();

    #[allow(non_snake_case)]
    let pref_code__lines = remove_outlines(&pref_code__lines);

    #[allow(non_snake_case)]
    let area_code__lines: HashMap<codes::Area, Vec<Line>> = area_code__lines
        .into_iter()
        .map(|(code, lines)| (code, remove_internal_lines(lines)))
        .collect();

    #[allow(non_snake_case)]
    let area_code__lines = remove_outlines(&area_code__lines);

    let pref_line_set: HashSet<Line> = HashSet::from_iter(pref_code__lines.values().flatten().copied());

    let pref_lines: Vec<u32> = pref_code__lines
        .values()
        .flatten()
        .map(|line| [line.a, line.b])
        .flatten()
        .map(|v| v as u32)
        .collect();

    let area_lines: Vec<u32> = area_code__lines
        .values()
        .flatten()
        .filter(|line| !pref_line_set.contains(line))
        .map(|line| [line.a, line.b])
        .flatten()
        .map(|v| v as u32)
        .collect();

    (
        code_to_center,
        vertex_buffer.into_buffer(),
        indices,
        area_lines,
        pref_lines,
    )
}
