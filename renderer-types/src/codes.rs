#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 地震情報細分区域(pub u32);

impl 地震情報細分区域 {
    /// 諸外国や北方領土など
    pub const UNNUMBERED: Self = Self(65535);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 津波予報区(pub u32);

impl 津波予報区 {
    /// 極小の島など
    pub const 帰属未定: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 震度観測点(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct 地震情報都道府県等(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InternalTsunamiAreaCode(pub u16);
