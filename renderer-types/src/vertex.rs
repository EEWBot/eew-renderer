use crate::{CoordType, GeoDegree, GeoRadian, Mercator, Pixel, Screen, Size};
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
    pub fn to_screen(&self, dimension: Size<u32>) -> Vertex<Screen> {
        let dimension = dimension.to_f32();
        Vertex::new(
            f32::mul_add((self.x + 0.5) / dimension.x(), 2.0, -1.0),
            f32::mul_add((self.y + 0.5) / dimension.y(), 2.0, -1.0),
        )
    }
}

impl Vertex<Screen> {
    pub fn to_pixel(&self, dimension: Size<u32>) -> Vertex<Pixel> {
        let half_dim = dimension.to_f32();
        let half_dim = (half_dim.x() * 0.5, half_dim.y() * 0.5);
        Vertex::new(
            f32::mul_add(self.x, half_dim.0, half_dim.0).floor(),
            f32::mul_add(self.y, half_dim.1, half_dim.1).floor(),
        )
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

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use rstest_reuse::{apply, template};
    use crate::{Pixel, Screen, Size, Vertex};

    #[template]
    #[rstest]
    #[case(Vertex::new(-1.0, -1.0), Vertex::new(-1.0 - 1.0 / 128.0, -1.0 - 1.0 / 256.0))]
    #[case(Vertex::new(0.0, 0.0), Vertex::new(-1.0 + 1.0 / 128.0, -1.0 + 1.0 / 256.0))]
    #[case(Vertex::new(63.0, 127.0), Vertex::new(0.0 - 1.0 / 128.0, 0.0 - 1.0 / 256.0))]
    #[case(Vertex::new(64.0, 128.0), Vertex::new(0.0 + 1.0 / 128.0, 0.0 + 1.0 / 256.0))]
    #[case(Vertex::new(127.0, 255.0), Vertex::new(1.0 - 1.0 / 128.0, 1.0 - 1.0 / 256.0))]
    #[case(Vertex::new(128.0, 256.0), Vertex::new(1.0 + 1.0 / 128.0, 1.0 + 1.0 / 256.0))]
    fn pixel_screen_cases(
        #[values(Size::from_tuple((128, 256)))] dimension: Size<u32>,
        #[case] pixel: Vertex<Pixel>,
        #[case] screen: Vertex<Screen>,
    ) {}

    #[apply(pixel_screen_cases)]
    fn pixel_to_screen(dimension: Size<u32>, pixel: Vertex<Pixel>, screen: Vertex<Screen>) {
        assert_eq!(pixel.to_screen(dimension), screen);
    }

    #[apply(pixel_screen_cases)]
    fn screen_to_pixel(dimension: Size<u32>, pixel: Vertex<Pixel>, screen: Vertex<Screen>) {
        assert_eq!(screen.to_pixel(dimension), pixel);
    }
}
