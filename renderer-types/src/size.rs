#[derive(PartialEq, Copy, Clone, Debug)]
pub struct Size {
    pub x: f32,
    pub y: f32,
}

impl std::ops::Sub for Size {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self::Output {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Size {
    #[must_use]
    pub fn from_tuple((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn to_i(&self) -> SizeU {
        SizeU {
            x: self.x as u32,
            y: self.y as u32,
        }
    }

    #[must_use]
    pub fn scale(&self, scaler: f32) -> Self {
        Self {
            x: self.x * scaler,
            y: self.y * scaler,
        }
    }

    #[must_use]
    pub fn fit_scale(&self, other: &Self) -> f32 {
        f32::min(other.x / self.x, other.y / self.y)
    }

    #[must_use]
    pub fn fit(&self, other: &Self) -> Self {
        let scale = self.fit_scale(&other);
        self.scale(scale)
    }

    #[must_use]
    pub fn capped_fit(&self, other: &Self) -> Self {
        let scale = f32::min(self.fit_scale(&other), 1.0);
        self.scale(scale)
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct SizeU {
    pub x: u32,
    pub y: u32,
}

impl SizeU {
    #[must_use]
    pub fn from_tuple((x, y): (u32, u32)) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn to_f(&self) -> Size {
        Size {
            x: self.x as f32,
            y: self.y as f32,
        }
    }
}
