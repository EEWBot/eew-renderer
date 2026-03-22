use crate::{CoordType, GeoDegree, GeoRadian, Mercator, Pixel, Screen, SizeU};
use std::f32::consts::PI;
use std::marker::PhantomData;

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct Vertex<Type: CoordType> {
    x: f32,
    y: f32,
    _type: PhantomData<Type>,
}

impl<Type: CoordType> Vertex<Type> {
    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            _type: PhantomData,
        }
    }

    pub fn euclidean_distance<T: Into<Self>>(&self, other: T) -> f32 {
        let other: Self = other.into();
        let distance = *self - other;
        f32::sqrt((distance.x * distance.x) + (distance.y * distance.y))
    }

    pub const fn x(&self) -> f32 {
        self.x
    }

    pub const fn y(&self) -> f32 {
        self.y
    }

    pub const fn to_tuple(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    pub const fn to_slice(&self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl Vertex<GeoDegree> {
    pub const fn to_geo_radian(&self) -> Vertex<GeoRadian> {
        let x = self.x.to_radians();
        let y = self.y.to_radians();
        Vertex::new(x, y)
    }

    pub fn to_mercator(&self) -> Vertex<Mercator> {
        self.to_geo_radian().to_mercator()
    }
}

impl Vertex<GeoRadian> {
    pub const fn to_geo_degree(&self) -> Vertex<GeoDegree> {
        let x = self.x.to_degrees();
        let y = self.y.to_degrees();
        Vertex::new(x, y)
    }

    pub fn to_mercator(&self) -> Vertex<Mercator> {
        const E: f32 = 0.081819191042815791;
        let x = self.x / PI;
        let y = (self.y.sin().atanh() - E * (E * self.y.sin()).atanh()) / PI;
        Vertex::new(x, y)
    }
}

impl Vertex<Mercator> {
    pub fn to_screen(&self, offset: Vertex<Mercator>, scale: f32) -> Vertex<Screen> {
        let vertex = (*self + offset) * scale;
        Vertex::new(vertex.x, vertex.y)
    }
}

impl Vertex<Pixel> {
    pub const fn to_screen(&self, dimension: SizeU) -> Vertex<Screen> {
        let dimension = dimension.to_f();
        let x = self.x / dimension.x;
        let y = self.y / dimension.y;
        Vertex::new(x, y)
    }
}

impl Vertex<Screen> {
    pub const fn to_pixel(&self, dimension: SizeU) -> Vertex<Pixel> {
        let dimension = dimension.to_f();
        let x = (self.x * dimension.x).round_ties_even();
        let y = (self.y * dimension.y).round_ties_even();
        Vertex::new(x, y)
    }
}

impl<Type: CoordType> std::ops::Add for Vertex<Type> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Vertex {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            _type: PhantomData,
        }
    }
}

impl<Type: CoordType> std::ops::Sub for Vertex<Type> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Vertex {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            _type: PhantomData,
        }
    }
}

impl<Type: CoordType> std::ops::Mul<f32> for Vertex<Type> {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Vertex {
            x: self.x * rhs,
            y: self.y * rhs,
            _type: PhantomData,
        }
    }
}

impl<Type: CoordType> std::ops::Div<f32> for Vertex<Type> {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Vertex {
            x: self.x / rhs,
            y: self.y / rhs,
            _type: PhantomData,
        }
    }
}

impl<Type: CoordType> std::ops::Neg for Vertex<Type> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Vertex {
            x: -self.x,
            y: -self.y,
            _type: PhantomData,
        }
    }
}

impl<Type: CoordType> From<(f64, f64)> for Vertex<Type> {
    fn from(value: (f64, f64)) -> Vertex<Type> {
        Self {
            x: value.0 as f32,
            y: value.1 as f32,
            _type: PhantomData,
        }
    }
}

impl<Type: CoordType> From<(f32, f32)> for Vertex<Type> {
    fn from(value: (f32, f32)) -> Vertex<Type> {
        Self {
            x: value.0,
            y: value.1,
            _type: PhantomData,
        }
    }
}

#[cfg(feature = "shapefile")]
impl From<shapefile::Point> for Vertex<GeoDegree> {
    fn from(value: shapefile::Point) -> Self {
        Self {
            x: value.x as f32,
            y: value.y as f32,
            _type: PhantomData,
        }
    }
}
