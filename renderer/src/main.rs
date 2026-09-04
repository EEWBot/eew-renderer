pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/net.eewbot.rs"));
}

mod frame_context;
mod model;
mod namesgenerator;
mod rendering_context;
mod station_watcher;
mod web;
mod worker;

use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

use crate::model::*;

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

    #[clap(long, env)]
    intensity_stations: Option<PathBuf>,
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
    tracing::info!(
        "Intensity Stations: {}",
        cli.intensity_stations
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "[embedded]".to_owned())
    );

    let watcher = match cli.intensity_stations.as_deref() {
        None => None,
        Some(path) => match station_watcher::prepare(path) {
            Ok(watcher) => Some(watcher),
            Err(e) => {
                tracing::error!("UNRECOVERABLE ERROR (Assets): {e:?}");
                std::process::exit(1);
            }
        },
    };

    if let Err(e) =
        renderer_assets::QueryInterface::init_intensity_stations(cli.intensity_stations.as_deref())
    {
        tracing::error!("UNRECOVERABLE ERROR (Assets): {e:?}");
        std::process::exit(1);
    }

    if let Some(watcher) = watcher {
        watcher.start();
    }

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
