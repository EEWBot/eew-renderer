pub mod api;
pub mod map;

use std::net::SocketAddr;
use axum::{response::Html, routing::get, Router};
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

#[tokio::main]
async fn main() {
    let c = Cli::parse();
    KEY.set(c.key.into_bytes()).unwrap();

    let app = Router::new()
        .route("/", get(handler))
        .route("/api/v1/maps", get(api::v1::maps))
        .route("/api/v1/map/:map_id", get(api::v1::map))
        .route("/api/v1/draw", get(api::v1::draw));

    let listener = tokio::net::TcpListener::bind(c.listen).await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
