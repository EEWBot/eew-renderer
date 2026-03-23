use num_traits::AsPrimitive;
use std::fmt::Debug;

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct Size<Type: PartialEq + Copy + Clone + Debug> {
    x: Type,
    y: Type,
}

impl<Type: PartialEq + Copy + Clone + Debug> Size<Type> {
    #[must_use]
    pub const fn from_tuple((x, y): (Type, Type)) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn from_array(input: [Type; 2]) -> Self {
        Self {
            x: input[0],
            y: input[1],
        }
    }

    #[must_use]
    pub const fn to_tuple(&self) -> (Type, Type) {
        (self.x, self.y)
    }

    #[must_use]
    pub const fn to_array(&self) -> [Type; 2] {
        [self.x, self.y]
    }

    #[must_use]
    pub fn to_i32(&self) -> Size<i32>
    where
        Type: AsPrimitive<i32>,
    {
        Size {
            x: self.x.as_(),
            y: self.y.as_(),
        }
    }

    #[must_use]
    pub fn to_u32(&self) -> Size<u32>
    where
        Type: AsPrimitive<u32>,
    {
        Size {
            x: self.x.as_(),
            y: self.y.as_(),
        }
    }

    #[must_use]
    pub fn to_f32(&self) -> Size<f32>
    where
        Type: AsPrimitive<f32>,
    {
        Size {
            x: self.x.as_(),
            y: self.y.as_(),
        }
    }

    #[must_use]
    pub const fn x(&self) -> Type {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> Type {
        self.y
    }

    #[must_use]
    pub fn aspect_ratio(&self) -> f32
    where
        Type: AsPrimitive<f32>,
    {
        let self_f32 = self.to_f32();
        self_f32.y / self_f32.x
    }

    /// selfに掛けるとotherにぴったり収まる倍率を計算し、返す。
    /// self.x > other.x || self.y > other.y -> <1.0
    /// self.x < other.x && self.y < other.y -> >1.0
    #[must_use]
    pub fn scale_fit_within_other(&self, other: &Self) -> f32
    where
        Type: AsPrimitive<f32>,
    {
        let self_f32 = self.to_f32();
        let other_f32 = other.to_f32();
        f32::min(other_f32.x / self_f32.x, other_f32.y / self_f32.y)
    }

    /// selfにscaleを掛けた結果を返す。
    #[must_use]
    pub fn scale(&self, scale: f32) -> Self
    where
        Type: AsPrimitive<f32>,
        f32: AsPrimitive<Type>,
    {
        let self_f32 = self.to_f32();
        Self {
            x: (self_f32.x * scale).as_(),
            y: (self_f32.y * scale).as_(),
        }
    }

    /// selfをotherにぴったり収まるよう拡大縮小し、結果を返す。
    #[must_use]
    pub fn fit_exactly_with_other(&self, other: &Self) -> Self
    where
        Type: AsPrimitive<f32>,
        f32: AsPrimitive<Type>,
    {
        let self_f32 = self.to_f32();
        let other_f32 = other.to_f32();
        let scale = self_f32.scale_fit_within_other(&other_f32);
        self.scale(scale)
    }

    /// selfがotherに収まる場合はそのまま、はみ出る場合は縮小し、結果を返す。
    #[must_use]
    pub fn downscale_to_fit(&self, other: &Self) -> Self
    where
        Type: AsPrimitive<f32>,
        f32: AsPrimitive<Type>,
    {
        let self_f32 = self.to_f32();
        let other_f32 = other.to_f32();
        let scale = self_f32.scale_fit_within_other(&other_f32);

        match scale {
            1.0.. => *self,
            _ => self.scale(scale),
        }
    }
}

impl<Type: PartialEq + Copy + Clone + Debug + std::ops::Add<Type, Output = Type>> std::ops::Add
    for Size<Type>
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<Type: PartialEq + Copy + Clone + Debug + std::ops::Sub<Type, Output = Type>> std::ops::Sub
    for Size<Type>
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<Type: Eq + PartialEq + Copy + Clone + Debug> Eq for Size<Type> {}
