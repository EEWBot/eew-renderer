pub mod quake_prefecture {
    include!(concat!(env!("OUT_DIR"), "/quake_prefecture_v0.rs"));
}

mod model;
mod rendering_context_v0;
mod web;
mod worker;
mod namesgenerator;

use std::error::Error;
use std::net::SocketAddr;

use clap::Parser;

use crate::model::*;
use crate::rendering_context_v0::RenderingContextV0;

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

    #[clap(long, env)]
    #[clap(default_value = "200ms")]
    minimum_response_interval: humantime::Duration,

    #[clap(long, env)]
    #[clap(default_value_t = 512)]
    image_cache_capacity: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    tracing::info!("Instance Name: {}", cli.instance_name);
    tracing::info!("Allow Demo: {}", cli.security_rules.allow_demo);
    tracing::info!("Allow RemoteIP from CF header: {}", cli.security_rules.allow_remote_ip_from_cf_header);
    tracing::info!("Image Cache Capacity: {}", cli.image_cache_capacity);
    tracing::info!("Minimum Response Interval: {}", cli.minimum_response_interval);

    if cli.security_rules.bypass_hmac {
        tracing::warn!("[SECURITY NOTICE] BYPASS HMAC MODE!");
        tracing::warn!("[SECURITY NOTICE] DO NOT USE THIS OPTION IN PRODUCTION!!");
    }

    let (webe_tx, webe_rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
    let (tx, rx) = tokio::sync::mpsc::channel::<Message>(16);

    tokio::spawn(async move {
        let e = web::run(
            cli.listen,
            tx,
            &cli.hmac_key,
            &cli.instance_name,
            cli.security_rules,
            cli.minimum_response_interval.into(),
            cli.image_cache_capacity,
        )
        .await;

        webe_tx.send(e).unwrap()
    });

    tokio::select! {
        e = worker::run(rx) => {
            tracing::error!("UNRECOVERABLE ERROR (Worker): {e:?}");
        }
        e = webe_rx => {
            tracing::error!("UNRECOVERABLE ERROR (Web): {e:?}");
        }
    }

    Ok(())
}
