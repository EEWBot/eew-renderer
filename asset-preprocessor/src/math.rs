use itertools::Itertools;
use renderer_types::{GeoDegree, Vertex};
use std::hash::{Hash, Hasher};

pub(crate) type Of32 = ordered_float::OrderedFloat<f32>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) struct Point {
    pub(crate) latitude: Of32,
    pub(crate) longitude: Of32,
}

impl Point {
    pub(crate) fn new(latitude: Of32, longitude: Of32) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    pub(crate) fn multiply_by(&self, scale: f32) -> Self {
        Self {
            latitude: self.latitude * scale,
            longitude: self.longitude * scale,
        }
    }

    pub(crate) fn divide_by(&self, scale: f32) -> Self {
        Self {
            latitude: self.latitude / scale,
            longitude: self.longitude / scale,
        }
    }
}

impl std::ops::Add for Point {
    type Output = Point;

    fn add(self, other: Point) ->  Self::Output {
        Self::Output {
            latitude: self.latitude + other.latitude,
            longitude: self.longitude + other.longitude,
        }
    }
}


impl From<&geo::Point> for Point {
    fn from(value: &geo::Point) -> Self {
        Self::new(Of32::from(value.x() as f32), Of32::from(value.y() as f32))
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

impl From<Point> for (Of32, Of32) {
    fn from(val: Point) -> Self {
        (val.longitude, val.latitude)
    }
}

impl From<&Point> for geo::Coord {
    fn from(val: &Point) -> Self {
        geo::coord! { x: val.longitude.0 as f64, y: val.latitude.0 as f64 }
    }
}

pub(crate) struct Ring {
    points: Vec<Point>,
}

impl Ring {
    pub(crate) fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    pub(crate) fn points(&self) -> &[Point] {
        &self.points
    }

    pub(crate) fn iter_adjacent_points(&self) -> AdjacentPointsIter<'_> {
        AdjacentPointsIter::new(&self.points)
    }

    pub(crate) fn triangulate(&self) -> Vec<Point> {
        earcutr::earcut(
            &self
                .points
                .iter()
                .flat_map(|p| [p.longitude.0, p.latitude.0])
                .collect_vec(),
            &[],
            2,
        )
        .unwrap()
        .iter()
        .map(|i| self.points[*i])
        .collect()
    }
}

impl From<Vec<shapefile::Point>> for Ring {
    fn from(value: Vec<shapefile::Point>) -> Self {
        let points = value.into_iter().map(|p| p.into()).collect();
        Self::new(points)
    }
}

pub(crate) struct AdjacentPointsIter<'a> {
    points: &'a [Point],
    index: usize,
}

impl<'a> AdjacentPointsIter<'a> {
    fn new(points: &'a [Point]) -> Self {
        Self { points, index: 0 }
    }
}

impl Iterator for AdjacentPointsIter<'_> {
    type Item = AdjacentPointsIterItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.points.len() {
            None
        } else {
            let previous_index = (self.points.len() - 1 + self.index) % self.points.len();
            let current_index = self.index;
            let next_index = (self.index + 1) % self.points.len();

            self.index += 1;

            Some(Self::Item::new(
                self.points[previous_index],
                self.points[current_index],
                self.points[next_index],
            ))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.points.len() - self.index;
        (size, Some(size))
    }
}

impl ExactSizeIterator for AdjacentPointsIter<'_> {}

pub(crate) struct AdjacentPointsIterItem {
    pub(crate) previous: Point,
    pub(crate) current: Point,
    pub(crate) next: Point,
}

impl AdjacentPointsIterItem {
    fn new(previous: Point, current: Point, next: Point) -> Self {
        Self {
            previous,
            current,
            next,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Line {
    pub(crate) vertices: Vec<Point>,
}

impl Line {
    pub(crate) fn new(points: &[Point]) -> Self {
        Self {
            vertices: points.to_owned(),
        }
    }

    pub(crate) fn join_first(&mut self, mut other: Self) {
        if other.vertices.last() == self.vertices.first() {
            other.vertices.pop();
        }
        other.vertices.append(&mut self.vertices);
        self.vertices = other.vertices;
    }
}

impl From<Vec<shapefile::Point>> for Line {
    fn from(value: Vec<shapefile::Point>) -> Self {
        let points: Vec<_> = value.into_iter().map(|p| p.into()).collect();
        Self::new(&points)
    }
}

impl From<&Line> for geo::LineString {
    fn from(val: &Line) -> Self {
        geo::LineString::new(val.vertices.iter().map(|v| v.into()).collect())
    }
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        let (is_self_ordered, is_other_hand_ordered) = (
            self.vertices.first().unwrap() < self.vertices.last().unwrap(),
            other.vertices.first().unwrap() < other.vertices.last().unwrap(),
        );

        if is_self_ordered == is_other_hand_ordered {
            itertools::equal(&self.vertices, &other.vertices)
        } else {
            itertools::equal(self.vertices.iter().rev(), &other.vertices)
        }
    }
}

impl Eq for Line {}

impl Hash for Line {
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

#[cfg(test)]
mod tests {
    use crate::math::*;

    #[test]
    fn line_equal_to_self() {
        let line = Line::new(&[
            Point::new(Of32::from(0.0), Of32::from(0.0)),
            Point::new(Of32::from(1.0), Of32::from(1.0)),
            Point::new(Of32::from(2.0), Of32::from(2.0)),
            Point::new(Of32::from(3.0), Of32::from(3.0)),
        ]);

        assert_eq!(line, line);
    }

    #[test]
    fn line_equal_to_reversed() {
        let line1 = Line::new(&[
            Point::new(Of32::from(0.0), Of32::from(0.0)),
            Point::new(Of32::from(1.0), Of32::from(1.0)),
            Point::new(Of32::from(2.0), Of32::from(2.0)),
            Point::new(Of32::from(3.0), Of32::from(3.0)),
        ]);

        let line2 = Line::new(&[
            Point::new(Of32::from(3.0), Of32::from(3.0)),
            Point::new(Of32::from(2.0), Of32::from(2.0)),
            Point::new(Of32::from(1.0), Of32::from(1.0)),
            Point::new(Of32::from(0.0), Of32::from(0.0)),
        ]);

        assert_eq!(line1, line2);
    }
}
