pub mod quake_prefecture {
    include!(concat!(env!("OUT_DIR"), "/quake_prefecture_v0.rs"));
}

mod model;
mod rendering_context_v0;
mod web;
mod worker;

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

    #[clap(env, long, default_value_t = false)]
    allow_demo: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let (tx, rx) = tokio::sync::mpsc::channel::<Message>(16);

    tokio::spawn(async move {
        web::run(
            cli.listen,
            tx,
            &cli.hmac_key,
            &cli.instance_name,
            cli.allow_demo,
        )
        .await
    });

    worker::run(rx).await?;

    Ok(())
}
