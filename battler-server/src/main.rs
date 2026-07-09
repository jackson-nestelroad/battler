use std::net::IpAddr;

use anyhow::{
    Error,
    Result,
};
use battler_server::{
    ServerConfig,
    start_server,
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "battler-server",
    about = "WAMP router and battler service host"
)]
struct Args {
    /// IP address to listen on
    #[arg(short, long, default_value = "127.0.0.1")]
    address: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Path to Pokemon data directory
    #[arg(short, long, default_value = "battle-data/data")]
    data_dir: String,

    /// Name of the WAMP realm
    #[arg(long, default_value = "battler")]
    realm_name: String,

    /// URI of the WAMP realm
    #[arg(long, default_value = "com.battler")]
    realm_uri: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if let Err(err) = run_server().await {
        log::error!("Server error: {err:#}");
        std::process::exit(1);
    }
}

async fn run_server() -> Result<()> {
    let args = Args::parse();

    log::info!("Starting Battler Server...");
    let mut handle = start_server(ServerConfig {
        address: args.address,
        port: args.port,
        data_dir: args.data_dir,
        realm_name: args.realm_name.clone(),
        realm_uri: args.realm_uri,
    })
    .await?;

    log::info!(
        "Server is running at ws://{}:{}/ (realm: {})",
        args.address,
        handle.router_handle.local_addr().port(),
        args.realm_name
    );

    let exit_result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received shutdown signal. Stopping services...");
            Ok(())
        }
        res = &mut handle.router_join_handle => {
            Err(handle_task_error("WAMP router", res))
        }
        res = &mut handle.battle_producer_handle => {
            Err(handle_task_error("Battle producer", res))
        }
        res = &mut handle.multiplayer_producer_handle => {
            Err(handle_task_error("Multiplayer producer", res))
        }
    };

    handle.shutdown().await?;
    log::info!("Server stopped successfully.");

    exit_result
}

fn handle_task_error<T: std::fmt::Debug>(name: &str, result: T) -> Error {
    Error::msg(format!("{name} exited unexpectedly: {:?}", result))
}
