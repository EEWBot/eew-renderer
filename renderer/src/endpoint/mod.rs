use std::sync::Arc;

use axum::{
    extract::{State, Request},
    http::{header::CONTENT_TYPE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use hmac::{Hmac, Mac};
use prost::Message;

mod model;
use crate::model::*;

type HmacSha1 = Hmac<sha1::Sha1>;

#[derive(Clone, Debug)]
pub struct AppState {
    pub request_channel: tokio::sync::mpsc::Sender<UserEvent>,
    pub hmac_key: Arc<String>,
}

async fn render_handler(State(app): State<AppState>, req: Request) -> Response {
    let bin = &req.uri().path()[1..];

    let Ok(bin) = urlencoding::decode(bin) else {
        return (StatusCode::BAD_REQUEST, "Failed to UTF-8 parsing").into_response();
    };

    let Ok(bin) = base65536::decode(&bin, false) else {
        return (StatusCode::BAD_REQUEST, "Failed to Base65536 decoding").into_response();
    };

    println!("{bin:x?}");

    if bin.len() < 21 {
        return (StatusCode::BAD_REQUEST, "Minimum length is not satisfied").into_response();
    }

    let version = bin[0];
    let provided_sha1 = &bin[1..21];
    let body = &bin[21..];

    if version != 0 {
        return (StatusCode::BAD_REQUEST, "Unknown protocol v{version}").into_response();
    }

    let calculated_sha1 = {
        let mut mac = HmacSha1::new_from_slice(app.hmac_key.as_bytes()).unwrap();
        mac.update(body);
        mac.finalize().into_bytes()
    };

    if calculated_sha1.as_slice() != provided_sha1 {
        return (StatusCode::UNAUTHORIZED, "Invalid HMAC").into_response();
    }

    let Ok(decoded) = crate::quake_prefecture::QuakePrefectureData::decode(body) else {
        return (StatusCode::BAD_REQUEST, "Failed to deserialize data").into_response();
    };

    // println!("{:?}", decoded.epicenter.unwrap());

    (StatusCode::OK, "a").into_response()
}

async fn handler(State(state): State<AppState>) -> Response {
    let (tx, rx) = tokio::sync::oneshot::channel();

    state
        .request_channel
        .send(UserEvent::RenderingRequest(tx))
        .await
        .unwrap();

    (
        [(CONTENT_TYPE, HeaderValue::from_str("image/png").unwrap())],
        rx.await.unwrap(),
    )
        .into_response()
}

pub async fn run(
    listen: &str,
    request_channel: tokio::sync::mpsc::Sender<UserEvent>,
    hmac_key: &str,
) {
    let shutdowner = model::Shutdowner::new(request_channel.clone());

    let hmac_key = Arc::new(hmac_key.to_string());

    let app = Router::new()
        .fallback(get(render_handler))
        .with_state(AppState {
            request_channel,
            hmac_key,
        });

    let listener = tokio::net::TcpListener::bind(listen).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    std::mem::drop(shutdowner);
}
