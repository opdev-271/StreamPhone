//! `streamphone-agent hook` and `streamphone-agent report`: how tools that can
//! only run a program (Codex, scripts, other coding agents) tell the running
//! agent what they are doing.
//!
//! Both commands always exit successfully and stay quiet when the agent is not
//! running, so a tool that calls them is never held up or broken by them.

use std::io::Read;
use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::config::Config;
use crate::http;

pub const HELP: &str = "\
streamphone-agent hook [--source NAME] [--port N]
    Reads one hook event (JSON) from standard input, as Claude Code, Codex and
    similar tools provide it, and passes it to the running agent. Prints the
    agent's answer, if any.

streamphone-agent report --source NAME [--status working|waiting|done|idle|end]
                         [--title TEXT] [--detail TEXT] [--session ID] [--cwd DIR]
                         [--port N] [JSON]
    Reports a state from any tool or script. A trailing JSON argument (as
    Codex's `notify` passes) is sent along and understood for known tools.";

/// The agent's dashboard port: `--port`, else the config file, else the default.
fn port(given: Option<u16>) -> u16 {
    given
        .or_else(|| {
            let text = std::fs::read_to_string(Config::default_path()).ok()?;
            serde_json::from_str::<Value>(&text).ok()?.get("port")?.as_u64().map(|p| p as u16)
        })
        .unwrap_or(47800)
}

fn post(port: u16, path: &str, body: &Value, wait: Duration) -> Option<String> {
    // The editor port, which only this PC can reach.
    let url = format!("http://127.0.0.1:{}{path}", port.wrapping_add(1));
    let res = http::request("POST", &url, Some(body.to_string().as_bytes()), Duration::from_millis(250), wait).ok()?;
    (res.status == 200).then(|| String::from_utf8_lossy(&res.body).into_owned())
}

/// Runs the command if `args` is one of ours. Returns whether it was.
pub fn run(args: &[String]) -> bool {
    let Some(command) = args.first().map(String::as_str).filter(|c| matches!(*c, "hook" | "report")) else {
        return false;
    };

    let mut fields: Map<String, Value> = Map::new();
    let mut given_port = None;
    let mut it = args[1..].iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--port" => given_port = it.next().and_then(|p| p.parse().ok()),
            "--source" | "--status" | "--title" | "--detail" | "--session" | "--cwd" => {
                if let Some(value) = it.next() {
                    fields.insert(arg[2..].to_owned(), Value::String(value.clone()));
                }
            }
            // Codex's `notify` hands its event over as one JSON argument.
            other => {
                if let Ok(payload @ Value::Object(_)) = serde_json::from_str::<Value>(other) {
                    fields.insert("payload".to_owned(), payload);
                }
            }
        }
    }
    let source = fields.get("source").and_then(Value::as_str).unwrap_or("").to_owned();

    if command == "hook" {
        let mut text = String::new();
        let _ = std::io::stdin().read_to_string(&mut text);
        let Ok(event @ Value::Object(_)) = serde_json::from_str::<Value>(&text) else { return true };
        let source = if source.is_empty() { "claude".to_owned() } else { source };
        let body = json!({ "source": source, "event": event });
        // A permission question may wait for the phone for minutes.
        if let Some(answer) = post(port(given_port), "/api/hook", &body, Duration::from_secs(320)) {
            print!("{answer}");
        }
    } else {
        if !fields.contains_key("cwd") {
            if let Ok(dir) = std::env::current_dir() {
                fields.insert("cwd".to_owned(), Value::String(dir.to_string_lossy().into_owned()));
            }
        }
        post(port(given_port), "/api/event", &Value::Object(fields), Duration::from_secs(5));
    }
    true
}
