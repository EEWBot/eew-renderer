use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

use image::{DynamicImage, RgbaImage};

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

async fn composite_image(
    rendering_context: RenderingContext,
    request_channel: &tokio::sync::mpsc::Sender<crate::model::Message>,
) -> Result<bytes::Bytes, RenderingError> {
    let request_identity = &rendering_context.request_identity;

    match rendering_context.payload {
        RenderingPayload::Earthquake(rendering_payload) => {
            let payload = rendering_payload.into_frame_payload();
            let (tx, rx) = tokio::sync::oneshot::channel();

            request_channel
                .send(crate::Message::FrameRequest((
                    crate::frame_context::FrameContext {
                        payload,
                        request_identity: request_identity.to_string(),
                    },
                    tx,
                )))
                .await
                .unwrap();

            let image = rx.await.unwrap()?;

            let start_at = Instant::now();
            let image = RgbaImage::from_raw(image.width, image.height, image.data).unwrap();

            let mut image = DynamicImage::ImageRgba8(image);
            image.apply_orientation(image::metadata::Orientation::FlipVertical);

            let encoder = webp::Encoder::from_image(&image).unwrap();
            let bin = encoder.encode(90f32).to_vec();

            let encode_time = Instant::now() - start_at;

            tracing::info!("Encode: {:?} ({request_identity})", encode_time);

            Ok(bytes::Bytes::from_owner(bin))
        }
        RenderingPayload::Tsunami(rendering_payload) => {
            let payloads = rendering_payload.into_frame_payloads();

            let mut frames = vec![];
            for payload in payloads {
                let (tx, rx) = tokio::sync::oneshot::channel();

                request_channel
                    .send(crate::Message::FrameRequest((
                        crate::frame_context::FrameContext {
                            payload,
                            request_identity: request_identity.to_string(),
                        },
                        tx,
                    )))
                    .await
                    .unwrap();

                let image = rx.await.unwrap()?;

                let image = RgbaImage::from_raw(image.width, image.height, image.data).unwrap();
                let mut image = DynamicImage::ImageRgba8(image);
                image.apply_orientation(image::metadata::Orientation::FlipVertical);

                frames.push(image);
            }

            let start_at = Instant::now();
            let mut config = webp::WebPConfig::new().unwrap();
            config.quality = 90f32;

            let first_frame = frames.first().unwrap();
            let mut encoder = webp::AnimEncoder::new(
                first_frame.width(),
                first_frame.height(),
                &config,
            );

            for (n, frame) in frames.iter().enumerate() {
                encoder.add_frame(webp::AnimFrame::from_image(frame, (n * 500) as i32).unwrap());
            }

            let bin = encoder.encode().to_vec();

            let encode_time = Instant::now() - start_at;

            tracing::info!("AnimEncode: {:?} ({request_identity})", encode_time);

            Ok(bytes::Bytes::from_owner(bin))
        }
    }
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
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to deserialize {type_id}"),
                )
                    .into_response();
            };

            RenderingPayload::try_from(decoded)
        }
        VersionedTypeId::TsunamiForecastV0 => {
            let Ok(decoded) = crate::proto::TsunamiForecastV0::decode(body) else {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to deserialize {type_id}"),
                )
                    .into_response();
            };

            RenderingPayload::try_from(decoded)
        }
        VersionedTypeId::TsunamiForecastV1 => {
            let Ok(decoded) = crate::proto::TsunamiForecastV1::decode(body) else {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to deserialize {type_id}"),
                )
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

    let rendering_context = RenderingContext {
        payload: rendering_payload,
        request_identity: request_identity.clone(),
    };

    let image_binary = app
        .cache
        .try_get_with::<_, RenderingError>(calculated_sha1.into(), async move {
            composite_image(rendering_context, &app.request_channel).await
        })
        .await;

    let image_binary = match image_binary {
        Ok(image_binary) => image_binary,
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
            (CONTENT_TYPE, HeaderValue::from_str("image/webp").unwrap()),
            (
                HeaderName::from_bytes(b"X-Instance-Name").unwrap(),
                HeaderValue::from_str(&app.instance_name).unwrap(),
            ),
        ],
        image_binary,
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
            震度::震度1 => vec![codes::地震情報細分区域(211), codes::地震情報細分区域(355), codes::地震情報細分区域(357), codes::地震情報細分区域(203), codes::地震情報細分区域(590), codes::地震情報細分区域(622), codes::地震情報細分区域(632), codes::地震情報細分区域(741), codes::地震情報細分区域(101), codes::地震情報細分区域(106), codes::地震情報細分区域(107), codes::地震情報細分区域(161), codes::地震情報細分区域(700), codes::地震情報細分区域(703), codes::地震情報細分区域(704), codes::地震情報細分区域(711), codes::地震情報細分区域(713)],
            震度::震度2 => vec![codes::地震情報細分区域(332), codes::地震情報細分区域(440), codes::地震情報細分区域(532), codes::地震情報細分区域(210), codes::地震情報細分区域(213), codes::地震情報細分区域(351), codes::地震情報細分区域(352), codes::地震情報細分区域(354), codes::地震情報細分区域(356), codes::地震情報細分区域(551), codes::地震情報細分区域(571), codes::地震情報細分区域(601), codes::地震情報細分区域(611), codes::地震情報細分区域(200), codes::地震情報細分区域(201), codes::地震情報細分区域(202), codes::地震情報細分区域(591), codes::地震情報細分区域(592), codes::地震情報細分区域(620), codes::地震情報細分区域(621), codes::地震情報細分区域(630), codes::地震情報細分区域(631), codes::地震情報細分区域(721), codes::地震情報細分区域(740), codes::地震情報細分区域(751), codes::地震情報細分区域(763), codes::地震情報細分区域(770)],
            震度::震度3 => vec![codes::地震情報細分区域(241), codes::地震情報細分区域(251), codes::地震情報細分区域(301), codes::地震情報細分区域(311), codes::地震情報細分区域(321), codes::地震情報細分区域(331), codes::地震情報細分区域(441), codes::地震情報細分区域(442), codes::地震情報細分区域(450), codes::地震情報細分区域(461), codes::地震情報細分区域(462), codes::地震情報細分区域(510), codes::地震情報細分区域(521), codes::地震情報細分区域(531), codes::地震情報細分区域(535), codes::地震情報細分区域(562), codes::地震情報細分区域(563), codes::地震情報細分区域(212), codes::地震情報細分区域(220), codes::地震情報細分区域(221), codes::地震情報細分区域(222), codes::地震情報細分区域(230), codes::地震情報細分区域(231), codes::地震情報細分区域(232), codes::地震情報細分区域(233), codes::地震情報細分区域(340), codes::地震情報細分区域(341), codes::地震情報細分区域(342), codes::地震情報細分区域(350), codes::地震情報細分区域(360), codes::地震情報細分区域(361), codes::地震情報細分区域(411), codes::地震情報細分区域(412), codes::地震情報細分区域(550), codes::地震情報細分区域(570), codes::地震情報細分区域(575), codes::地震情報細分区域(580), codes::地震情報細分区域(581), codes::地震情報細分区域(600), codes::地震情報細分区域(610)],
            震度::震度4 => vec![codes::地震情報細分区域(401), codes::地震情報細分区域(421), codes::地震情報細分区域(422), codes::地震情報細分区域(431), codes::地震情報細分区域(432), codes::地震情報細分区域(240), codes::地震情報細分区域(242), codes::地震情報細分区域(243), codes::地震情報細分区域(250), codes::地震情報細分区域(252), codes::地震情報細分区域(300), codes::地震情報細分区域(310), codes::地震情報細分区域(320), codes::地震情報細分区域(330), codes::地震情報細分区域(443), codes::地震情報細分区域(451), codes::地震情報細分区域(460), codes::地震情報細分区域(500), codes::地震情報細分区域(501), codes::地震情報細分区域(511), codes::地震情報細分区域(520), codes::地震情報細分区域(530), codes::地震情報細分区域(540), codes::地震情報細分区域(560)],
            震度::震度5弱 => vec![codes::地震情報細分区域(420), codes::地震情報細分区域(430)],
            震度::震度5強 => vec![codes::地震情報細分区域(391), codes::地震情報細分区域(370), codes::地震情報細分区域(372), codes::地震情報細分区域(375), codes::地震情報細分区域(380), codes::地震情報細分区域(381), codes::地震情報細分区域(400)],
            震度::震度6弱 => vec![codes::地震情報細分区域(371)],
            震度::震度6強 => vec![],
            震度::震度7 => vec![codes::地震情報細分区域(390)],
        },
    });

    let bin = composite_image(
        RenderingContext {
            payload: rendering_payload,
            request_identity: request_identity.clone(),
        },
        &app.request_channel,
    )
    .await
    .unwrap();

    let response_at = app.response_limiter.schedule([0; 20], request_identity);

    tokio::time::sleep_until(response_at.into()).await;

    (
        [
            (CONTENT_TYPE, HeaderValue::from_str("image/webp").unwrap()),
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
            津波情報::津波予報 => vec![codes::津波予報区(111), codes::津波予報区(202), codes::津波予報区(300), codes::津波予報区(310), codes::津波予報区(311), codes::津波予報区(312), codes::津波予報区(320), codes::津波予報区(321), codes::津波予報区(330), codes::津波予報区(380), codes::津波予報区(400), codes::津波予報区(580), codes::津波予報区(610), codes::津波予報区(771), codes::津波予報区(772)],
            津波情報::津波注意報 => vec![codes::津波予報区(100), codes::津波予報区(102), codes::津波予報区(200), codes::津波予報区(220), codes::津波予報区(250)],
            津波情報::津波警報 => vec![codes::津波予報区(101), codes::津波予報区(201), codes::津波予報区(210)],
            _ => vec![],
        },
    });

    let bin = composite_image(
        RenderingContext {
            payload: tsunami_payload,
            request_identity: request_identity.clone(),
        },
        &app.request_channel,
    )
    .await
    .unwrap();

    let response_at = app.response_limiter.schedule([0; 20], request_identity);

    tokio::time::sleep_until(response_at.into()).await;

    (
        [
            (CONTENT_TYPE, HeaderValue::from_str("image/webp").unwrap()),
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
