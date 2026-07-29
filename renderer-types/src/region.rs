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

fn bbox_contains(bbox: &ViewBBox, lon: f32, lat: f32) -> bool {
    let ((min_lon, min_lat), (max_lon, max_lat)) = *bbox;
    (min_lon..=max_lon).contains(&lon) && (min_lat..=max_lat).contains(&lat)
}

/// 震央の振り分けには使わない (renderer 側でカメラの可視範囲で判定)
pub fn classify_coordinate(lon: f32, lat: f32) -> InsetRegion {
    if bbox_contains(&OKINAWA_VIEW_BBOX, lon, lat) {
        InsetRegion::Okinawa
    } else if bbox_contains(&OGASAWARA_VIEW_BBOX, lon, lat) {
        InsetRegion::Ogasawara
    } else {
        InsetRegion::Main
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsunami_area_classification() {
        assert_eq!(classify_tsunami_area(772), InsetRegion::Okinawa); // 奄美群島・トカラ列島
        assert_eq!(classify_tsunami_area(800), InsetRegion::Okinawa); // 沖縄本島地方
        assert_eq!(classify_tsunami_area(801), InsetRegion::Okinawa); // 大東島地方
        assert_eq!(classify_tsunami_area(802), InsetRegion::Okinawa); // 宮古島・八重山地方
        assert_eq!(classify_tsunami_area(321), InsetRegion::Ogasawara); // 小笠原諸島
        assert_eq!(classify_tsunami_area(771), InsetRegion::Main); // 鹿児島県東部
        assert_eq!(classify_tsunami_area(320), InsetRegion::Main); // 伊豆諸島
        assert_eq!(classify_tsunami_area(100), InsetRegion::Main);
    }

    #[test]
    fn saibunkuiki_classification() {
        assert_eq!(classify_saibunkuiki(774), InsetRegion::Okinawa); // 十島村
        assert_eq!(classify_saibunkuiki(778), InsetRegion::Okinawa); // 奄美北部
        assert_eq!(classify_saibunkuiki(779), InsetRegion::Okinawa); // 奄美南部
        assert_eq!(classify_saibunkuiki(800), InsetRegion::Okinawa); // 沖縄県本島北部
        assert_eq!(classify_saibunkuiki(801), InsetRegion::Okinawa); // 沖縄県本島中南部
        assert_eq!(classify_saibunkuiki(802), InsetRegion::Okinawa); // 沖縄県久米島
        assert_eq!(classify_saibunkuiki(803), InsetRegion::Okinawa); // 沖縄県大東島
        assert_eq!(classify_saibunkuiki(804), InsetRegion::Okinawa); // 沖縄県宮古島
        assert_eq!(classify_saibunkuiki(805), InsetRegion::Okinawa); // 沖縄県石垣島
        assert_eq!(classify_saibunkuiki(806), InsetRegion::Okinawa); // 沖縄県与那国島
        assert_eq!(classify_saibunkuiki(807), InsetRegion::Okinawa); // 沖縄県西表島
        assert_eq!(classify_saibunkuiki(359), InsetRegion::Ogasawara); // 小笠原
        assert_eq!(classify_saibunkuiki(775), InsetRegion::Main); // 甑島
        assert_eq!(classify_saibunkuiki(776), InsetRegion::Main); // 種子島
        assert_eq!(classify_saibunkuiki(777), InsetRegion::Main); // 屋久島
        assert_eq!(classify_saibunkuiki(358), InsetRegion::Main); // 八丈島
        assert_eq!(classify_saibunkuiki(65535), InsetRegion::Main); // UNNUMBERED
    }

    #[test]
    fn coordinate_classification() {
        // 沖縄本島 那覇
        assert_eq!(classify_coordinate(127.68, 26.21), InsetRegion::Okinawa);
        // 口之島 (774 に属するためインセット側、矩形にも含まれる)
        assert_eq!(classify_coordinate(129.93, 29.97), InsetRegion::Okinawa);
        // 南大東島
        assert_eq!(classify_coordinate(131.23, 25.83), InsetRegion::Okinawa);
        // 与那国島
        assert_eq!(classify_coordinate(122.94, 24.45), InsetRegion::Okinawa);
        // 父島
        assert_eq!(classify_coordinate(142.19, 27.09), InsetRegion::Ogasawara);
        // 硫黄島
        assert_eq!(classify_coordinate(141.32, 24.78), InsetRegion::Ogasawara);
        // 西之島
        assert_eq!(classify_coordinate(140.88, 27.25), InsetRegion::Ogasawara);
        // 屋久島 (本土側)
        assert_eq!(classify_coordinate(130.53, 30.34), InsetRegion::Main);
        // 八丈島 (本土側)
        assert_eq!(classify_coordinate(139.79, 33.11), InsetRegion::Main);
        // 東京湾
        assert_eq!(classify_coordinate(139.9, 35.4), InsetRegion::Main);
    }
}
