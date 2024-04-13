pub mod api;
pub mod map;

use std::collections::HashMap;
use std::net::SocketAddr;

use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    RequestPartsExt, Router,
};
use clap::Parser;
use once_cell::sync::OnceCell;

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(long, env)]
    pub listen: SocketAddr,

    #[clap(long, env)]
    pub key: String,
}

static KEY: OnceCell<Vec<u8>> = OnceCell::new();

async fn handler() -> Html<&'static str> {
    Html("<h1>EEW Renderer</h1>")
}

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
            _ =>  Err((StatusCode::NOT_FOUND, "Unknown API version").into_response()),
        }
    }
}

#[tokio::main]
async fn main() {
    let c = Cli::parse();
    KEY.set(c.key.into_bytes()).unwrap();

    let app = Router::new()
        .route("/", get(handler))
        .route("/api/:version/maps", get(api::maps))
        .route("/api/:version/map/:map_id", get(api::map))
        .route("/api/:version/draw", get(api::draw));

    let listener = tokio::net::TcpListener::bind(c.listen).await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
