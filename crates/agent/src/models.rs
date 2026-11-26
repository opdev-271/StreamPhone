//! Local model servers: which ones are running and what they have loaded.
//!
//! Understands LM Studio, Ollama and anything that answers the
//! OpenAI-compatible `GET /v1/models` (llama.cpp's server, Jan, KoboldCpp,
//! LocalAI, vLLM, text-generation-webui…).

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::task::spawn_blocking;
use tokio::time::sleep;

use crate::http::get_json;
use crate::App;

pub const MODELS: &str = "ai.models";

const WAIT: Duration = Duration::from_secs(3);
const MAX_LISTED: usize = 12;

#[derive(Serialize, Default)]
struct Server {
    name: String,
    kind: String,
    online: bool,
    /// Models in memory (for OpenAI-compatible servers: models on offer).
    models: Vec<Model>,
    /// Models on disk, where the server tells.
    installed: Option<usize>,
}

#[derive(Serialize)]
struct Model {
    name: String,
    /// Size, quantization or context, whatever the server tells.
    info: String,
}

fn gigabytes(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

/// LM Studio 0.4+: `GET /api/v1/models`; a model is loaded when it has instances.
fn lmstudio(url: &str) -> Option<(Vec<Model>, Option<usize>)> {
    if let Ok(body) = get_json(&format!("{url}/api/v1/models"), WAIT) {
        let all = body.get("models")?.as_array()?;
        let loaded = all
            .iter()
            .filter(|m| m.get("loaded_instances").and_then(Value::as_array).is_some_and(|i| !i.is_empty()))
            .map(|m| {
                let name = match text(m, "display_name") {
                    "" => text(m, "key"),
                    shown => shown,
                };
                let quant = m.get("quantization").map(|q| text(q, "name")).unwrap_or("");
                let size = m.get("size_bytes").and_then(Value::as_u64).map(gigabytes).unwrap_or_default();
                Model { name: name.to_owned(), info: [quant, size.as_str()].iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(" · ") }
            })
            .collect();
        return Some((loaded, Some(all.len())));
    }
    // Older LM Studio: `GET /api/v0/models` with a `state` per model.
    let body = get_json(&format!("{url}/api/v0/models"), WAIT).ok()?;
    let all = body.get("data")?.as_array()?;
    let loaded = all
        .iter()
        .filter(|m| text(m, "state") == "loaded")
        .map(|m| Model { name: text(m, "id").to_owned(), info: text(m, "quantization").to_owned() })
        .collect();
    Some((loaded, Some(all.len())))
}

/// Ollama: `GET /api/ps` lists what is in memory, `GET /api/tags` what is on disk.
fn ollama(url: &str) -> Option<(Vec<Model>, Option<usize>)> {
    let running = get_json(&format!("{url}/api/ps"), WAIT).ok()?;
    let loaded = running
        .get("models")?
        .as_array()?
        .iter()
        .map(|m| {
            let quant = m.get("details").map(|d| text(d, "quantization_level")).unwrap_or("");
            let size = m.get("size").and_then(Value::as_u64).map(gigabytes).unwrap_or_default();
            Model { name: text(m, "name").to_owned(), info: [quant, size.as_str()].iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(" · ") }
        })
        .collect();
    let installed = get_json(&format!("{url}/api/tags"), WAIT).ok().and_then(|t| Some(t.get("models")?.as_array()?.len()));
    Some((loaded, installed))
}

/// Anything OpenAI-compatible: `GET /v1/models`.
fn openai(url: &str) -> Option<(Vec<Model>, Option<usize>)> {
    let body = get_json(&format!("{url}/v1/models"), WAIT).ok()?;
    let models = body
        .get("data")?
        .as_array()?
        .iter()
        .map(|m| Model { name: text(m, "id").to_owned(), info: String::new() })
        .filter(|m| !m.name.is_empty())
        .collect();
    Some((models, None))
}

fn probe(name: String, url: String, kind: String) -> Server {
    let found = match kind.as_str() {
        "lmstudio" => lmstudio(&url).or_else(|| openai(&url)),
        "ollama" => ollama(&url),
        _ => openai(&url),
    };
    match found {
        Some((mut models, installed)) => {
            models.truncate(MAX_LISTED);
            Server { name, kind, online: true, models, installed }
        }
        None => Server { name, kind, ..Server::default() },
    }
}

pub fn spawn(app: Arc<App>) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(4)).await;
            if !app.hub.has_subscribers(MODELS) {
                continue;
            }
            let servers = app.config.read().unwrap().model_servers();
            let mut found = Vec::with_capacity(servers.len());
            for (name, url, kind) in servers {
                if let Ok(server) = spawn_blocking(move || probe(name, url, kind)).await {
                    found.push(server);
                }
            }
            app.hub.publish(MODELS, &found);
        }
    });
}
