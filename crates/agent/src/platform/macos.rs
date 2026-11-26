//! macOS, through the tools every Mac ships with: `osascript` (AppleScript),
//! `open` and `pmset`. No extra libraries, at the price of a short process
//! start per call.
//!
//! Permissions macOS will ask for the first time each is needed:
//! - Automation, to read and control Spotify or Music;
//! - Accessibility, to send key combinations (and to lock the screen).
//!
//! The parts that build scripts and read their answers are plain text
//! handling, so they are compiled and tested on every platform.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use super::MediaCmd;
use crate::protocol::{MediaNow, Volume};

/// Separates the fields a script returns; cannot occur in a track title.
const SEP: char = '\u{1f}';

/// Players we know how to talk to, in order of preference.
const PLAYERS: [&str; 2] = ["Spotify", "Music"];

fn osascript(lines: &[&str]) -> Result<String> {
    let mut command = Command::new("osascript");
    for line in lines {
        command.arg("-e").arg(line);
    }
    let out = command.output().context("running osascript")?;
    if !out.status.success() {
        let problem = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        if problem.contains("1002") || problem.contains("not allowed") {
            bail!("macOS blocked it: allow StreamPhone under System Settings → Privacy & Security → Accessibility");
        }
        if problem.contains("-1743") {
            bail!("macOS blocked it: allow StreamPhone under System Settings → Privacy & Security → Automation");
        }
        bail!("{problem}");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end_matches(['\n', '\r']).to_owned())
}

/// Text as an AppleScript string literal.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn is_running(app: &str) -> bool {
    osascript(&[&format!("application {} is running", quoted(app))]).is_ok_and(|answer| answer == "true")
}

fn player() -> Option<&'static str> {
    PLAYERS.into_iter().find(|app| is_running(app))
}

/// The script that asks a player what it is playing. Spotify reports the
/// length in milliseconds, Music in seconds; both are returned as seconds.
fn now_playing_script(app: &str) -> Vec<String> {
    let seconds = if app == "Spotify" { "((duration of current track) / 1000)" } else { "(duration of current track)" };
    vec![
        "set sep to ASCII character 31".to_owned(),
        format!("tell application {}", quoted(app)),
        "if player state is stopped then return \"\"".to_owned(),
        format!(
            "return (name of current track) & sep & (artist of current track) & sep & (album of current track) & sep & (player state as text) & sep & (player position as text) & sep & ({seconds} as text)"
        ),
        "end tell".to_owned(),
    ]
}

/// A number as AppleScript prints it: the decimal mark follows the system
/// language, so "187,5" and "187.5" both occur.
fn number(text: &str) -> f64 {
    text.trim().replace(',', ".").parse().unwrap_or(0.0)
}

fn parse_now_playing(app: &str, answer: &str) -> Option<MediaNow> {
    let fields: Vec<&str> = answer.split(SEP).collect();
    let [title, artist, album, state, position, duration] = fields[..] else { return None };
    Some(MediaNow {
        title: title.to_owned(),
        artist: artist.to_owned(),
        album: album.to_owned(),
        app: app.to_owned(),
        playing: state.trim() == "playing",
        position_ms: (number(position) * 1000.0).max(0.0) as u64,
        duration_ms: (number(duration) * 1000.0).max(0.0) as u64,
        art: None,
    })
}

pub fn media_now() -> Result<Option<MediaNow>> {
    let Some(app) = player() else { return Ok(None) };
    let script = now_playing_script(app);
    let lines: Vec<&str> = script.iter().map(String::as_str).collect();
    Ok(parse_now_playing(app, &osascript(&lines)?))
}

/// Cover art is not available through AppleScript without writing files.
pub fn media_art() -> Result<Option<Vec<u8>>> {
    Ok(None)
}

pub fn media_command(cmd: MediaCmd) -> Result<()> {
    let app = player().ok_or_else(|| anyhow!("neither Spotify nor Music is running"))?;
    let verb = match cmd {
        MediaCmd::PlayPause => "playpause",
        MediaCmd::Next => "next track",
        MediaCmd::Prev => "previous track",
    };
    osascript(&[&format!("tell application {} to {verb}", quoted(app))]).map(drop)
}

/// "50,false" → half volume, not muted. Some outputs (HDMI, some DACs) have
/// no software volume and answer "missing value".
fn parse_volume(answer: &str) -> Result<Volume> {
    let (level, muted) = answer.split_once(',').ok_or_else(|| anyhow!("unexpected volume answer: {answer}"))?;
    let level: f32 = level.trim().parse().map_err(|_| anyhow!("this audio output has no volume control"))?;
    Ok(Volume { level: (level / 100.0).clamp(0.0, 1.0), muted: muted.trim() == "true" })
}

pub fn volume() -> Result<Volume> {
    parse_volume(&osascript(&[
        "set v to get volume settings",
        "return (output volume of v as text) & \",\" & (output muted of v as text)",
    ])?)
}

pub fn set_volume(level: f32) -> Result<()> {
    let percent = (level.clamp(0.0, 1.0) * 100.0).round() as u32;
    osascript(&[&format!("set volume output volume {percent}")]).map(drop)
}

pub fn toggle_mute() -> Result<()> {
    osascript(&["set volume output muted not (output muted of (get volume settings))"]).map(drop)
}

/// How `open` should be called for a launch target: an app by name, or a
/// file, folder or link.
fn open_arguments(target: &str, args: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    // "Safari" or "Visual Studio Code": no path, no scheme, so an app's name.
    let by_name = !target.contains('/') && !target.contains(':') && !target.starts_with('~');
    if by_name {
        out.push("-a".to_owned());
    }
    // `open` does not expand "~" itself (no shell is involved).
    match (target.strip_prefix("~/"), std::env::var("HOME")) {
        (Some(rest), Ok(home)) => out.push(format!("{home}/{rest}")),
        _ => out.push(target.to_owned()),
    }
    if let Some(args) = args.filter(|a| !a.trim().is_empty()) {
        out.push("--args".to_owned());
        out.extend(args.split_whitespace().map(str::to_owned));
    }
    out
}

pub fn launch(target: &str, args: Option<&str>) -> Result<()> {
    let status = Command::new("open").args(open_arguments(target, args)).status().context("running open")?;
    if !status.success() {
        bail!("macOS could not open \"{target}\"");
    }
    Ok(())
}

/// Keys without a character, by their macOS key code.
fn key_code(name: &str) -> Option<u8> {
    Some(match name {
        "enter" | "return" => 36,
        "tab" => 48,
        "space" => 49,
        "backspace" => 51,
        "esc" | "escape" => 53,
        "delete" | "del" => 117,
        "home" => 115,
        "end" => 119,
        "pageup" => 116,
        "pagedown" => 121,
        "left" => 123,
        "right" => 124,
        "down" => 125,
        "up" => 126,
        "f1" => 122,
        "f2" => 120,
        "f3" => 99,
        "f4" => 118,
        "f5" => 96,
        "f6" => 97,
        "f7" => 98,
        "f8" => 100,
        "f9" => 101,
        "f10" => 109,
        "f11" => 103,
        "f12" => 111,
        _ => return None,
    })
}

/// "cmd+shift+4" → the AppleScript line that presses it. `ctrl`, `alt` and
/// `win` keep their Windows names so one config works on both systems; `win`
/// is the Command key.
fn keys_script(combo: &str) -> Result<String> {
    let mut modifiers: Vec<&str> = Vec::new();
    let mut key: Option<String> = None;
    for part in combo.split('+') {
        let name = part.trim().to_ascii_lowercase();
        let modifier = match name.as_str() {
            "ctrl" | "control" => Some("control down"),
            "shift" => Some("shift down"),
            "alt" | "option" | "opt" => Some("option down"),
            "cmd" | "command" | "win" | "meta" | "super" => Some("command down"),
            _ => None,
        };
        match modifier {
            Some(modifier) => {
                if !modifiers.contains(&modifier) {
                    modifiers.push(modifier);
                }
            }
            None if key.is_some() => bail!("more than one key in \"{combo}\""),
            None => {
                key = Some(match key_code(&name) {
                    Some(code) => format!("key code {code}"),
                    None if name.chars().count() == 1 => format!("keystroke {}", quoted(&name)),
                    None => bail!("unknown key \"{part}\" in \"{combo}\""),
                });
            }
        }
    }
    let key = key.ok_or_else(|| anyhow!("no key in \"{combo}\""))?;
    let using = if modifiers.is_empty() { String::new() } else { format!(" using {{{}}}", modifiers.join(", ")) };
    Ok(format!("tell application \"System Events\" to {key}{using}"))
}

pub fn send_keys(combo: &str) -> Result<()> {
    osascript(&[&keys_script(combo)?]).map(drop)
}

/// Control-Command-Q is the system's own "Lock Screen" shortcut.
pub fn lock() -> Result<()> {
    send_keys("ctrl+cmd+q")
}

pub fn display_off() -> Result<()> {
    let status = Command::new("pmset").arg("displaysleepnow").status().context("running pmset")?;
    if !status.success() {
        bail!("pmset could not put the display to sleep");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_combos_become_applescript() {
        assert_eq!(keys_script("cmd+shift+4").unwrap(), "tell application \"System Events\" to keystroke \"4\" using {command down, shift down}");
        assert_eq!(keys_script("ctrl+alt+esc").unwrap(), "tell application \"System Events\" to key code 53 using {control down, option down}");
        assert_eq!(keys_script("f5").unwrap(), "tell application \"System Events\" to key code 96");
        assert_eq!(keys_script("win+\"").unwrap(), "tell application \"System Events\" to keystroke \"\\\"\" using {command down}");
        assert!(keys_script("ctrl+shift").is_err());
        assert!(keys_script("ctrl+a+b").is_err());
        assert!(keys_script("ctrl+nonsense").is_err());
    }

    #[test]
    fn players_answers_are_understood() {
        let answer = ["Píseň \"X\"", "Kapela", "Album", "playing", "62,5", "187.4"].join(&SEP.to_string());
        let now = parse_now_playing("Spotify", &answer).unwrap();
        assert_eq!((now.title.as_str(), now.app.as_str(), now.playing), ("Píseň \"X\"", "Spotify", true));
        assert_eq!((now.position_ms, now.duration_ms), (62_500, 187_400));
        assert!(parse_now_playing("Music", "").is_none());
        assert!(now_playing_script("Spotify")[3].contains("/ 1000"));
        assert!(!now_playing_script("Music")[3].contains("/ 1000"));
    }

    #[test]
    fn volume_answers_are_understood() {
        let v = parse_volume("50,false").unwrap();
        assert_eq!((v.level, v.muted), (0.5, false));
        assert!(parse_volume("0,true").unwrap().muted);
        assert!(parse_volume("missing value,false").is_err());
    }

    #[test]
    fn launch_targets_map_to_open() {
        assert_eq!(open_arguments("Safari", None), ["-a", "Safari"]);
        assert_eq!(open_arguments("https://example.com", None), ["https://example.com"]);
        assert_eq!(open_arguments("/Applications/Notes.app", Some("--new  window")), ["/Applications/Notes.app", "--args", "--new", "window"]);
        assert_eq!(open_arguments("spotify:", Some(" ")), ["spotify:"]);
    }
}
