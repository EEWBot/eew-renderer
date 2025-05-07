use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::{
    extract::{Request, State},
    http::{header::CONTENT_TYPE, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::{DateTime, TimeZone};
use enum_map::enum_map;
use hmac::{Hmac, Mac};
use prost::Message;

use crate::model::*;

mod rate_limiter;
use rate_limiter::ResponseRateLimiter;

type HmacSha1 = Hmac<sha1::Sha1>;

#[derive(Clone, Debug)]
pub struct AppState {
    request_channel: tokio::sync::mpsc::Sender<crate::model::Message>,
    hmac_key: Arc<String>,
    instance_name: Arc<String>,
    allow_demo: bool,
    response_limiter: ResponseRateLimiter,
    cache: moka::future::Cache<[u8; 20], bytes::Bytes>,
}

async fn render_handler(State(app): State<AppState>, req: Request) -> Response {
    let request_id = crate::namesgenerator::generate(&mut rand::rng());

    let bin = &req.uri().path()[1..];

    let Ok(bin) = urlencoding::decode(bin) else {
        return (StatusCode::BAD_REQUEST, "Failed to UTF-8 parsing").into_response();
    };

    let Ok(bin) = base65536::decode(&bin, false) else {
        return (StatusCode::BAD_REQUEST, "Failed to Base65536 decoding").into_response();
    };

    if bin.len() < 21 {
        return (StatusCode::BAD_REQUEST, "Minimum length is not satisfied").into_response();
    }

    let version = bin[0];
    let provided_sha1 = &bin[1..21];
    let body = &bin[21..];

    if version != 0 {
        return (
            StatusCode::BAD_REQUEST,
            format!("Unknown protocol v{version}"),
        )
            .into_response();
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

    let short_hash = &format!("{:x}", calculated_sha1)[0..6];
    let request_identity = &format!("{short_hash}#{request_id}");

    tracing::info!("Request: {request_identity}");

    let bin = app
        .cache
        .get_with(calculated_sha1.into(), async move {
            let rendering_context = crate::rendering_context_v0::RenderingContextV0 {
                time: DateTime::from_timestamp(decoded.time as i64, 0).unwrap(),
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
                request_identity: request_identity.to_string(),
            };

            let (tx, rx) = tokio::sync::oneshot::channel();

            app.request_channel
                .send(crate::Message::RenderingRequest((rendering_context, tx)))
                .await
                .unwrap();

            bytes::Bytes::from_owner(rx.await.unwrap())
        })
        .await;

    let response_at = app
        .response_limiter
        .schedule(calculated_sha1.into(), request_identity);

    tokio::time::sleep_until(response_at.into()).await;

    (
        [
            (CONTENT_TYPE, HeaderValue::from_str("image/png").unwrap()),
            (
                HeaderName::from_bytes(b"X-Instance-Name").unwrap(),
                HeaderValue::from_str(&app.instance_name).unwrap(),
            ),
        ],
        bin,
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

async fn demo_handler(State(app): State<AppState>) -> Response {
    let request_id = crate::namesgenerator::generate(&mut rand::rng());

    use renderer_types::*;

    if !app.allow_demo {
        return (StatusCode::UNAUTHORIZED, "Demo endpoint is not allowed").into_response();
    }

    let request_identity = &format!("demo#{request_id}");

    let rendering_context = crate::rendering_context_v0::RenderingContextV0 {
        time: chrono_tz::Japan
            .with_ymd_and_hms(2024, 1, 1, 16, 10, 0)
            .unwrap()
            .to_utc(),
        epicenter: Some(Vertex::<GeoDegree>::new(137.2, 37.5)),
        area_intensities: enum_map! {
            震度::震度1 => vec![211, 355, 357, 203, 590, 622, 632, 741, 101, 106, 107, 161, 700, 703, 704, 711, 713],
            震度::震度2 => vec![332, 440, 532, 210, 213, 351, 352, 354, 356, 551, 571, 601, 611, 200, 201, 202, 591, 592, 620, 621, 630, 631, 721, 740, 751, 763, 770],
            震度::震度3 => vec![241, 251, 301, 311, 321, 331, 441, 442, 450, 461, 462, 510, 521, 531, 535, 562, 563, 212, 220, 221, 222, 230, 231, 232, 233, 340, 341, 342, 350, 360, 361, 411, 412, 550, 570, 575, 580, 581, 600, 610],
            震度::震度4 => vec![401, 421, 422, 431, 432, 240, 242, 243, 250, 252, 300, 310, 320, 330, 443, 451, 460, 500, 501, 511, 520, 530, 540, 560],
            震度::震度5弱 => vec![420, 430],
            震度::震度5強 => vec![391, 370, 372, 375, 380, 381, 400],
            震度::震度6弱 => vec![371],
            震度::震度6強 => vec![],
            震度::震度7 => vec![390],
        },
        request_identity: request_identity.to_string(),
    };

    let (tx, rx) = tokio::sync::oneshot::channel();

    app.request_channel
        .send(crate::Message::RenderingRequest((rendering_context, tx)))
        .await
        .unwrap();

    let bin = rx.await.unwrap();

    let response_at = app.response_limiter.schedule([0; 20], request_identity);

    tokio::time::sleep_until(response_at.into()).await;

    (
        [
            (CONTENT_TYPE, HeaderValue::from_str("image/png").unwrap()),
            (
                HeaderName::from_bytes(b"X-Instance-Name").unwrap(),
                HeaderValue::from_str(&app.instance_name).unwrap(),
            ),
        ],
        bin,
    )
        .into_response()
}

pub async fn run(
    listen: SocketAddr,
    request_channel: tokio::sync::mpsc::Sender<crate::Message>,
    hmac_key: &str,
    instance_name: &str,
    allow_demo: bool,
    minimum_response_interval: Duration,
    image_cache_capacity: u64,
) -> Result<()> {
    let hmac_key = Arc::new(hmac_key.to_string());
    let instance_name = Arc::new(instance_name.to_string());

    let response_limiter = ResponseRateLimiter::new(minimum_response_interval);

    let cache = moka::future::Cache::builder()
        .max_capacity(image_cache_capacity)
        .build();

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/demo", get(demo_handler))
        .fallback(get(render_handler))
        .with_state(AppState {
            request_channel,
            hmac_key,
            instance_name,
            allow_demo,
            response_limiter,
            cache,
        });

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("Failed to bind address {listen}"))?;

    tracing::info!("Listening on {listen}");

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
