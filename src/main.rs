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
        .route("/api/:version/maps", get(api::maps))
        .route("/api/:version/map/:map_id", get(api::map))
        .route("/api/:version/draw", get(api::draw));

    let listener = tokio::net::TcpListener::bind(c.listen).await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
