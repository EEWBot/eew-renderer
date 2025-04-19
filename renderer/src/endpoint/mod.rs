use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header::CONTENT_TYPE, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use enum_map::enum_map;
use hmac::{Hmac, Mac};
use prost::Message;

mod model;
use crate::intensity::震度;
use crate::model::*;

type HmacSha1 = Hmac<sha1::Sha1>;

#[derive(Clone, Debug)]
pub struct AppState {
    pub request_channel: tokio::sync::mpsc::Sender<UserEvent>,
    pub hmac_key: Arc<String>,
    pub instance_name: Arc<String>,
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
        return (StatusCode::UNAUTHORIZED, "Invalid HMAC Key").into_response();
    }

    let Ok(decoded) = crate::quake_prefecture::QuakePrefectureData::decode(body) else {
        return (StatusCode::BAD_REQUEST, "Failed to deserialize data").into_response();
    };

    let rendering_context = crate::rendering_context_v0::RenderingContextV0 {
        epicenter: decoded.epicenter.map(
            |crate::quake_prefecture::Epicenter { lat_x10, lon_x10 }| {
                renderer_types::Vertex::new(lon_x10 as f32 / 10.0, lat_x10 as f32 / 10.0)
            },
        ),
        area_intensities: enum_map! {
            震度::震度1 => decoded.one.clone().map(|v| v.codes).unwrap_or(vec![]),
            震度::震度2 => decoded.two.clone().map(|v| v.codes).unwrap_or(vec![]),
            震度::震度3 => decoded.three.clone().map(|v| v.codes).unwrap_or(vec![]),
            震度::震度4 => decoded.four.clone().map(|v| v.codes).unwrap_or(vec![]),
            震度::震度5弱 => decoded.five_minus.clone().map(|v| v.codes).unwrap_or(vec![]),
            震度::震度5強 => decoded.five_plus.clone().map(|v| v.codes).unwrap_or(vec![]),
            震度::震度6弱 => decoded.six_minus.clone().map(|v| v.codes).unwrap_or(vec![]),
            震度::震度6強 => decoded.six_plus.clone().map(|v| v.codes).unwrap_or(vec![]),
            震度::震度7 => decoded.seven.clone().map(|v| v.codes).unwrap_or(vec![]),
        },
    };

    let (tx, rx) = tokio::sync::oneshot::channel();

    app.request_channel
        .send(UserEvent::RenderingRequest((rendering_context, tx)))
        .await
        .unwrap();

    (
        [
            (CONTENT_TYPE, HeaderValue::from_str("image/png").unwrap()),
            (
                HeaderName::from_bytes(b"X-Instance-Name").unwrap(),
                HeaderValue::from_str(&app.instance_name).unwrap(),
            ),
        ],
        rx.await.unwrap(),
    )
        .into_response()
}

async fn root_handler(State(app): State<AppState>) -> Response {
    (
        [(CONTENT_TYPE, HeaderValue::from_str("text/html").unwrap())],
        format!(
            "<h1>EEW Renderer</h1><p>Instance Name: {}</p>",
            app.instance_name
        ),
    )
        .into_response()
}

pub async fn run(
    listen: &str,
    request_channel: tokio::sync::mpsc::Sender<UserEvent>,
    hmac_key: &str,
    instance_name: &str,
) {
    let shutdowner = model::Shutdowner::new(request_channel.clone());

    let hmac_key = Arc::new(hmac_key.to_string());
    let instance_name = Arc::new(instance_name.to_string());

    let app = Router::new()
        .route("/", get(root_handler))
        .fallback(get(render_handler))
        .with_state(AppState {
            request_channel,
            hmac_key,
            instance_name,
        });

    let listener = tokio::net::TcpListener::bind(listen).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    std::mem::drop(shutdowner);
}
