use std::collections::HashMap;

use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;

static MAP_DIR: Dir = include_dir!("maps");
static MAPS: Lazy<HashMap<&'static str, Vec<String>>> = Lazy::new(|| {
    MAP_DIR
        .files()
        .map(|f| {
            (
                f.path().file_stem().unwrap().to_str().unwrap(),
                serde_json::from_str(f.contents_utf8().unwrap()).expect("Informed JSON"),
            )
        })
        .collect()
});

pub fn map_names() -> Vec<&'static str> {
    MAP_DIR
        .files()
        .map(|f| f.path().file_stem().unwrap().to_str().unwrap())
        .collect()
}

pub fn query_map(key: &str) -> Option<&'static str> {
    MAP_DIR
        .get_file(format!("{key}.json"))
        .map(|f| f.contents_utf8().unwrap())
}

pub fn demap(key: &str, n: u16) -> Option<&'static str> {
    MAPS.get(key)
        .map(|v| v.get(n as usize).map(|v| v.as_str()))
        .flatten()
}
