use crate::{CoordType, Size, Vertex};

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox<Type: CoordType> {
    pub min: Vertex<Type>,
    pub max: Vertex<Type>,
}

impl<Type: CoordType> BoundingBox<Type> {
    pub const fn value_top_left(&self) -> Vertex<Type> {
        Vertex::new(self.min.x(), self.min.y())
    }

    pub const fn value_top_right(&self) -> Vertex<Type> {
        Vertex::new(self.max.x(), self.min.y())
    }

    pub const fn value_bottom_left(&self) -> Vertex<Type> {
        Vertex::new(self.min.x(), self.max.y())
    }

    pub const fn value_bottom_right(&self) -> Vertex<Type> {
        Vertex::new(self.max.x(), self.max.y())
    }

    pub const fn gl_top_left(&self) -> Vertex<Type> {
        Vertex::new(self.max.x(), self.min.y())
    }

    pub const fn gl_top_right(&self) -> Vertex<Type> {
        Vertex::new(self.max.x(), self.max.y())
    }

    pub const fn gl_bottom_left(&self) -> Vertex<Type> {
        Vertex::new(self.min.x(), self.min.y())
    }

    pub const fn gl_bottom_right(&self) -> Vertex<Type> {
        Vertex::new(self.min.x(), self.max.y())
    }

    pub const fn extends_with(&self, other: &Self) -> Self {
        Self {
            min: Vertex::new(
                f32::min(other.min.x(), self.min.x()),
                f32::min(other.min.y(), self.min.y()),
            ),
            max: Vertex::new(
                f32::max(other.max.x(), self.max.x()),
                f32::max(other.max.y(), self.max.y()),
            ),
        }
    }

    pub const fn extends_by_vertex(&self, vertex: &Vertex<Type>) -> Self {
        Self {
            min: Vertex::new(
                f32::min(self.min.x(), vertex.x()),
                f32::min(self.min.y(), vertex.y()),
            ),
            max: Vertex::new(
                f32::max(self.max.x(), vertex.x()),
                f32::max(self.max.y(), vertex.y()),
            ),
        }
    }

    pub const fn gl_vertices(&self) -> [Vertex<Type>; 4] {
        [
            self.gl_bottom_left(),
            self.gl_bottom_right(),
            self.gl_top_right(),
            self.gl_top_left(),
        ]
    }

    pub const fn size(&self) -> Size {
        Size {
            x: self.max.x() - self.min.x(),
            y: self.max.y() - self.min.y(),
        }
    }

    /// まって、これ原点またいだとき、どうなるの？
    pub const fn center(&self) -> Vertex<Type> {
        Vertex::new(
            (self.min.x() + self.max.x()) / 2.0,
            (self.min.y() + self.max.y()) / 2.0,
        )
    }

    pub const fn to_tuple(&self) -> (f32, f32, f32, f32) {
        (self.min.x(), self.min.y(), self.max.x(), self.max.y())
    }

    pub const fn from_tuple<T>(v: (f32, f32, f32, f32)) -> BoundingBox<Type> {
        Self {
            min: Vertex::new(v.0, v.1),
            max: Vertex::new(v.2, v.3),
        }
    }

    pub fn from_vertices(vertices: &[Vertex<Type>]) -> BoundingBox<Type> {
        vertices.iter().fold(
            BoundingBox {
                min: Vertex::new(f32::MAX, f32::MAX),
                max: Vertex::new(f32::MIN, f32::MIN),
            },
            |acc, vertex| acc.extends_by_vertex(vertex),
        )
    }
}

#[cfg(feature = "shapefile")]
use shapefile::{record::GenericBBox, Point};

#[cfg(feature = "shapefile")]
impl From<GenericBBox<Point>> for BoundingBox<crate::GeoDegree> {
    fn from(value: GenericBBox<Point>) -> Self {
        Self {
            min: value.min.into(),
            max: value.max.into(),
        }
    }
}
