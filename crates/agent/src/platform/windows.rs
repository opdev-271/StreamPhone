//! Windows 10+ implementation (media sessions need Windows 10 1809 or newer).

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    PostMessageW, HWND_BROADCAST, SC_MONITORPOWER, SW_SHOWNORMAL, WM_SYSCOMMAND,
};

use super::{expand_env, MediaCmd};
use crate::protocol::{MediaNow, Volume};

/// Blocking-pool threads are reused, so this is called at the top of every
/// entry point; repeated calls on the same thread are harmless.
fn com_init() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

// ---------------------------------------------------------------- media

fn current_session() -> Result<Option<Session>> {
    com_init();
    let manager = SessionManager::RequestAsync()?.get()?;
    // A null session comes back as an error: nothing is playing anywhere.
    Ok(manager.GetCurrentSession().ok())
}

/// 100 ns ticks since 1601-01-01, the unit of `DateTime::UniversalTime`.
fn now_ticks() -> i64 {
    const UNIX_EPOCH_TICKS: i64 = 116_444_736_000_000_000;
    let unix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    UNIX_EPOCH_TICKS + (unix.as_nanos() / 100) as i64
}

pub fn media_now() -> Result<Option<MediaNow>> {
    let Some(session) = current_session()? else {
        return Ok(None);
    };
    let props = session.TryGetMediaPropertiesAsync()?.get()?;
    let playing = session.GetPlaybackInfo()?.PlaybackStatus()? == PlaybackStatus::Playing;

    let timeline = session.GetTimelineProperties()?;
    let duration = (timeline.EndTime()?.Duration - timeline.StartTime()?.Duration).max(0);
    let mut position = timeline.Position()?.Duration;
    // Apps report the position only now and then; extrapolate while playing.
    let updated = timeline.LastUpdatedTime()?.UniversalTime;
    if playing && updated > 0 {
        position += (now_ticks() - updated).max(0);
    }
    if duration > 0 {
        position = position.clamp(0, duration);
    }

    Ok(Some(MediaNow {
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
        album: props.AlbumTitle()?.to_string(),
        app: session.SourceAppUserModelId()?.to_string(),
        playing,
        position_ms: (position.max(0) / 10_000) as u64,
        duration_ms: (duration / 10_000) as u64,
        art: None,
    }))
}

/// Cover image of the current track (JPEG or PNG bytes), if the app provides one.
pub fn media_art() -> Result<Option<Vec<u8>>> {
    let Some(session) = current_session()? else {
        return Ok(None);
    };
    let props = session.TryGetMediaPropertiesAsync()?.get()?;
    let Ok(thumbnail) = props.Thumbnail() else {
        return Ok(None);
    };
    let stream = thumbnail.OpenReadAsync()?.get()?;
    let size = stream.Size()? as u32;
    if size == 0 || size > 8 * 1024 * 1024 {
        return Ok(None);
    }
    let reader = DataReader::CreateDataReader(&stream)?;
    reader.LoadAsync(size)?.get()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes)?;
    Ok(Some(bytes))
}

pub fn media_command(cmd: MediaCmd) -> Result<()> {
    if let Some(session) = current_session()? {
        let accepted = match cmd {
            MediaCmd::PlayPause => session.TryTogglePlayPauseAsync()?.get()?,
            MediaCmd::Next => session.TrySkipNextAsync()?.get()?,
            MediaCmd::Prev => session.TrySkipPreviousAsync()?.get()?,
        };
        if accepted {
            return Ok(());
        }
    }
    // No session, or the app refused: fall back to the keyboard media keys.
    send_keys(match cmd {
        MediaCmd::PlayPause => "mediaplay",
        MediaCmd::Next => "medianext",
        MediaCmd::Prev => "mediaprev",
    })
}

// ---------------------------------------------------------------- audio

fn endpoint() -> Result<IAudioEndpointVolume> {
    com_init();
    unsafe {
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = devices.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        Ok(device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)?)
    }
}

pub fn volume() -> Result<Volume> {
    let endpoint = endpoint()?;
    unsafe {
        Ok(Volume {
            level: (endpoint.GetMasterVolumeLevelScalar()? * 100.0).round() / 100.0,
            muted: endpoint.GetMute()?.as_bool(),
        })
    }
}

pub fn set_volume(level: f32) -> Result<()> {
    if !level.is_finite() {
        bail!("invalid volume level");
    }
    let endpoint = endpoint()?;
    unsafe {
        endpoint.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null())?;
        if level > 0.0 && endpoint.GetMute()?.as_bool() {
            endpoint.SetMute(BOOL::from(false), std::ptr::null())?;
        }
    }
    Ok(())
}

pub fn toggle_mute() -> Result<()> {
    let endpoint = endpoint()?;
    unsafe {
        let muted = endpoint.GetMute()?.as_bool();
        endpoint.SetMute(BOOL::from(!muted), std::ptr::null())?;
    }
    Ok(())
}

// ---------------------------------------------------------------- actions

/// Opens a program, document, URL or protocol link (`spotify:`, `ms-settings:`)
/// the same way the Run dialog would.
pub fn launch(target: &str, args: Option<&str>) -> Result<()> {
    let file = HSTRING::from(expand_env(target));
    let args = args.map(|a| HSTRING::from(expand_env(a)));
    let params = args.as_ref().map_or(PCWSTR::null(), |a| PCWSTR(a.as_ptr()));
    let result = unsafe { ShellExecuteW(HWND::default(), w!("open"), &file, params, PCWSTR::null(), SW_SHOWNORMAL) };
    // ShellExecute reports success as a value greater than 32.
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(anyhow!("could not open \"{target}\" (code {})", result.0 as usize))
    }
}

fn virtual_key(name: &str) -> Option<u16> {
    Some(match name {
        "ctrl" | "control" => 0x11,
        "shift" => 0x10,
        "alt" => 0x12,
        "win" | "super" | "meta" => 0x5B,
        "esc" | "escape" => 0x1B,
        "enter" | "return" => 0x0D,
        "tab" => 0x09,
        "space" => 0x20,
        "backspace" => 0x08,
        "delete" | "del" => 0x2E,
        "insert" => 0x2D,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "printscreen" => 0x2C,
        "mediaplay" => 0xB3,
        "medianext" => 0xB0,
        "mediaprev" => 0xB1,
        "mediastop" => 0xB2,
        "volumeup" => 0xAF,
        "volumedown" => 0xAE,
        "volumemute" => 0xAD,
        _ => {
            if let Some(n) = name.strip_prefix('f').and_then(|n| n.parse::<u16>().ok()) {
                if (1..=24).contains(&n) {
                    return Some(0x70 + n - 1);
                }
            }
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphanumeric() => c.to_ascii_uppercase() as u16,
                _ => return None,
            }
        }
    })
}

/// Presses a combination such as `ctrl+shift+esc`.
pub fn send_keys(combo: &str) -> Result<()> {
    let keys = combo
        .split('+')
        .map(|part| {
            let name = part.trim().to_ascii_lowercase();
            virtual_key(&name).ok_or_else(|| anyhow!("unknown key \"{part}\" in \"{combo}\""))
        })
        .collect::<Result<Vec<u16>>>()?;

    let input = |vk: u16, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let mut inputs: Vec<INPUT> = keys.iter().map(|&vk| input(vk, false)).collect();
    inputs.extend(keys.iter().rev().map(|&vk| input(vk, true)));

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        bail!("the system blocked the key press")
    }
}

pub fn lock() -> Result<()> {
    unsafe { LockWorkStation()? };
    Ok(())
}

pub fn display_off() -> Result<()> {
    const MONITOR_OFF: isize = 2;
    unsafe { PostMessageW(HWND_BROADCAST, WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(MONITOR_OFF))? };
    Ok(())
}
