//! Who is online on Discord.
//!
//! Discord does not let third-party apps read a user's friends list, so this
//! uses the one public source there is: a server's widget. When a server's
//! admin switches the widget on (Server Settings → Widget), Discord lists the
//! members who are online, without any login. For a group of friends on a
//! shared server that is exactly "who is online".

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::task::spawn_blocking;
use tokio::time::sleep;

use crate::App;

pub const SERVERS: &str = "discord.servers";

const AVATAR_HOST: &str = "https://cdn.discordapp.com/";
const MAX_AVATARS: usize = 300;
/// Discord refreshes the widget every minute or so; asking more often is pointless.
const EVERY: Duration = Duration::from_secs(30);

#[derive(Serialize, Clone, Default)]
struct Server {
    id: String,
    name: String,
    /// Members online, as Discord counts them (the list itself stops at 100).
    online: u64,
    members: Vec<Member>,
    /// Why there is no list: `disabled`, `unknown`, `offline`.
    #[serde(skip_serializing_if = "Option::is_none")]
    problem: Option<&'static str>,
}

#[derive(Serialize, Clone)]
struct Member {
    name: String,
    /// `online`, `idle` or `dnd`.
    status: String,
    /// What they are playing or doing, when Discord tells.
    #[serde(skip_serializing_if = "String::is_empty")]
    activity: String,
    /// Voice channel they sit in.
    #[serde(skip_serializing_if = "String::is_empty")]
    voice: String,
    muted: bool,
    /// Key for `/discord/avatar/{key}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar: Option<String>,
}

/// A picture: its media type and bytes.
type Picture = (String, Vec<u8>);

#[derive(Default)]
pub struct Discord {
    /// Avatar key → where to fetch it, and the picture once fetched.
    avatars: Mutex<HashMap<String, (String, Option<Picture>)>>,
    last: Mutex<HashMap<String, Server>>,
}

/// The API root; overridable so tests can point at a stand-in server.
fn api() -> String {
    std::env::var("STREAMPHONE_DISCORD_API").unwrap_or_else(|_| "https://discord.com/api".to_owned())
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(8)).user_agent("StreamPhone").build()
}

fn text(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").to_owned()
}

fn avatar_key(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Turns Discord's widget JSON into what the phone shows. Also returns the
/// avatar addresses found, keyed for the avatar route.
fn parse(id: &str, body: &Value) -> (Server, Vec<(String, String)>) {
    let channels: HashMap<String, String> = body
        .get("channels")
        .and_then(Value::as_array)
        .map(|list| list.iter().map(|c| (text(c, "id"), text(c, "name"))).collect())
        .unwrap_or_default();

    let mut avatars = Vec::new();
    let mut members: Vec<Member> = body
        .get("members")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|m| {
                    let url = text(m, "avatar_url");
                    let avatar = (!url.is_empty()).then(|| {
                        let key = avatar_key(&url);
                        avatars.push((key.clone(), url));
                        key
                    });
                    let flag = |key: &str| m.get(key).and_then(Value::as_bool).unwrap_or(false);
                    Member {
                        name: text(m, "username"),
                        status: text(m, "status"),
                        activity: m.get("game").or_else(|| m.get("activity")).map(|g| text(g, "name")).unwrap_or_default(),
                        voice: channels.get(&text(m, "channel_id")).cloned().unwrap_or_default(),
                        muted: flag("self_mute") || flag("mute") || flag("self_deaf") || flag("deaf"),
                        avatar,
                    }
                })
                .filter(|m| !m.name.is_empty())
                .collect()
        })
        .unwrap_or_default();
    members.sort_by_key(|m| m.name.to_lowercase());

    let server = Server {
        id: id.to_owned(),
        name: text(body, "name"),
        online: body.get("presence_count").and_then(Value::as_u64).unwrap_or(members.len() as u64),
        members,
        problem: None,
    };
    (server, avatars)
}

enum Fetched {
    Ok(Value),
    Problem(&'static str),
    /// Rate limited or a hiccup: keep showing what we had.
    Keep,
}

fn fetch(id: &str) -> Fetched {
    match agent().get(&format!("{}/guilds/{id}/widget.json", api())).call() {
        Ok(res) => res.into_json().map_or(Fetched::Keep, Fetched::Ok),
        Err(ureq::Error::Status(403, _)) => Fetched::Problem("disabled"),
        Err(ureq::Error::Status(404 | 400, _)) => Fetched::Problem("unknown"),
        Err(ureq::Error::Status(_, _)) => Fetched::Keep,
        Err(ureq::Error::Transport(_)) => Fetched::Problem("offline"),
    }
}

impl Discord {
    /// A member's picture. Only addresses Discord's own widget handed out are
    /// fetched, so this is not a way to make the agent fetch arbitrary pages.
    pub async fn avatar(&self, key: &str) -> Option<Picture> {
        let url = {
            let avatars = self.avatars.lock().unwrap();
            let (url, cached) = avatars.get(key)?;
            if let Some(found) = cached {
                return Some(found.clone());
            }
            url.clone()
        };
        let fetched = spawn_blocking(move || {
            let res = agent().get(&url).call().ok()?;
            let kind = res.content_type().to_owned();
            if !kind.starts_with("image/") {
                return None;
            }
            let mut bytes = Vec::new();
            use std::io::Read;
            res.into_reader().take(2 * 1024 * 1024).read_to_end(&mut bytes).ok()?;
            Some((kind, bytes))
        })
        .await
        .ok()??;
        if let Some(entry) = self.avatars.lock().unwrap().get_mut(key) {
            entry.1 = Some(fetched.clone());
        }
        Some(fetched)
    }

    fn remember(&self, found: Vec<(String, String)>) {
        let testing = std::env::var_os("STREAMPHONE_DISCORD_API").is_some();
        let mut avatars = self.avatars.lock().unwrap();
        if avatars.len() + found.len() > MAX_AVATARS {
            avatars.clear();
        }
        for (key, url) in found {
            if url.starts_with(AVATAR_HOST) || testing {
                avatars.entry(key).or_insert((url, None));
            }
        }
    }
}

pub fn spawn(app: Arc<App>) {
    tokio::spawn(async move {
        let mut due = Duration::ZERO;
        let mut asked: Vec<String> = Vec::new();
        loop {
            sleep(Duration::from_secs(1)).await;
            if !app.hub.has_subscribers(SERVERS) {
                due = Duration::ZERO;
                continue;
            }
            let ids = app.config.read().unwrap().discord_servers();
            // Ask again at once when the list of servers changes.
            if ids != asked {
                due = Duration::ZERO;
            }
            if due > Duration::ZERO {
                due = due.saturating_sub(Duration::from_secs(1));
                // The hub forgets idle topics; keep the last answer available.
                let last = app.discord.last.lock().unwrap();
                let servers: Vec<&Server> = ids.iter().filter_map(|id| last.get(id)).collect();
                app.hub.publish(SERVERS, &servers);
                continue;
            }
            due = EVERY;
            asked = ids.clone();

            for id in &ids {
                let wanted = id.clone();
                let Ok(fetched) = spawn_blocking(move || fetch(&wanted)).await else { continue };
                let server = match fetched {
                    Fetched::Ok(body) => {
                        let (server, avatars) = parse(id, &body);
                        app.discord.remember(avatars);
                        server
                    }
                    Fetched::Problem(problem) => Server { id: id.clone(), problem: Some(problem), ..Server::default() },
                    Fetched::Keep => continue,
                };
                app.discord.last.lock().unwrap().insert(id.clone(), server);
            }
            let last = app.discord.last.lock().unwrap();
            let servers: Vec<&Server> = ids.iter().filter_map(|id| last.get(id)).collect();
            app.hub.publish(SERVERS, &servers);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn widget_json_becomes_a_member_list() {
        let body = json!({
            "id": "1", "name": "Parta", "presence_count": 3,
            "channels": [{ "id": "10", "name": "Herna", "position": 0 }],
            "members": [
                { "id": "0", "username": "Zuzka", "status": "idle", "avatar_url": "https://cdn.discordapp.com/widget-avatars/a/b" },
                { "id": "1", "username": "adam", "status": "online", "game": { "name": "Factorio" }, "channel_id": "10", "self_mute": true,
                  "avatar_url": "https://cdn.discordapp.com/widget-avatars/c/d" },
                { "id": "2", "username": "", "status": "online" }
            ]
        });
        let (server, avatars) = parse("1", &body);
        assert_eq!((server.name.as_str(), server.online, server.members.len()), ("Parta", 3, 2));
        let adam = &server.members[0];
        assert_eq!((adam.name.as_str(), adam.activity.as_str(), adam.voice.as_str(), adam.muted), ("adam", "Factorio", "Herna", true));
        assert_eq!(avatars.len(), 2);
        assert_eq!(server.members[1].avatar.as_deref(), Some(avatar_key("https://cdn.discordapp.com/widget-avatars/a/b").as_str()));
    }
}
