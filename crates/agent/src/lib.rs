//! The StreamPhone agent: serves the phone's dashboard and the PC's editor,
//! and carries out actions on the PC. Used by the command-line program in this
//! crate and by the desktop app, which wraps it in a window.

mod actions;
mod ai;
mod adb;
mod cli;
mod config;
mod discord;
mod editor;
mod http;
mod hub;
pub mod lang;
mod models;
mod pairing;
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
    /// Phones paired for Wi-Fi.
    pub pairing: pairing::Pairing,
}

impl App {
    /// The user's own images and fonts, next to the config file.
    pub fn assets_dir(&self) -> PathBuf {
        self.config_path.parent().unwrap_or(std::path::Path::new(".")).join("assets")
    }
}

/// How to start the agent.
pub struct Options {
    /// The config file; created with the defaults when missing.
    pub config: PathBuf,
    /// Overrides the port from the config.
    pub port: Option<u16>,
}

impl Default for Options {
    fn default() -> Self {
        Self { config: Config::default_path(), port: None }
    }
}

impl Options {
    /// Reads `--config <path>` and `--port <number>`.
    pub fn from_args(args: &[String]) -> Result<Self> {
        let mut options = Self::default();
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--config" => options.config = it.next().context("--config needs a path")?.into(),
                "--port" => options.port = Some(it.next().context("--port needs a number")?.parse().context("invalid --port")?),
                other => anyhow::bail!("unknown argument: {other}"),
            }
        }
        Ok(options)
    }
}

/// The language for texts on the PC (the tray menu): the one chosen in the
/// editor, or the system's.
pub fn language() -> &'static str {
    let saved = std::fs::read_to_string(Config::default_path()).ok().and_then(|text| serde_json::from_str::<Config>(&text).ok());
    saved.as_ref().map_or_else(lang::system, lang::chosen)
}

/// Help for the `hook` and `report` commands.
pub const CLI_HELP: &str = cli::HELP;

/// Runs `hook` or `report` if `args` asks for one of them: the one-shot
/// messengers to an agent that is already running. Returns whether it did.
pub fn run_cli(args: &[String]) -> bool {
    cli::run(args)
}

/// Offers the dashboard on the local network while Wi-Fi is switched on in
/// the config. It listens on the PC's private address only, never on every
/// interface, and follows the address when the network changes.
async fn wifi(app: Arc<App>) {
    let mut serving: Option<(Ipv4Addr, tokio::task::JoinHandle<()>)> = None;
    let mut complained = false;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let wanted = if app.config.read().unwrap().wireless() { pairing::lan_ip() } else { None };
        if serving.as_ref().map(|s| s.0) == wanted {
            continue;
        }
        if let Some((ip, task)) = serving.take() {
            task.abort();
            tracing::info!("Wi-Fi: no longer listening on {ip}");
        }
        let Some(ip) = wanted else { continue };
        match tokio::net::TcpListener::bind(SocketAddr::from((ip, app.port))).await {
            Ok(listener) => {
                tracing::info!("Wi-Fi: http://{ip}:{}/ (paired phones only)", app.port);
                complained = false;
                let router = server::router(app.clone());
                let task = tokio::spawn(async move {
                    let _ = axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await;
                });
                serving = Some((ip, task));
            }
            Err(err) if !complained => {
                complained = true;
                tracing::warn!("Wi-Fi: cannot listen on {ip}: {err}");
            }
            Err(_) => {}
        }
    }
}

/// Runs the agent until the process is interrupted. `ready` is told the
/// dashboard's port once it is being served; the editor is on the next one.
pub async fn serve(options: Options, ready: Option<tokio::sync::oneshot::Sender<u16>>) -> Result<()> {
    let config = Config::load(&options.config)?;
    let port = options.port.unwrap_or(config.port);
    tracing::info!("config: {}", options.config.display());

    let app = Arc::new(App {
        hub: Hub::new(),
        config: RwLock::new(config),
        config_path: options.config.clone(),
        port,
        art: Mutex::new(None),
        kick: Notify::new(),
        ai: ai::Ai::default(),
        discord: discord::Discord::default(),
        pairing: pairing::Pairing::new(options.config.parent().unwrap_or(std::path::Path::new("."))),
        tools: tools::Tools::new(options.config.parent().unwrap_or(std::path::Path::new("."))),
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
                if let Err(err) = axum::serve(editor_listener, router.into_make_service_with_connect_info::<SocketAddr>()).await {
                    tracing::warn!("editor stopped: {err}");
                }
            });
        }
        // Without its own editor the agent must not start: the desktop app
        // would otherwise show whatever other program holds that port.
        Err(err) => return Err(err).with_context(|| format!("port {editor_port} is in use (the editor needs it)")),
    }
    // Also on IPv6 loopback: Windows tries `localhost` there first and only
    // falls back to IPv4 after two seconds, which would stall every hook call.
    if let Ok(v6) = tokio::net::TcpListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, editor_port))).await {
        let router = editor::router(app.clone());
        tokio::spawn(async move {
            let _ = axum::serve(v6, router.into_make_service_with_connect_info::<SocketAddr>()).await;
        });
    }

    tokio::spawn(wifi(app.clone()));

    // Both ports are open: whoever started us may show the editor now.
    if let Some(ready) = ready {
        let _ = ready.send(port);
    }

    axum::serve(listener, server::router(app).into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
