mod actions;
mod ai;
mod adb;
mod cli;
mod config;
mod discord;
mod editor;
mod http;
mod hub;
mod models;
mod platform;
mod protocol;
mod providers;
mod server;
mod tools;

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Context, Result};
use tokio::sync::Notify;

use crate::config::Config;
use crate::hub::Hub;

/// Shared state of the agent.
pub struct App {
    pub hub: Hub,
    pub config: RwLock<Config>,
    pub config_path: PathBuf,
    pub port: u16,
    /// Cover of the current track: `(id, image bytes)`.
    pub art: Mutex<Option<(String, Vec<u8>)>>,
    /// Wakes the media provider right after an action.
    pub kick: Notify,
    /// Focus timer, to-do list and process list.
    pub tools: tools::Tools,
    /// Claude Code sessions reported through hooks.
    pub ai: ai::Ai,
    /// Who is online on the watched Discord servers.
    pub discord: discord::Discord,
}

impl App {
    /// The user's own images and fonts, next to the config file.
    pub fn assets_dir(&self) -> PathBuf {
        self.config_path.parent().unwrap_or(std::path::Path::new(".")).join("assets")
    }
}

struct Args {
    config: PathBuf,
    port: Option<u16>,
}

fn parse_args() -> Result<Args> {
    let mut args = Args { config: Config::default_path(), port: None };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--config" => args.config = it.next().context("--config needs a path")?.into(),
            "--port" => args.port = Some(it.next().context("--port needs a number")?.parse().context("invalid --port")?),
            "-h" | "--help" => {
                println!("streamphone-agent [--config <path>] [--port <number>]\n\n{}", cli::HELP);
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }
    Ok(args)
}

#[tokio::main]
async fn main() -> Result<()> {
    // `hook` and `report` are one-shot messengers to an agent that is already running.
    let cli_args: Vec<String> = std::env::args().skip(1).collect();
    if cli::run(&cli_args) {
        return Ok(());
    }

    tracing_subscriber::fmt().with_target(false).init();

    let args = parse_args()?;
    let config = Config::load(&args.config)?;
    let port = args.port.unwrap_or(config.port);
    tracing::info!("config: {}", args.config.display());

    let app = Arc::new(App {
        hub: Hub::new(),
        config: RwLock::new(config),
        config_path: args.config.clone(),
        port,
        art: Mutex::new(None),
        kick: Notify::new(),
        ai: ai::Ai::default(),
        discord: discord::Discord::default(),
        tools: tools::Tools::new(args.config.parent().unwrap_or(std::path::Path::new("."))),
    });

    // Loopback only: the phone gets in through the USB port forward.
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("port {port} is in use (is the agent already running?)"))?;
    tracing::info!("dashboard: http://localhost:{port}/");

    let _ = std::fs::create_dir_all(app.assets_dir());
    tracing::info!("images and fonts: {}", app.assets_dir().display());

    providers::spawn(app.clone());
    tools::spawn(app.clone());
    ai::spawn(app.clone());
    models::spawn(app.clone());
    discord::spawn(app.clone());
    tokio::spawn(config::watch(app.clone()));
    tokio::spawn(adb::run(app.clone()));

    // The editor gets the next port. Only the dashboard port is forwarded to
    // the phone, so the editor stays reachable from this PC alone.
    let editor_port = port.wrapping_add(1);
    match tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, editor_port))).await {
        Ok(editor_listener) => {
            tracing::info!("editor: http://localhost:{editor_port}/");
            let router = editor::router(app.clone());
            tokio::spawn(async move {
                if let Err(err) = axum::serve(editor_listener, router).await {
                    tracing::warn!("editor stopped: {err}");
                }
            });
        }
        Err(err) => tracing::warn!("editor is off, port {editor_port} is taken: {err}"),
    }
    // Also on IPv6 loopback: Windows tries `localhost` there first and only
    // falls back to IPv4 after two seconds, which would stall every hook call.
    if let Ok(v6) = tokio::net::TcpListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, editor_port))).await {
        let router = editor::router(app.clone());
        tokio::spawn(async move {
            let _ = axum::serve(v6, router).await;
        });
    }

    axum::serve(listener, server::router(app))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
