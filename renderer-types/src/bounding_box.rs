use std::ops::Div;
use num_traits::{Bounded, Float, Zero};
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

    pub fn extends_with(&self, other: &Self) -> Self
    where
        Type::InnerType: Ord
    {
        Self {
            min: Vertex::new(
                Ord::min(self.min.x(), other.min.x()),
                Ord::min(self.min.y(), other.min.y()),
            ),
            max: Vertex::new(
                Ord::max(self.max.x(), other.max.x()),
                Ord::max(self.max.x(), other.max.x()),
            ),
        }
    }

    pub fn extends_with_float(&self, other: &Self) -> Self
    where
        Type::InnerType: Float
    {
        Self {
            min: Vertex::new(
                Float::min(self.min.x(), other.min.x()),
                Float::min(self.min.y(), other.min.y()),
            ),
            max: Vertex::new(
                Float::max(self.max.x(), other.max.x()),
                Float::max(self.max.y(), other.max.y()),
            )
        }
    }

    pub fn extends_by_vertex(&self, vertex: &Vertex<Type>) -> Self
    where
        Type::InnerType: Ord
    {
        Self {
            min: Vertex::new(
                Ord::min(self.min.x(), vertex.x()),
                Ord::min(self.min.y(), vertex.y()),
            ),
            max: Vertex::new(
                Ord::max(self.max.x(), vertex.x()),
                Ord::max(self.max.y(), vertex.y()),
            ),
        }
    }

    pub fn extends_by_vertex_float(&self, vertex: &Vertex<Type>) -> Self
    where
        Type::InnerType: Float
    {
        Self {
            min: Vertex::new(
                Float::min(self.min.x(), vertex.x()),
                Float::min(self.min.y(), vertex.y()),
            ),
            max: Vertex::new(
                Float::max(self.max.x(), vertex.x()),
                Float::max(self.max.y(), vertex.y()),
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

    pub fn size(&self) -> Size<Type::InnerType> {
        Size::from_tuple((
            self.max.x() - self.min.x(),
            self.max.y() - self.min.y(),
        ))
    }

    /// まって、これ原点またいだとき、どうなるの？
    pub fn center(&self) -> Vertex<Type>
    where
        Type::InnerType: Div<f32, Output = Type::InnerType>
    {
        Vertex::new(
            (self.min.x() + self.max.x()) / 2.0,
            (self.min.y() + self.max.y()) / 2.0,
        )
    }

    pub const fn to_tuple(&self) -> (Type::InnerType, Type::InnerType, Type::InnerType, Type::InnerType) {
        (self.min.x(), self.min.y(), self.max.x(), self.max.y())
    }

    pub const fn from_tuple<T>(v: (Type::InnerType, Type::InnerType, Type::InnerType, Type::InnerType)) -> BoundingBox<Type> {
        Self {
            min: Vertex::new(v.0, v.1),
            max: Vertex::new(v.2, v.3),
        }
    }

    pub fn from_vertices(vertices: &[Vertex<Type>]) -> BoundingBox<Type>
    where
        Type::InnerType: Ord + Zero + Bounded
    {
        if vertices.is_empty() {
            Self {
                min: Vertex::new(Type::InnerType::zero(), Type::InnerType::zero()),
                max: Vertex::new(Type::InnerType::zero(), Type::InnerType::zero()),
            }
        } else {
            vertices.iter().fold(
                BoundingBox {
                    min: Vertex::new(Type::InnerType::max_value(), Type::InnerType::max_value()),
                    max: Vertex::new(Type::InnerType::min_value(), Type::InnerType::min_value()),
                },
                |acc, vertex| acc.extends_by_vertex(vertex),
            )
        }
    }

    pub fn from_vertices_float(vertices: &[Vertex<Type>]) -> BoundingBox<Type>
    where
        Type::InnerType: Float
    {
        if vertices.is_empty() {
            Self {
                min: Vertex::new(Type::InnerType::zero(), Type::InnerType::zero()),
                max: Vertex::new(Type::InnerType::zero(), Type::InnerType::zero()),
            }
        } else {
            vertices.iter().fold(
                BoundingBox {
                    min: Vertex::new(Type::InnerType::max_value(), Type::InnerType::max_value()),
                    max: Vertex::new(Type::InnerType::min_value(), Type::InnerType::min_value()),
                },
                |acc, vertex| acc.extends_by_vertex_float(vertex),
            )
        }
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
