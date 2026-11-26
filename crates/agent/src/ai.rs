//! Coding-agent sessions on the phone.
//!
//! Claude Code reports what it is doing through HTTP hooks; Codex and other
//! tools through the `hook` and `report` commands (see the README for the
//! settings). The agent keeps one entry per session and, when a session
//! asks for permission to use a tool, can hold the question until somebody
//! taps Allow or Deny on the phone. If nobody answers in time the hook returns
//! nothing and Claude Code asks in its own window as usual.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::oneshot;
use tokio::time::{sleep, timeout};

use crate::protocol::Action;
use crate::App;

pub const SESSIONS: &str = "ai.sessions";

/// A session that has said nothing for this long is forgotten.
const STALE: Duration = Duration::from_secs(12 * 3600);
const MAX_SESSIONS: usize = 24;

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    /// Open, nothing asked yet.
    Idle,
    Working,
    /// Needs the user: a permission prompt or a question.
    Waiting,
    /// Finished its answer.
    Done,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    id: String,
    /// Which tool the session belongs to: `claude`, `codex`…
    source: String,
    /// Name of the folder the session works in.
    project: String,
    title: String,
    status: Status,
    /// What it is doing or asking, in a line.
    detail: String,
    /// Milliseconds since the Unix epoch.
    updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    request: Option<Request>,
}

/// A permission question the phone may answer.
#[derive(Clone, Serialize)]
struct Request {
    id: u64,
    tool: String,
    detail: String,
}

#[derive(Default)]
struct Inner {
    sessions: HashMap<String, Session>,
    answers: HashMap<u64, oneshot::Sender<bool>>,
    /// Go up when a session finishes / asks, so the phone knows when to buzz.
    finished: u32,
    asked: u32,
}

#[derive(Default)]
pub struct Ai {
    inner: Mutex<Inner>,
    next_request: AtomicU64,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn shorten(text: &str, max: usize) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= max {
        return line;
    }
    let mut out: String = line.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Other tools name the same moments differently; map them onto Claude Code's names.
fn canonical(event: &str) -> &str {
    match event {
        "BeforeTool" | "beforeShellExecution" | "beforeMCPExecution" | "beforeReadFile" => "PreToolUse",
        "AfterTool" | "afterShellExecution" | "afterMCPExecution" | "afterFileEdit" => "PostToolUse",
        "BeforeAgent" | "beforeSubmitPrompt" => "UserPromptSubmit",
        "AfterAgent" | "stop" | "agent-turn-complete" => "Stop",
        "sessionStart" => "SessionStart",
        "sessionEnd" => "SessionEnd",
        other => other,
    }
}

fn tidy_source(source: &str) -> String {
    let name: String = source.chars().filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '.')).take(24).collect();
    if name.trim().is_empty() { "claude".to_owned() } else { name.trim().to_ascii_lowercase() }
}

/// What a tool call is about, in a line: the command, the file, the URL…
fn describe(input: &Value) -> String {
    for key in ["command", "file_path", "path", "url", "pattern", "query", "description", "prompt"] {
        if let Some(text) = input.get(key).and_then(Value::as_str) {
            return shorten(text, 140);
        }
    }
    String::new()
}

impl Ai {
    fn publish(&self, app: &App) {
        let inner = self.inner.lock().unwrap();
        let mut sessions: Vec<&Session> = inner.sessions.values().collect();
        sessions.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
        app.hub.publish(SESSIONS, &json!({ "sessions": sessions, "finished": inner.finished, "asked": inner.asked }));
    }

    /// Takes one hook event. Returns the JSON Claude Code should act on, or
    /// `None` when the agent has nothing to say about it.
    pub async fn hook(&self, app: &App, source: &str, event: Value) -> Option<Value> {
        let text = |key: &str| event.get(key).and_then(Value::as_str).unwrap_or("");
        let first = |keys: &[&str]| keys.iter().map(|k| text(k)).find(|v| !v.is_empty()).unwrap_or("").to_owned();
        let source = tidy_source(source);
        // Tools without session ids get one entry per tool.
        let raw_id = first(&["session_id", "conversation_id", "thread-id", "thread_id", "session"]);
        let id = format!("{source}:{}", if raw_id.is_empty() { "default" } else { &raw_id });
        let kind = canonical(&first(&["hook_event_name", "event", "type"])).to_owned();
        let kind = kind.as_str();
        let tool = first(&["tool_name", "tool"]);
        // Some tools put the command at the top level instead of in `tool_input`.
        let about = event.get("tool_input").map(describe).filter(|d| !d.is_empty()).unwrap_or_else(|| describe(&event));
        let cwd = match text("cwd") {
            "" => event.get("workspace_roots").and_then(|r| r.get(0)).and_then(Value::as_str).unwrap_or("").to_owned(),
            dir => dir.to_owned(),
        };
        let prompt = match text("prompt") {
            "" => event.get("input-messages").and_then(|m| m.get(0)).and_then(Value::as_str).unwrap_or("").to_owned(),
            p => p.to_owned(),
        };
        let last_message = first(&["last_assistant_message", "last-assistant-message"]);

        let mut ask: Option<(u64, oneshot::Receiver<bool>)> = None;
        {
            let mut inner = self.inner.lock().unwrap();
            if kind == "SessionEnd" {
                if let Some(old) = inner.sessions.remove(&id) {
                    if let Some(request) = old.request {
                        inner.answers.remove(&request.id);
                    }
                }
            } else {
                if !inner.sessions.contains_key(&id) && inner.sessions.len() >= MAX_SESSIONS {
                    let oldest = inner.sessions.values().min_by_key(|s| s.updated_at).map(|s| s.id.clone());
                    if let Some(oldest) = oldest {
                        inner.sessions.remove(&oldest);
                    }
                }
                let project = std::path::Path::new(&cwd)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut finished = false;
                let mut asked = false;
                let mut dropped: Option<u64> = None;
                let session = inner.sessions.entry(id.clone()).or_insert_with(|| Session {
                    id: id.clone(),
                    source: source.clone(),
                    project: String::new(),
                    title: String::new(),
                    status: Status::Idle,
                    detail: String::new(),
                    updated_at: 0,
                    request: None,
                });
                if !project.is_empty() {
                    session.project = project;
                }
                if !text("session_title").is_empty() {
                    session.title = shorten(text("session_title"), 80);
                }
                session.updated_at = now_ms();

                match kind {
                    "SessionStart" => session.status = Status::Idle,
                    "UserPromptSubmit" => {
                        session.status = Status::Working;
                        session.detail.clear();
                        if session.title.is_empty() {
                            session.title = shorten(&prompt, 80);
                        }
                    }
                    "PreToolUse" | "PostToolUse" | "PostToolUseFailure" => {
                        session.status = Status::Working;
                        session.detail = if about.is_empty() { tool.clone() } else { format!("{tool}: {about}") };
                        // The tool ran or was refused elsewhere: the question is settled.
                        dropped = session.request.take().map(|r| r.id);
                    }
                    "Notification" => {
                        if matches!(text("notification_type"), "permission_prompt" | "idle_prompt" | "agent_needs_input" | "elicitation_dialog") {
                            session.status = Status::Waiting;
                            session.detail = shorten(text("message"), 140);
                        }
                    }
                    "Stop" => {
                        session.status = Status::Done;
                        session.detail = shorten(&last_message, 140);
                        if session.title.is_empty() {
                            session.title = shorten(&prompt, 80);
                        }
                        dropped = session.request.take().map(|r| r.id);
                        finished = true;
                    }
                    "PermissionRequest" => {
                        session.status = Status::Waiting;
                        session.detail = if about.is_empty() { tool.clone() } else { format!("{tool}: {about}") };
                        dropped = session.request.take().map(|r| r.id);
                        // Hold the question only if a phone is there to answer it, and
                        // only for Claude Code, whose answer format is known.
                        if source == "claude" && app.config.read().unwrap().ai_wait().is_some() && app.hub.has_subscribers(SESSIONS) {
                            let request = self.next_request.fetch_add(1, Ordering::Relaxed) + 1;
                            let (tx, rx) = oneshot::channel();
                            session.request = Some(Request { id: request, tool: tool.clone(), detail: about.clone() });
                            ask = Some((request, rx));
                            inner.answers.insert(request, tx);
                            asked = true;
                        }
                    }
                    _ => {}
                }
                if let Some(old) = dropped {
                    inner.answers.remove(&old);
                }
                inner.finished += u32::from(finished);
                inner.asked += u32::from(asked);
            }
        }
        self.publish(app);

        let (request, answer) = ask?;
        let wait = app.config.read().unwrap().ai_wait().unwrap_or(45);
        let verdict = timeout(Duration::from_secs(wait), answer).await;

        {
            let mut inner = self.inner.lock().unwrap();
            inner.answers.remove(&request);
            if let Some(session) = inner.sessions.get_mut(&id) {
                if session.request.as_ref().is_some_and(|r| r.id == request) {
                    session.request = None;
                    if let Ok(Ok(allowed)) = verdict {
                        session.status = if allowed { Status::Working } else { Status::Waiting };
                        session.updated_at = now_ms();
                    }
                }
            }
        }
        self.publish(app);

        let decision = match verdict {
            Ok(Ok(true)) => json!({ "behavior": "allow" }),
            Ok(Ok(false)) => json!({ "behavior": "deny", "message": "Denied from the phone (StreamPhone)." }),
            // Nobody answered: Claude Code asks in its own window.
            _ => return None,
        };
        tracing::info!("permission for {tool} answered from the phone: {}", decision["behavior"]);
        Some(json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": decision } }))
    }

    /// A state reported by any tool through `streamphone-agent report` (or a
    /// POST to `/api/event`): `source`, `status`, and optionally `session`,
    /// `cwd`, `title`, `detail`. Codex's `notify` event may ride along as
    /// `payload`.
    pub async fn report(&self, app: &App, body: Value) {
        let text = |key: &str| body.get(key).and_then(Value::as_str).unwrap_or("").to_owned();
        let mut event = match body.get("payload") {
            Some(Value::Object(payload)) => payload.clone(),
            _ => serde_json::Map::new(),
        };
        let name = match text("status").as_str() {
            "working" => "UserPromptSubmit",
            "waiting" => "Notification",
            "idle" => "SessionStart",
            "end" => "SessionEnd",
            "done" => "Stop",
            // No status: a bare `notify` payload names its own event.
            _ if event.contains_key("type") || event.contains_key("hook_event_name") => "",
            _ => "Stop",
        };
        let has_cwd = event.contains_key("cwd");
        let mut set = |key: &str, value: String| {
            if !value.is_empty() {
                event.insert(key.to_owned(), Value::String(value));
            }
        };
        set("hook_event_name", name.to_owned());
        set("session_id", text("session"));
        // The tool's own folder beats the one the `report` command happened to run in.
        if !has_cwd {
            set("cwd", text("cwd"));
        }
        set("session_title", text("title"));
        match name {
            "Notification" => {
                set("notification_type", "agent_needs_input".to_owned());
                set("message", text("detail"));
            }
            "Stop" | "" => set("last_assistant_message", text("detail")),
            _ => {}
        }
        self.hook(app, &text("source"), Value::Object(event)).await;
    }

    /// The phone's answer to a permission question.
    pub fn handle(&self, app: &App, action: &Action) -> Option<Result<(), String>> {
        let Action::AiDecide { request, allow } = action else { return None };
        if app.config.read().unwrap().ai_wait().is_none() {
            return Some(Err("answering from the phone is turned off in the config".to_owned()));
        }
        let answer = self.inner.lock().unwrap().answers.remove(request);
        Some(match answer {
            Some(tx) => tx.send(*allow).map_err(|_| "the question is no longer open".to_owned()),
            None => Err("the question is no longer open".to_owned()),
        })
    }
}

pub fn spawn(app: Arc<App>) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(1)).await;
            {
                let limit = now_ms().saturating_sub(STALE.as_millis() as u64);
                app.ai.inner.lock().unwrap().sessions.retain(|_, s| s.updated_at >= limit || s.request.is_some());
            }
            // The hub forgets topics nobody listens to; keep it fed while somebody does.
            if app.hub.has_subscribers(SESSIONS) {
                app.ai.publish(&app);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_calls_are_described_in_a_line() {
        assert_eq!(describe(&json!({ "command": "npm   test\n--watch" })), "npm test --watch");
        assert_eq!(describe(&json!({ "file_path": "C:\\a\\b.rs", "content": "x" })), "C:\\a\\b.rs");
        assert_eq!(describe(&json!({ "nothing": 1 })), "");
        assert_eq!(shorten(&"x".repeat(200), 10).chars().count(), 10);
    }
}
