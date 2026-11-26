//! HTTP + WebSocket server. Serves the phone UI and speaks the protocol.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast::error::RecvError;
use tokio::task::spawn_blocking;

use crate::protocol::{Action, ClientMsg, ServerMsg};
use crate::{actions, App};

#[derive(rust_embed::RustEmbed)]
#[folder = "../../ui/dist"]
struct Ui;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/ws", get(ws_upgrade))
        .route("/art/{id}", get(art))
        .route("/user/{*path}", get(user_file))
        .route("/discord/avatar/{key}", get(discord_avatar))
        .fallback(static_file)
        .with_state(app)
}

/// `localhost`, `127.0.0.1` or `[::1]`, with any port.
fn is_local_host(host: &str) -> bool {
    let name = match host.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => host.split(':').next().unwrap_or(""),
    };
    matches!(name, "localhost" | "127.0.0.1" | "::1")
}

/// The agent can launch programs, so a random website open in the PC's browser
/// must not be able to talk to it. Browsers always send `Origin` on WebSocket
/// handshakes: accept only pages served from this machine (the phone reaches us
/// as `localhost` through the USB port forward).
pub fn is_trusted(headers: &HeaderMap) -> bool {
    let value = |name| headers.get(name).and_then(|v: &header::HeaderValue| v.to_str().ok());
    if !value(header::HOST).is_some_and(is_local_host) {
        return false;
    }
    match value(header::ORIGIN) {
        None => true,
        Some(origin) => origin.strip_prefix("http://").is_some_and(is_local_host),
    }
}

async fn ws_upgrade(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    // From this machine (the PC's browser, or a phone on the USB port forward)
    // the page's origin decides. From the network, pairing does.
    let remote = !peer.ip().is_loopback();
    let allowed = if remote { app.config.read().unwrap().wireless() } else { is_trusted(&headers) };
    if !allowed {
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.on_upgrade(move |socket| client(socket, app, remote))
}

async fn client(socket: WebSocket, app: Arc<App>, remote: bool) {
    let (mut tx, mut rx) = socket.split();
    let mut bus = app.hub.listen();
    let mut topics: HashSet<String> = HashSet::new();

    // A phone on Wi-Fi has to prove it is paired before it gets anything.
    let device = if remote {
        // Each waiting handshake holds a connection for up to 20 seconds.
        let Some(_slot) = crate::pairing::Slot::take() else { return };
        match crate::pairing::admit(&app, &mut tx, &mut rx).await {
            Some(device) => Some(device),
            None => return,
        }
    } else {
        None
    };
    let mut recheck = tokio::time::interval(std::time::Duration::from_secs(3));

    let welcome = {
        let config = app.config.read().unwrap();
        serde_json::to_string(&ServerMsg::Welcome { version: env!("CARGO_PKG_VERSION"), config: &config })
    };
    let Ok(welcome) = welcome else { return };
    if tx.send(Message::Text(welcome.into())).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            // Unpairing a phone or switching Wi-Fi off cuts it loose at once.
            _ = recheck.tick() => {
                if let Some(device) = &device {
                    if !app.config.read().unwrap().wireless() || !app.pairing.knows(device) {
                        break;
                    }
                }
            }
            incoming = rx.next() => {
                let text = match incoming {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => continue,
                };
                let Ok(msg) = serde_json::from_str::<ClientMsg>(text.as_str()) else {
                    tracing::debug!("ignoring malformed message");
                    continue;
                };
                let mut replies: Vec<String> = Vec::new();
                match msg {
                    ClientMsg::Sub { topics: wanted } => {
                        for topic in wanted {
                            if topics.len() >= 64 || !topics.insert(topic.clone()) {
                                continue;
                            }
                            app.hub.add_subscriber(&topic);
                            if let Some(last) = app.hub.last(&topic) {
                                replies.push(last.to_string());
                            }
                        }
                    }
                    ClientMsg::Unsub { topics: unwanted } => {
                        for topic in unwanted {
                            if topics.remove(&topic) {
                                app.hub.remove_subscriber(&topic);
                            }
                        }
                    }
                    ClientMsg::Action { id, action } => {
                        let outcome = if action.config_only() {
                            Err("this action must be configured on the PC".to_owned())
                        } else {
                            execute(&app, action).await
                        };
                        replies.push(result(id, outcome));
                    }
                    ClientMsg::Press { id, widget, button } => {
                        let action = app.config.read().unwrap().button_action(&widget, button);
                        let outcome = match action {
                            Some(action) => execute(&app, action).await,
                            None => Err("unknown button".to_owned()),
                        };
                        replies.push(result(id, outcome));
                    }
                }
                for reply in replies {
                    if tx.send(Message::Text(reply.into())).await.is_err() {
                        break;
                    }
                }
            }
            event = bus.recv() => match event {
                Ok((topic, msg)) => {
                    if topics.contains(&*topic) && tx.send(Message::Text(msg.to_string().into())).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            },
        }
    }

    for topic in &topics {
        app.hub.remove_subscriber(topic);
    }
}

async fn execute(app: &Arc<App>, action: Action) -> Result<(), String> {
    tracing::info!("action: {action:?}");
    let outcome = match app.tools.handle(app, &action).or_else(|| app.ai.handle(app, &action)) {
        Some(outcome) => outcome,
        None => match spawn_blocking(move || actions::run(&action)).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => Err(format!("{err:#}")),
            Err(_) => Err("the action crashed".to_owned()),
        },
    };
    if let Err(err) = &outcome {
        tracing::warn!("action failed: {err}");
    }
    // Let the media/volume provider refresh immediately.
    app.kick.notify_one();
    outcome
}

fn result(id: u64, outcome: Result<(), String>) -> String {
    let msg = ServerMsg::Result { id, ok: outcome.is_ok(), error: outcome.err() };
    serde_json::to_string(&msg).unwrap_or_default()
}

async fn art(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let art = app.art.lock().unwrap().clone();
    match art {
        Some((current, bytes)) if current == id => {
            let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" };
            ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, "max-age=86400")], bytes).into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

/// A path inside the assets folder: plain names only, no `..`, drives or roots.
pub fn safe_relative(path: &str) -> Option<&std::path::Path> {
    let rel = std::path::Path::new(path);
    let plain = rel.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    (plain && !path.is_empty() && !path.contains(':')).then_some(rel)
}

/// `"image"` or `"font"` for files the assets folder may hold, by extension.
pub fn asset_kind(path: &str) -> Option<&'static str> {
    match mime_guess::from_path(path).first()?.type_().as_str() {
        "image" => Some("image"),
        "font" => Some("font"),
        _ => None,
    }
}

/// Wallpapers, button images and fonts from the user's assets folder.
pub async fn user_file(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(path): Path<String>,
) -> Response {
    // The host check guards requests from this machine against DNS rebinding;
    // a phone on Wi-Fi legitimately addresses the PC by its IP.
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok());
    if peer.ip().is_loopback() && !host.is_some_and(is_local_host) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(rel) = safe_relative(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if asset_kind(&path).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mime = mime_guess::from_path(rel).first_or_octet_stream();
    match tokio::fs::read(app.assets_dir().join(rel)).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, mime.as_ref().to_owned()),
                (header::CACHE_CONTROL, "no-cache".to_owned()),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
                // An SVG opened directly must not run scripts on this origin.
                (header::CONTENT_SECURITY_POLICY, "sandbox".to_owned()),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn discord_avatar(State(app): State<Arc<App>>, Path(key): Path<String>) -> Response {
    match app.discord.avatar(&key).await {
        Some((kind, bytes)) => ([(header::CONTENT_TYPE, kind), (header::CACHE_CONTROL, "max-age=86400".to_owned())], bytes).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn static_file(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    // The editor is not offered to the phone.
    if path == "editor.html" {
        return StatusCode::NOT_FOUND.into_response();
    }
    embedded(if path.is_empty() { "index.html" } else { path })
}

/// A file of the built UI.
pub fn embedded(path: &str) -> Response {
    match Ui::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            // Hashed asset names never change; everything else must stay fresh.
            let cache = if path.starts_with("assets/") { "max-age=31536000, immutable" } else { "no-cache" };
            (
                [(header::CONTENT_TYPE, mime.as_ref().to_owned()), (header::CACHE_CONTROL, cache.to_owned())],
                file.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(host: &str, origin: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::HOST, host.parse().unwrap());
        if let Some(origin) = origin {
            h.insert(header::ORIGIN, origin.parse().unwrap());
        }
        h
    }

    #[test]
    fn asset_paths_cannot_escape_the_folder() {
        assert!(safe_relative("wall.jpg").is_some());
        assert!(safe_relative("icons/obs.png").is_some());
        assert!(safe_relative("../config.json").is_none());
        assert!(safe_relative("icons/../../x.png").is_none());
        assert!(safe_relative("/etc/passwd").is_none());
        assert!(safe_relative("C:/Windows/x.png").is_none());
        assert!(safe_relative("").is_none());
    }

    #[test]
    fn only_local_pages_may_connect() {
        assert!(is_trusted(&headers("localhost:47800", Some("http://localhost:47800"))));
        assert!(is_trusted(&headers("127.0.0.1:47800", Some("http://localhost:5173"))));
        assert!(is_trusted(&headers("localhost:47800", None)));
        assert!(!is_trusted(&headers("localhost:47800", Some("https://evil.example"))));
        assert!(!is_trusted(&headers("localhost:47800", Some("http://localhost.evil.example"))));
        assert!(!is_trusted(&headers("evil.example", Some("http://localhost:47800"))));
    }
}
