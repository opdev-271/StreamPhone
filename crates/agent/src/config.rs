use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::Action;
use crate::App;

/// Topic on which a changed config is pushed to the phones.
pub const TOPIC: &str = "agent.config";

/// Settings only the UI cares about (themes, backgrounds, styles…). The agent
/// passes them through untouched.
type Extra = serde_json::Map<String, Value>;

const DEFAULT_CONFIG: &str = include_str!("../default-config.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    /// Open the dashboard in the phone's browser when it is plugged in.
    #[serde(default = "default_true")]
    pub open_on_connect: bool,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub widgets: Vec<Widget>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    /// Widget-specific settings; the UI and the agent each read what they need.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub config: Value,
    #[serde(flatten)]
    pub extra: Extra,
}

fn default_port() -> u16 {
    47800
}

fn default_true() -> bool {
    true
}

impl Config {
    /// `%APPDATA%\StreamPhone\config.json` (or the platform equivalent).
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("StreamPhone")
            .join("config.json")
    }

    /// The config a fresh install starts with.
    pub fn built_in() -> Self {
        serde_json::from_str(DEFAULT_CONFIG).expect("the built-in config is valid")
    }

    /// Loads the config, writing the built-in default on first run.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
            }
            // A fresh install gets its pages and buttons named in the system's language.
            let mut fresh = Self::built_in();
            crate::lang::localize(&mut fresh, crate::lang::system());
            let text = serde_json::to_string_pretty(&fresh)?;
            std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        }
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    /// Ending processes from the phone is allowed while a process list is on
    /// some page and has not switched it off (`"kill": false`).
    pub fn allows_kill(&self) -> bool {
        self.pages
            .iter()
            .flat_map(|p| &p.widgets)
            .any(|w| w.kind == "processes" && w.config.get("kill").and_then(Value::as_bool) != Some(false))
    }

    /// How many seconds a Claude Code permission question waits for an answer
    /// from the phone. `None` when no AI widget is on any page or it has
    /// answering switched off (`"approve": false`): then the phone only watches.
    pub fn ai_wait(&self) -> Option<u64> {
        self.pages
            .iter()
            .flat_map(|p| &p.widgets)
            .filter(|w| w.kind == "ai" && w.config.get("approve").and_then(Value::as_bool) != Some(false))
            .map(|w| w.config.get("wait").and_then(Value::as_u64).unwrap_or(45).clamp(5, 300))
            .max()
    }

    /// Local model servers to watch: those listed on the `models` widgets, or
    /// LM Studio and Ollama at their usual addresses when a widget lists none.
    pub fn model_servers(&self) -> Vec<(String, String, String)> {
        let mut servers: Vec<(String, String, String)> = Vec::new();
        for widget in self.pages.iter().flat_map(|p| &p.widgets).filter(|w| w.kind == "models") {
            match widget.config.get("servers").and_then(Value::as_array) {
                Some(list) => {
                    for server in list {
                        let field = |key: &str| server.get(key).and_then(Value::as_str).unwrap_or("").trim().to_owned();
                        let url = field("url").trim_end_matches('/').to_owned();
                        if url.is_empty() {
                            continue;
                        }
                        let kind = match field("kind").as_str() {
                            "" => "openai".to_owned(),
                            kind => kind.to_owned(),
                        };
                        let name = if field("name").is_empty() { url.clone() } else { field("name") };
                        servers.push((name, url, kind));
                    }
                }
                None => {
                    servers.push(("LM Studio".to_owned(), "http://127.0.0.1:1234".to_owned(), "lmstudio".to_owned()));
                    servers.push(("Ollama".to_owned(), "http://127.0.0.1:11434".to_owned(), "ollama".to_owned()));
                }
            }
        }
        servers.sort();
        servers.dedup_by(|a, b| a.1 == b.1);
        servers.truncate(8);
        servers
    }

    /// Ids of the Discord servers listed on `discord` widgets.
    pub fn discord_servers(&self) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        for widget in self.pages.iter().flat_map(|p| &p.widgets).filter(|w| w.kind == "discord") {
            for id in widget.config.get("servers").and_then(Value::as_array).into_iter().flatten() {
                // Server ids are plain numbers; anything else is not sent anywhere.
                let id = id.as_str().unwrap_or("").trim();
                if (15..=22).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit()) && !ids.iter().any(|known| known == id) {
                    ids.push(id.to_owned());
                }
            }
        }
        ids.truncate(5);
        ids
    }

    /// Whether phones may connect over Wi-Fi (after pairing).
    pub fn wireless(&self) -> bool {
        self.extra.get("wireless").and_then(Value::as_bool) == Some(true)
    }

    /// The action behind a deck button. Launch targets and key combos only ever
    /// come from here, never from the phone.
    pub fn button_action(&self, widget_id: &str, button: usize) -> Option<Action> {
        let widget = self.pages.iter().flat_map(|p| &p.widgets).find(|w| w.id == widget_id)?;
        let action = widget.config.get("buttons")?.get(button)?.get("action")?;
        serde_json::from_value(action.clone()).ok()
    }
}

/// Stores a config made in the editor and shows it on the phones right away.
/// The file as it was before the editor first touched it is kept as
/// `config.backup.json`.
pub fn save(app: &App, config: Config) -> Result<()> {
    let path = &app.config_path;
    let backup = path.with_extension("backup.json");
    if path.exists() && !backup.exists() {
        std::fs::copy(path, &backup).with_context(|| format!("writing {}", backup.display()))?;
    }
    let text = serde_json::to_string_pretty(&config)?;
    // Write beside the file and swap, so a crash never leaves half a config.
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text).with_context(|| format!("writing {}", temp.display()))?;
    std::fs::rename(&temp, path).with_context(|| format!("replacing {}", path.display()))?;
    app.hub.publish(TOPIC, &config);
    *app.config.write().unwrap() = config;
    Ok(())
}

/// Reloads the config whenever the file changes and pushes it to the phones,
/// so editing themes or buttons shows up without restarting anything. A file
/// that does not parse is ignored and the previous config stays in effect.
pub async fn watch(app: Arc<App>) {
    let modified = |path: &Path| std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let mut seen = modified(&app.config_path);
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let now = modified(&app.config_path);
        if now == seen || now.is_none() {
            continue;
        }
        seen = now;
        match Config::load(&app.config_path) {
            Ok(config) => {
                tracing::info!("config reloaded");
                app.hub.publish(TOPIC, &config);
                *app.config.write().unwrap() = config;
            }
            Err(err) => tracing::warn!("config not reloaded: {err:#}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_parses_and_resolves_buttons() {
        let cfg: Config = serde_json::from_str(DEFAULT_CONFIG).unwrap();
        assert!(!cfg.pages.is_empty());
        assert!(matches!(cfg.button_action("home-deck", 0), Some(Action::Launch { .. })));
        assert!(matches!(cfg.button_action("deck-main", 6), Some(Action::Keys { .. })));
        assert!(cfg.button_action("home-deck", 99).is_none());
        assert!(cfg.button_action("nope", 0).is_none());
    }

    #[test]
    fn ui_only_settings_pass_through() {
        let text = r##"{"theme":{"preset":"ocean"},"pages":[{"id":"a","name":"A","background":{"image":"x.jpg"},
            "widgets":[{"id":"w","type":"clock","x":0,"y":0,"w":1,"h":1,"style":{"accent":"#fff"}}]}]}"##;
        let cfg: Config = serde_json::from_str(text).unwrap();
        let out = serde_json::to_value(&cfg).unwrap();
        assert_eq!(out["theme"]["preset"], "ocean");
        assert_eq!(out["pages"][0]["background"]["image"], "x.jpg");
        assert_eq!(out["pages"][0]["widgets"][0]["style"]["accent"], "#fff");
    }
}
