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
use axum_client_ip::{ClientIp, ClientIpSource};
use axum_extra::TypedHeader;
use chrono::TimeZone;
use enum_map::enum_map;
use headers::UserAgent;
use hmac::{Hmac, KeyInit, Mac};
use prost::Message;

use crate::model::*;
use crate::rendering_context::{
    EarthquakePayload, RenderingContext, RenderingPayload, TsunamiPayload,
};

mod rate_limiter;
use rate_limiter::ResponseRateLimiter;

mod versioned_type_id;
use versioned_type_id::VersionedTypeId;

type HmacSha1 = Hmac<sha1::Sha1>;
pub(in crate::web) type Sha1Bytes = [u8; 20];

#[derive(Clone, Debug, clap::Parser)]
pub struct SecurityRules {
    #[clap(env, long, default_value_t = false)]
    pub allow_demo: bool,

    #[clap(env, long, default_value_t = false)]
    pub bypass_hmac: bool,
}

#[derive(Clone, Debug)]
pub struct AppState {
    request_channel: tokio::sync::mpsc::Sender<crate::model::Message>,
    hmac_key: Arc<String>,
    instance_name: Arc<String>,
    response_limiter: ResponseRateLimiter,
    security_rules: SecurityRules,
    cache: moka::future::Cache<Sha1Bytes, bytes::Bytes>,
}

async fn render_handler(
    State(app): State<AppState>,
    ClientIp(client_ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    req: Request,
) -> Response {
    let request_id = crate::namesgenerator::generate(&mut rand::rng());

    let bin = &req.uri().path()[1..];

    let Ok(bin) = urlencoding::decode(bin) else {
        return (StatusCode::BAD_REQUEST, "Failed to UTF-8 parsing").into_response();
    };

    let Some(first_char) = bin.chars().next() else {
        return (
            StatusCode::BAD_REQUEST,
            "Failed to get first char for decoding",
        )
            .into_response();
    };

    let bin = if first_char as u16 & 0xff == 0 {
        let Ok(bin) = base65536::decode(&bin, false) else {
            return (StatusCode::BAD_REQUEST, "Failed to Base65536 decoding").into_response();
        };

        bin
    } else {
        let Ok(bin) = base32768::decode(&bin) else {
            return (StatusCode::BAD_REQUEST, "Failed to Base32768 decoding").into_response();
        };

        bin
    };

    use generic_array::typenum::U20;
    use generic_array::GenericArray;

    let is_legacy_format = first_char as u16 & 0xff == 0;

    let (raw_type_id, provided_sha1, body) = if is_legacy_format {
        if bin.len() < 21 {
            return (
                StatusCode::BAD_REQUEST,
                "Minimum length is not satisfied (Base65536)",
            )
                .into_response();
        }

        let raw_type_id = bin[0];
        let provided_sha1 = GenericArray::<_, U20>::from_slice(&bin[1..21]);
        let body = &bin[21..];

        (raw_type_id, provided_sha1, body)
    } else {
        if bin.len() < 22 {
            return (
                StatusCode::BAD_REQUEST,
                "Minimum length is not satisfied (Base32768)",
            )
                .into_response();
        }

        let raw_type_id = bin[0];
        let _non_base65536_marker = bin[1];
        let provided_sha1 = GenericArray::<_, U20>::from_slice(&bin[2..22]);
        let body = &bin[22..];

        (raw_type_id, provided_sha1, body)
    };

    let type_id = match VersionedTypeId::try_from(raw_type_id) {
        Ok(type_id) => type_id,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
        }
    };

    if is_legacy_format && !type_id.is_legacy_format_allowed() {
        return (StatusCode::BAD_REQUEST, "Unused pair detected").into_response();
    }

    let signing_target = if is_legacy_format {
        body.to_vec()
    } else {
        let mut buffer = Vec::new();
        buffer.push(raw_type_id);
        buffer.extend_from_slice(body);
        buffer
    };

    let calculated_sha1 = {
        let mut mac = HmacSha1::new_from_slice(app.hmac_key.as_bytes()).unwrap();
        mac.update(&signing_target);
        mac.finalize().into_bytes()
    };

    let short_hash = &format!("{:x}", provided_sha1)[0..6];

    if *calculated_sha1 != **provided_sha1 {
        if app.security_rules.bypass_hmac {
            tracing::warn!(
                "Invalid a HMAC Key provided, but allowed by server configuration. {short_hash}"
            );
        } else {
            return (StatusCode::UNAUTHORIZED, "Invalid HMAC Key").into_response();
        }
    }

    let request_identity = &format!("{short_hash}#{request_id}");

    let is_legacy = if is_legacy_format { "/legacy" } else { "" };
    tracing::info!(
        "Request({type_id}{is_legacy}): {request_identity} [{client_ip}] - {user_agent}"
    );

    let maybe_rendering_payload = match type_id {
        VersionedTypeId::QuakePrefectureV0 => {
            let Ok(decoded) = crate::proto::QuakePrefectureV0::decode(body) else {
                return (StatusCode::BAD_REQUEST, "Failed to deserialize {type_id}")
                    .into_response();
            };

            RenderingPayload::try_from(decoded)
        }
        VersionedTypeId::TsunamiForecastV0 => {
            let Ok(decoded) = crate::proto::TsunamiForecastV0::decode(body) else {
                return (StatusCode::BAD_REQUEST, "Failed to deserialize {type_id}")
                    .into_response();
            };

            RenderingPayload::try_from(decoded)
        }
        VersionedTypeId::TsunamiForecastV1 => {
            let Ok(decoded) = crate::proto::TsunamiForecastV1::decode(body) else {
                return (StatusCode::BAD_REQUEST, "Failed to deserialize {type_id}")
                    .into_response();
            };

            RenderingPayload::try_from(decoded)
        }
    };

    let rendering_payload = match maybe_rendering_payload {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("{e} ({request_identity})");
            return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
        }
    };

    let png = app
        .cache
        .try_get_with::<_, RenderingError>(calculated_sha1.into(), async move {
            let (tx, rx) = tokio::sync::oneshot::channel();

            app.request_channel
                .send(crate::Message::RenderingRequest((
                    RenderingContext {
                        payload: rendering_payload,
                        request_identity: request_identity.clone(),
                    },
                    tx,
                )))
                .await
                .unwrap();

            Ok(bytes::Bytes::from_owner(rx.await.unwrap()?))
        })
        .await;

    let png = match png {
        Ok(png) => png,
        Err(e) => {
            tracing::error!("Request {short_hash} is errored. Code: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

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
        png,
    )
        .into_response()
}

async fn root_handler(State(app): State<AppState>, ClientIp(_client_ip): ClientIp) -> Response {
    (
        [(CONTENT_TYPE, HeaderValue::from_str("text/html").unwrap())],
        format!(
            "<h1>EEW Renderer</h1><p>Instance Name: {}</p><p>Demo Endpoint: {}</p><p>Bypass HMAC: {}</p>",
            app.instance_name,
            app.security_rules.allow_demo,
            app.security_rules.bypass_hmac,
        ),
    )
        .into_response()
}

async fn demo_handler(
    State(app): State<AppState>,
    ClientIp(client_ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
) -> Response {
    let request_id = crate::namesgenerator::generate(&mut rand::rng());

    use renderer_types::*;

    if !app.security_rules.allow_demo {
        return (StatusCode::UNAUTHORIZED, "Demo endpoint is not allowed").into_response();
    }

    let request_identity = &format!("demo#{request_id}");
    tracing::info!("Request: {request_identity} [{client_ip}] - {user_agent}");

    let rendering_payload = RenderingPayload::Earthquake(EarthquakePayload {
        time: chrono_tz::Japan
            .with_ymd_and_hms(2024, 1, 1, 16, 10, 0)
            .unwrap()
            .to_utc(),
        epicenter: vec![Vertex::<GeoDegree>::new(137.2, 37.5)],
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
    });

    let (tx, rx) = tokio::sync::oneshot::channel();

    app.request_channel
        .send(crate::Message::RenderingRequest((
            RenderingContext {
                payload: rendering_payload,
                request_identity: request_identity.clone(),
            },
            tx,
        )))
        .await
        .unwrap();

    let bin = rx.await.unwrap().unwrap();

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

async fn tsunami_demo_handler(
    State(app): State<AppState>,
    ClientIp(client_ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
) -> Response {
    let request_id = crate::namesgenerator::generate(&mut rand::rng());

    use renderer_types::*;

    if !app.security_rules.allow_demo {
        return (StatusCode::UNAUTHORIZED, "Demo endpoint is not allowed").into_response();
    }

    let request_identity = &format!("demo#{request_id}");
    tracing::info!("TsunamiRequest: {request_identity} [{client_ip}] - {user_agent}");

    let tsunami_payload = RenderingPayload::Tsunami(TsunamiPayload {
        time: chrono_tz::Japan
            .with_ymd_and_hms(2025, 12, 8, 23, 23, 0)
            .unwrap()
            .to_utc(),
        epicenter: vec![Vertex::<GeoDegree>::new(142.3, 41.0)],
        forecast_levels: enum_map! {
            津波情報::津波予報 => vec![111, 202, 300, 310, 311, 312, 320, 321, 330, 380, 400, 580, 610, 771, 772],
            津波情報::津波注意報 => vec![100, 102, 200, 220, 250],
            津波情報::津波警報 => vec![101, 201, 210],
            _ => vec![],
        },
    });

    let (tx, rx) = tokio::sync::oneshot::channel();

    app.request_channel
        .send(crate::Message::RenderingRequest((
            RenderingContext {
                payload: tsunami_payload,
                request_identity: request_identity.clone(),
            },
            tx,
        )))
        .await
        .unwrap();

    let bin = rx.await.unwrap().unwrap();

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

#[allow(clippy::too_many_arguments)]
pub async fn run(
    listen: SocketAddr,
    request_channel: tokio::sync::mpsc::Sender<crate::Message>,
    hmac_key: &str,
    instance_name: &str,
    client_ip_source: ClientIpSource,
    security_rules: SecurityRules,
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
        .route("/tsunami_demo", get(tsunami_demo_handler))
        .fallback(get(render_handler))
        .with_state(AppState {
            request_channel,
            hmac_key,
            instance_name,
            security_rules,
            response_limiter,
            cache,
        })
        .layer(client_ip_source.into_extension());

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("Failed to bind address {listen}"))?;

    tracing::info!("Listening on {listen}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}
