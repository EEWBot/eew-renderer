use crate::{GeoDegree, Screen};
use std::f32::consts::PI;
use std::marker::PhantomData;

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct Vertex<Type> {
    pub x: f32,
    pub y: f32,
    pub _type: PhantomData<Type>,
}

impl<Type> Vertex<Type> {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            _type: PhantomData,
        }
    }

    pub fn euclidean_distance<T: Into<Self>>(&self, other: T) -> f32 {
        let other: Self = other.into();
        let x_dist = self.x - other.x;
        let y_dist = self.y - other.y;
        f32::sqrt((x_dist * x_dist) + (y_dist * y_dist))
    }

    pub fn to_slice(&self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl Vertex<GeoDegree> {
    pub fn to_screen(&self) -> Vertex<Screen> {
        const E: f32 = 0.081819191042815791;
        let radianized_x = self.x.to_radians();
        let radianized_y = self.y.to_radians();
        let x = radianized_x / PI;
        let y = (radianized_y.sin().atanh() - E * (E * radianized_y.sin()).atanh()) / PI;
        Vertex::new(x, y)
    }
}

impl<Type> std::ops::Neg for Vertex<Type> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Vertex {
            x: -self.x,
            y: -self.y,
            _type: PhantomData,
        }
    }
}

impl<Type> From<(f64, f64)> for Vertex<Type> {
    fn from(value: (f64, f64)) -> Vertex<Type> {
        Self {
            x: value.0 as f32,
            y: value.1 as f32,
            _type: PhantomData,
        }
    }
}

impl<Type> From<(f32, f32)> for Vertex<Type> {
    fn from(value: (f32, f32)) -> Vertex<Type> {
        Self {
            x: value.0,
            y: value.1,
            _type: PhantomData,
        }
    }
}

#[cfg(feature = "shapefile")]
impl From<shapefile::Point> for Vertex<crate::GeoDegree> {
    fn from(value: shapefile::Point) -> Self {
        Self {
            x: value.x as f32,
            y: value.y as f32,
            _type: PhantomData,
        }
    }
}
