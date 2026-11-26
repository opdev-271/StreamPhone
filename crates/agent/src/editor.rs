//! The editor: a page on the PC for changing the look and the buttons.
//!
//! It lives on its own port, which is never forwarded to the phone. The phone
//! can therefore still only press buttons; what a button launches can only be
//! changed from the PC itself.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::{StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::config::Config;
use crate::{server, App};

/// Largest wallpaper or font that can be uploaded.
const MAX_UPLOAD: usize = 25 * 1024 * 1024;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/config", get(read_config).put(write_config))
        .route("/api/reset", post(reset_config))
        .route("/api/hook", post(hook))
        .route("/api/event", post(event))
        .route("/api/info", get(info))
        .route("/api/assets", get(list_assets))
        .route("/api/assets/{*path}", put(upload_asset).delete(delete_asset))
        .route("/user/{*path}", get(server::user_file))
        .fallback(page)
        .layer(DefaultBodyLimit::max(MAX_UPLOAD))
        .layer(middleware::from_fn(local_only))
        .with_state(app)
}

/// Only pages served by this machine may use the editor: a website open in the
/// PC's browser must not be able to rewrite what the buttons launch.
async fn local_only(request: Request, next: Next) -> Response {
    if !server::is_trusted(request.headers()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(request).await
}

fn failure(status: StatusCode, err: impl std::fmt::Display) -> Response {
    (status, Json(json!({ "error": err.to_string() }))).into_response()
}

async fn read_config(State(app): State<Arc<App>>) -> Response {
    Json(app.config.read().unwrap().clone()).into_response()
}

async fn write_config(State(app): State<Arc<App>>, Json(body): Json<Value>) -> Response {
    let config: Config = match serde_json::from_value(body) {
        Ok(config) => config,
        Err(err) => return failure(StatusCode::BAD_REQUEST, err),
    };
    match crate::config::save(&app, config) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => failure(StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}")),
    }
}

async fn reset_config(State(app): State<Arc<App>>) -> Response {
    let mut config = Config::built_in();
    // The port is part of how the agent was started, not of the look.
    config.port = app.config.read().unwrap().port;
    match crate::config::save(&app, config.clone()) {
        Ok(()) => Json(config).into_response(),
        Err(err) => failure(StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}")),
    }
}

/// Claude Code's HTTP hooks post here. Being on this port, the endpoint is
/// reachable from the PC only: the phone cannot fake a session or a question.
///
/// Two shapes arrive: Claude Code posts the event itself; the `hook` command
/// wraps it as `{ "source": …, "event": … }` to say which tool it came from.
async fn hook(State(app): State<Arc<App>>, Json(mut body): Json<Value>) -> Response {
    let (source, event) = match body.get_mut("event").map(Value::take) {
        Some(event @ Value::Object(_)) => (body.get("source").and_then(Value::as_str).unwrap_or("claude").to_owned(), event),
        _ => ("claude".to_owned(), body),
    };
    match app.ai.hook(&app, &source, event).await {
        Some(answer) => Json(answer).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

/// A state reported by any tool or script (`streamphone-agent report`).
async fn event(State(app): State<Arc<App>>, Json(body): Json<Value>) -> Response {
    app.ai.report(&app, body).await;
    StatusCode::NO_CONTENT.into_response()
}

/// Where the agent lives, for the setup snippets the editor shows.
async fn info() -> Response {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    Json(json!({ "version": env!("CARGO_PKG_VERSION"), "exe": exe })).into_response()
}

/// Files in the assets folder, one level of subfolders deep.
async fn list_assets(State(app): State<Arc<App>>) -> Response {
    let root = app.assets_dir();
    let mut files: Vec<Value> = Vec::new();
    let mut pending = vec![(root.clone(), 0)];
    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth < 1 {
                    pending.push((path, depth + 1));
                }
                continue;
            }
            let Ok(rel) = path.strip_prefix(&root) else { continue };
            let name = rel.to_string_lossy().replace('\\', "/");
            let Some(kind) = server::asset_kind(&name) else { continue };
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            files.push(json!({ "name": name, "kind": kind, "size": size }));
        }
    }
    files.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Json(files).into_response()
}

async fn upload_asset(State(app): State<Arc<App>>, Path(path): Path<String>, body: Bytes) -> Response {
    let Some(rel) = server::safe_relative(&path).filter(|_| server::asset_kind(&path).is_some()) else {
        return failure(StatusCode::BAD_REQUEST, "only images and fonts can be stored");
    };
    let target = app.assets_dir().join(rel);
    let written = async {
        if let Some(dir) = target.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        tokio::fs::write(&target, &body).await
    };
    match written.await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => failure(StatusCode::INTERNAL_SERVER_ERROR, err),
    }
}

async fn delete_asset(State(app): State<Arc<App>>, Path(path): Path<String>) -> Response {
    let Some(rel) = server::safe_relative(&path).filter(|_| server::asset_kind(&path).is_some()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match tokio::fs::remove_file(app.assets_dir().join(rel)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn page(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    server::embedded(if path.is_empty() { "editor.html" } else { path })
}
