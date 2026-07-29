#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsetRegion {
    Main,
    Okinawa,
    Ogasawara,
}

impl InsetRegion {
    pub const COUNT: usize = 3;
    pub const ALL: [Self; Self::COUNT] = [Self::Main, Self::Okinawa, Self::Ogasawara];

    pub const fn index(self) -> usize {
        match self {
            Self::Main => 0,
            Self::Okinawa => 1,
            Self::Ogasawara => 2,
        }
    }
}

/// 772: 奄美群島・トカラ列島 / 800: 沖縄本島地方 / 801: 大東島地方 / 802: 宮古島・八重山地方
pub const TSUNAMI_AREA_CODES_OKINAWA: &[u32] = &[772, 800, 801, 802];

/// 321: 小笠原諸島
pub const TSUNAMI_AREA_CODES_OGASAWARA: &[u32] = &[321];

/// 774: 鹿児島県十島村 / 778: 鹿児島県奄美北部 / 779: 鹿児島県奄美南部
/// 800: 沖縄県本島北部 / 801: 沖縄県本島中南部 / 802: 沖縄県久米島 / 803: 沖縄県大東島
/// 804: 沖縄県宮古島 / 805: 沖縄県石垣島 / 806: 沖縄県与那国島 / 807: 沖縄県西表島
pub const SAIBUNKUIKI_CODES_OKINAWA: &[u32] =
    &[774, 778, 779, 800, 801, 802, 803, 804, 805, 806, 807];

/// 359: 東京都小笠原
pub const SAIBUNKUIKI_CODES_OGASAWARA: &[u32] = &[359];

pub fn classify_tsunami_area(code: u32) -> InsetRegion {
    if TSUNAMI_AREA_CODES_OKINAWA.contains(&code) {
        InsetRegion::Okinawa
    } else if TSUNAMI_AREA_CODES_OGASAWARA.contains(&code) {
        InsetRegion::Ogasawara
    } else {
        InsetRegion::Main
    }
}

pub fn classify_saibunkuiki(code: u32) -> InsetRegion {
    if SAIBUNKUIKI_CODES_OKINAWA.contains(&code) {
        InsetRegion::Okinawa
    } else if SAIBUNKUIKI_CODES_OGASAWARA.contains(&code) {
        InsetRegion::Ogasawara
    } else {
        InsetRegion::Main
    }
}

pub type ViewBBox = ((f32, f32), (f32, f32));

/// ((与那国島西, 波照間島南), (北大東島東, 口之島北))
pub const OKINAWA_VIEW_BBOX: ViewBBox = ((122.5, 23.9), (131.9, 30.1));

/// ((西ノ島西, 南硫黄島南), (東, 聟島列島北))
pub const OGASAWARA_VIEW_BBOX: ViewBBox = ((140.2, 23.9), (142.5, 27.9));
