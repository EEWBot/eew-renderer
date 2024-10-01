use renderer_types::{GeoDegree, Vertex};
use itertools::Itertools;
use std::hash::{Hash, Hasher};

pub(crate) type Of32 = ordered_float::OrderedFloat<f32>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) struct Point {
    latitude: Of32,
    longitude: Of32,
}

impl Point {
    pub(crate) fn new(latitude: Of32, longitude: Of32) -> Self {
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

impl Into<geo::Coord> for Point {
    fn into(self) -> geo::Coord {
        geo::coord! { x: self.longitude.0 as f64, y: self.latitude.0 as f64 }
    }
}


pub(crate) struct Ring {
    // TODO: fn new があるのに、pointsが公開されていて、変。
    pub(crate) points: Vec<Point>,
}

impl Ring {
    // TODO: 型変換の責任をnewが持ってはならない
    pub(crate) fn new(points: &[shapefile::Point]) -> Self {
        let points = points.iter().map(|p| (*p).into()).collect_vec();
        Self {
            points,
        }
    }

    pub(crate) fn iter_adjacent_points(&self) -> AdjacentPointsIter {
        AdjacentPointsIter::new(&self.points)
    }

    pub(crate) fn triangulate(&self) -> Vec<&Point> {
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

pub(crate) struct AdjacentPointsIter<'a> {
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

impl<'a> Iterator for AdjacentPointsIter<'a> {
    type Item = AdjacentPointsIterItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.points.len() {
            None
        } else {
            // TODO: もう少しどうにかならんか…？
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

pub(crate) struct AdjacentPointsIterItem<'a> {
    pub(crate) previous: &'a Point,
    pub(crate) current: &'a Point,
    pub(crate) next: &'a Point,
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


#[derive(Debug)]
pub(crate) struct Line<'a> {
    // TODO: newがあるのに直接参照していて変。
    pub(crate) vertices: Vec<&'a Point>,
}

impl<'a> Line<'a> {
    // TODO: Vecをnewが食うな。 (総じて、実はnewを取っ払うべき説)
    pub(crate) fn new(vertices: Vec<&'a Point>) -> Self {
        Self {
            vertices,
        }
    }

    pub(crate) fn join_first(&mut self, mut other: Self) {
        if other.vertices.last() == self.vertices.first() {
            other.vertices.pop();
        }
        other.vertices.append(&mut self.vertices);
        self.vertices = other.vertices;
    }

    // TODO: 逆向きの依存
    pub(crate) fn pref_reference_count(&self, references: &crate::parse_shapefile::PointReferences) -> usize {
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

impl Into<geo::LineString> for &Line<'_> {
    fn into(self) -> geo::LineString {
        geo::LineString::new(self.vertices.iter().map(|v| (**v).into()).collect_vec())
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