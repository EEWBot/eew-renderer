use std::collections::HashMap;

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json, RequestPartsExt,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug)]
pub enum Version {
    V1,
}

#[async_trait]
impl<S> FromRequestParts<S> for Version
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let params: Path<HashMap<String, String>> =
            parts.extract().await.map_err(IntoResponse::into_response)?;

        let version = params
            .get("version")
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Version param missing").into_response())?;

        match version.as_str() {
            "v1" => Ok(Version::V1),
            _ => Err((StatusCode::NOT_FOUND, "Unknown API version").into_response()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DrawQuery {
    #[serde(rename = "c")]
    pub content: String,

    #[serde(rename = "s")]
    pub sign: String,
}

pub async fn draw(_v: Version, query: Query<DrawQuery>) -> impl IntoResponse {
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

pub async fn maps(_v: Version) -> impl IntoResponse {
    Json(crate::map::map_names())
}

pub async fn map(_v: Version, Path((_, map_id)): Path<(String, String)>) -> impl IntoResponse {
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
