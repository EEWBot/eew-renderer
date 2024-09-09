use crate::intensity::震度;

pub struct Area {
    pub code: u32,
    pub intensity: 震度,
}

impl Area {
    const fn new(code: u32, intensity: 震度) -> Self {
        Self { code, intensity }
    }
}
