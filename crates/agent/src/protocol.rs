//! Wire format between the agent and the phone UI (JSON over WebSocket).
//! Keep in sync with `ui/src/lib/types.ts`.

use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ClientMsg {
    Sub { topics: Vec<String> },
    Unsub { topics: Vec<String> },
    /// A built-in action sent directly by a widget (media, volume…).
    Action { id: u64, action: Action },
    /// A deck button press; the action is looked up in the agent's config.
    Press { id: u64, widget: String, button: usize },
}

#[derive(Debug, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ServerMsg<'a> {
    Welcome { version: &'a str, config: &'a Config },
    Result { id: u64, ok: bool, #[serde(skip_serializing_if = "Option::is_none")] error: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "do")]
pub enum Action {
    #[serde(rename = "launch")]
    Launch {
        target: String,
        #[serde(default)]
        args: Option<String>,
    },
    #[serde(rename = "keys")]
    Keys { combo: String },
    #[serde(rename = "media.play_pause")]
    MediaPlayPause,
    #[serde(rename = "media.next")]
    MediaNext,
    #[serde(rename = "media.prev")]
    MediaPrev,
    #[serde(rename = "audio.mute_toggle")]
    MuteToggle,
    #[serde(rename = "audio.set_volume")]
    SetVolume { level: f32 },
    #[serde(rename = "system.lock")]
    Lock,
    #[serde(rename = "display.off")]
    DisplayOff,
    /// Starts or pauses the focus timer; lengths are in minutes.
    #[serde(rename = "timer.toggle")]
    TimerToggle {
        #[serde(default)]
        focus: Option<u32>,
        #[serde(default)]
        rest: Option<u32>,
    },
    #[serde(rename = "timer.reset")]
    TimerReset,
    #[serde(rename = "timer.skip")]
    TimerSkip,
    #[serde(rename = "todo.add")]
    TodoAdd { text: String },
    #[serde(rename = "todo.toggle")]
    TodoToggle { id: u64 },
    #[serde(rename = "todo.remove")]
    TodoRemove { id: u64 },
    #[serde(rename = "todo.clear_done")]
    TodoClearDone,
    /// Ends every process of that name, if the process list showed it.
    #[serde(rename = "process.kill")]
    ProcessKill { name: String },
    /// Answers a Claude Code permission question shown on the phone.
    #[serde(rename = "ai.decide")]
    AiDecide { request: u64, allow: bool },
}

impl Action {
    /// Actions that run something chosen by their parameters. The phone may not
    /// send these directly: they must come from the config on the PC.
    pub fn config_only(&self) -> bool {
        matches!(self, Action::Launch { .. } | Action::Keys { .. })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    /// Bytes per second.
    pub net_rx: u64,
    pub net_tx: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaNow {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub app: String,
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    /// Id of the cover image, served at `/art/{id}`.
    pub art: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct Volume {
    pub level: f32,
    pub muted: bool,
}
