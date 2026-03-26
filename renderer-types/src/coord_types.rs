use std::fmt::Debug;

pub trait CoordType: Clone + Copy + Eq + PartialEq + Debug {}

/// 度数法での経緯度
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct GeoDegree;
impl CoordType for GeoDegree {}

/// 弧度法での経緯度
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct GeoRadian;
impl CoordType for GeoRadian {}

/// 経緯度に楕円体補正をしてメルカトル図法に直した座標
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Mercator;
impl CoordType for Mercator {}

/// 左下原点のピクセル空間
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Pixel;
impl CoordType for Pixel {}

/// 中央原点のスクリーン空間
/// 標示範囲は-1.0 <= x,y <= 1.0
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Screen;
impl CoordType for Screen {}
