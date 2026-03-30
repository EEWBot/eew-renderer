use std::ops::Div;
use num_traits::{Bounded, Float, Zero};
use crate::{CoordType, Size, Vertex};

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct BoundingBox<Type: CoordType> {
    pub min: Vertex<Type>,
    pub max: Vertex<Type>,
}

impl<Type: CoordType> BoundingBox<Type> {
    pub const fn new(min: Vertex<Type>, max: Vertex<Type>) -> Self {
        Self { min, max }
    }

    pub const fn first_quadrant(&self) -> Vertex<Type> {
        self.max
    }

    pub const fn second_quadrant(&self) -> Vertex<Type> {
        Vertex::new(self.min.x(), self.max.y())
    }

    pub const fn third_quadrant(&self) -> Vertex<Type> {
        self.min
    }

    pub const fn fourth_quadrant(&self) -> Vertex<Type> {
        Vertex::new(self.max.x(), self.min.y())
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

    pub fn merge(&self, other: &Self) -> Self
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

    pub fn merge_float(&self, other: &Self) -> Self
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

    pub fn encapsulate(&self, vertex: &Vertex<Type>) -> Self
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

    pub fn encapsulate_float(&self, vertex: &Vertex<Type>) -> Self
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
            self.third_quadrant(),
            self.fourth_quadrant(),
            self.second_quadrant(),
            self.first_quadrant(),
        ]
    }

    pub fn size(&self) -> Size<Type::InnerType> {
        Size::new(
            self.max.x() - self.min.x(),
            self.max.y() - self.min.y(),
        )
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
                |acc, vertex| acc.encapsulate(vertex),
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
                |acc, vertex| acc.encapsulate_float(vertex),
            )
        }
    }
}

impl<Type: CoordType> Eq for BoundingBox<Type> where Vertex<Type>: Eq {}

impl<Type: CoordType> From<(Vertex<Type>, Vertex<Type>)> for BoundingBox<Type> {
    fn from((min, max): (Vertex<Type>, Vertex<Type>)) -> Self {
        Self { min, max }
    }
}

impl<Type: CoordType> From<[Vertex<Type>; 2]> for BoundingBox<Type> {
    fn from([min, max]: [Vertex<Type>; 2]) -> Self {
        Self { min, max }
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
