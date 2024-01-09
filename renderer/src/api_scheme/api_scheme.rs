use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct EarthquakeSubdivisionArea {
    epicenter: Epicenter,
    intensity_map: Vec<IntensityMapEntry>,
}

#[derive(Deserialize, Debug)]
struct Epicenter {
    latitude: f32,
    longitude: f32,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "intensity", content = "area_ids")]
enum IntensityMapEntry {
    #[serde(rename = "1")]
    One(Vec<u32>),
    #[serde(rename = "2")]
    Two(Vec<u32>),
    #[serde(rename = "3")]
    Three(Vec<u32>),
    #[serde(rename = "4")]
    Four(Vec<u32>),
    #[serde(rename = "5-")]
    FiveMinus(Vec<u32>),
    #[serde(rename = "5+")]
    FivePlus(Vec<u32>),
    #[serde(rename = "6-")]
    SixMinus(Vec<u32>),
    #[serde(rename = "6+")]
    SixPlus(Vec<u32>),
    #[serde(rename = "7")]
    Seven(Vec<u32>),
}
