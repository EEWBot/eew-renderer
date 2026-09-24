pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/net.eewbot.rs"));
}

mod frame_context;
mod model;
mod namesgenerator;
mod rendering_context;
mod station_http;
mod station_source;
mod station_watcher;
mod web;
mod worker;

use std::error::Error;
use std::fmt;
use std::net::SocketAddr;

use clap::Parser;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::model::*;
use crate::station_source::IntensityStationsSource;

const CF_ACCESS_CLIENT_ID: &str = "CF-Access-Client-Id";
const CF_ACCESS_CLIENT_SECRET: &str = "CF-Access-Client-Secret";

#[derive(Clone)]
struct Secret(String);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl std::str::FromStr for Secret {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

#[derive(Parser)]
struct Cli {
    #[clap(env, long, default_value = "")]
    hmac_key: String,

    #[clap(env, long, default_value = "[not specified]")]
    instance_name: String,

    #[clap(long, env)]
    #[clap(default_value = "0.0.0.0:3000")]
    listen: SocketAddr,

    #[command(flatten)]
    security_rules: web::SecurityRules,

    /// See: https://docs.rs/axum-client-ip/1.0.0/axum_client_ip/index.html#configurable-vs-specific-extractors
    #[clap(env, long, default_value = "ConnectInfo")]
    client_ip_source: axum_client_ip::ClientIpSource,

    #[clap(long, env)]
    #[clap(default_value = "200ms")]
    minimum_response_interval: humantime::Duration,

    #[clap(long, env)]
    #[clap(default_value_t = 512)]
    image_cache_capacity: u64,

    #[clap(long, env, default_value_t = false)]
    headless: bool,

    #[clap(long, env, default_value_t = 0)]
    egl_device_index: usize,

    #[clap(long, env, default_value = "embedded")]
    intensity_stations_source: IntensityStationsSource,

    #[clap(long, env, default_value = "60s")]
    intensity_stations_poll_interval: humantime::Duration,

    #[clap(long, env, hide_env_values = true)]
    intensity_stations_cf_access_client_id: Option<Secret>,

    #[clap(long, env, hide_env_values = true)]
    intensity_stations_cf_access_client_secret: Option<Secret>,

    #[clap(long, env, hide_env_values = true, value_delimiter = '\n')]
    intensity_stations_http_header: Vec<String>,
}

#[derive(thiserror::Error)]
enum ConfigError {
    #[error("HTTPヘッダの形式が不正 (NAME:VALUE である必要がある)")]
    MalformedHeader,

    #[error("HTTPヘッダ名が不正: {0:?}")]
    InvalidHeaderName(String),

    #[error("HTTPヘッダ {0:?} の値が不正")]
    InvalidHeaderValue(String),

    #[error("INTENSITY_STATIONS_CF_ACCESS_CLIENT_ID と _SECRET は両方指定する必要がある")]
    IncompleteCfAccessCredential,

    #[error("INTENSITY_STATIONS_POLL_INTERVAL に 0 は指定できない")]
    ZeroPollInterval,
}

impl fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

fn parse_http_header(raw: &str) -> Result<(HeaderName, HeaderValue), ConfigError> {
    let (name, value) = raw.split_once(':').ok_or(ConfigError::MalformedHeader)?;

    let (name, value) = (name.trim(), value.trim());

    let header_name: HeaderName = name
        .parse()
        .map_err(|_| ConfigError::InvalidHeaderName(name.to_owned()))?;

    Ok((header_name, sensitive_header_value(name, value)?))
}

fn sensitive_header_value(name: &str, value: &str) -> Result<HeaderValue, ConfigError> {
    let mut value = HeaderValue::from_str(value)
        .map_err(|_| ConfigError::InvalidHeaderValue(name.to_owned()))?;

    value.set_sensitive(true);

    Ok(value)
}

fn build_http_headers(cli: &Cli) -> Result<HeaderMap, ConfigError> {
    let mut headers = HeaderMap::new();

    for raw in &cli.intensity_stations_http_header {
        let (name, value) = parse_http_header(raw)?;
        headers.insert(name, value);
    }

    match (
        &cli.intensity_stations_cf_access_client_id,
        &cli.intensity_stations_cf_access_client_secret,
    ) {
        (None, None) => {}
        (Some(id), Some(secret)) => {
            let id = sensitive_header_value(CF_ACCESS_CLIENT_ID, &id.0)?;
            let secret = sensitive_header_value(CF_ACCESS_CLIENT_SECRET, &secret.0)?;

            headers.insert(HeaderName::from_static("cf-access-client-id"), id);
            headers.insert(HeaderName::from_static("cf-access-client-secret"), secret);
        }
        _ => return Err(ConfigError::IncompleteCfAccessCredential),
    }

    Ok(headers)
}

fn has_http_only_options(cli: &Cli) -> bool {
    cli.intensity_stations_cf_access_client_id.is_some()
        || cli.intensity_stations_cf_access_client_secret.is_some()
        || !cli.intensity_stations_http_header.is_empty()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    tracing::info!("Instance Name: {}", cli.instance_name);
    tracing::info!("ClientIP from: {:?}", cli.client_ip_source);
    tracing::info!("Image Cache Capacity: {}", cli.image_cache_capacity);
    tracing::info!(
        "Minimum Response Interval: {}",
        cli.minimum_response_interval
    );
    tracing::info!("Headless: {}", cli.headless);
    tracing::info!("Intensity Stations: {}", cli.intensity_stations_source);

    init_intensity_stations(&cli).await;

    if cli.security_rules.bypass_hmac {
        tracing::warn!("[SECURITY NOTICE] BYPASS HMAC MODE!");
        tracing::warn!("[SECURITY NOTICE] DO NOT USE THIS OPTION IN PRODUCTION!!");
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Message>(16);

    let (headless, egl_device_index) = (cli.headless, cli.egl_device_index);

    let listener = tokio::net::TcpListener::bind(&cli.listen).await.unwrap();
    tracing::info!("Listening on {}", cli.listen);

    tokio::spawn(async move {
        let e = web::run(
            listener,
            tx,
            &cli.hmac_key,
            &cli.instance_name,
            cli.client_ip_source,
            cli.security_rules,
            cli.minimum_response_interval.into(),
            cli.image_cache_capacity,
        )
        .await;

        tracing::error!("UNRECOVERABLE ERROR (Web): {e:?}");
        std::process::exit(1);
    });

    if let Err(e) = worker::run(rx, headless, egl_device_index).await {
        tracing::error!("UNRECOVERABLE ERROR (Worker): {e:?}");
        std::process::exit(1);
    }

    Ok(())
}

async fn init_intensity_stations(cli: &Cli) {
    match &cli.intensity_stations_source {
        IntensityStationsSource::Embedded => {
            warn_unused_http_options(cli);

            fatal_on_error(renderer_assets::QueryInterface::init_intensity_stations(
                renderer_assets::EMBEDDED_INTENSITY_STATIONS,
            ));
        }

        IntensityStationsSource::File(path) => {
            warn_unused_http_options(cli);

            let watcher = fatal_on_error(station_watcher::prepare(path));
            let data = fatal_on_error(station_watcher::load(path));

            fatal_on_error(renderer_assets::QueryInterface::init_intensity_stations(
                &data,
            ));

            watcher.start();
        }

        IntensityStationsSource::Http(url) => {
            let interval: std::time::Duration = cli.intensity_stations_poll_interval.into();

            if interval.is_zero() {
                fatal_on_error::<(), _>(Err(ConfigError::ZeroPollInterval));
            }

            let headers = fatal_on_error(build_http_headers(cli));

            tracing::info!(
                "Intensity Stations Poll Interval: {}",
                cli.intensity_stations_poll_interval
            );

            let mut source = fatal_on_error(station_http::prepare(url.clone(), headers, interval));

            fatal_on_error(source.initial_load().await);

            source.start();
        }
    }
}

fn warn_unused_http_options(cli: &Cli) {
    if has_http_only_options(cli) {
        tracing::warn!(
            "Cloudflare Access credentials and custom HTTP headers are ignored for non-HTTP sources"
        );
    }
}

fn fatal_on_error<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("UNRECOVERABLE ERROR (Assets): {e:?}");
            std::process::exit(1);
        }
    }
}
