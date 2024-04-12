use serde::Deserialize;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use axum::{
    extract::{Path, Query},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct DrawQuery {
    #[serde(rename = "c")]
    pub content: String,

    #[serde(rename = "s")]
    pub sign: String,
}

pub async fn draw(query: Query<DrawQuery>) -> impl IntoResponse {
    let key = crate::KEY.get().unwrap();

    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(query.content.as_bytes());
    let s = format!("{:02x}", mac.finalize().into_bytes());

    if s != query.sign {
        println!("{s} {}", query.sign);
        return (StatusCode::BAD_REQUEST, "Invalid Signature");
    }

    println!("{query:?}");

    (StatusCode::SERVICE_UNAVAILABLE, "")
}

pub async fn maps() -> impl IntoResponse {
    Json(crate::map::map_names())
}

pub async fn map(Path(map_id): Path<String>) -> impl IntoResponse {
    let Some(map) = crate::map::query_map(&map_id) else {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
            "NOT FOUND",
        );
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        map,
    )
}
