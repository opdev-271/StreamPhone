//! Placeholder for platforms that are not ported yet (Linux).

use anyhow::{bail, Result};

use super::MediaCmd;
use crate::protocol::{MediaNow, Volume};

pub fn media_now() -> Result<Option<MediaNow>> {
    Ok(None)
}

pub fn media_art() -> Result<Option<Vec<u8>>> {
    Ok(None)
}

pub fn media_command(_cmd: MediaCmd) -> Result<()> {
    bail!("media control is not supported on this platform yet")
}

pub fn volume() -> Result<Volume> {
    bail!("volume control is not supported on this platform yet")
}

pub fn set_volume(_level: f32) -> Result<()> {
    bail!("volume control is not supported on this platform yet")
}

pub fn toggle_mute() -> Result<()> {
    bail!("volume control is not supported on this platform yet")
}

pub fn launch(_target: &str, _args: Option<&str>) -> Result<()> {
    bail!("launching apps is not supported on this platform yet")
}

pub fn send_keys(_combo: &str) -> Result<()> {
    bail!("sending keys is not supported on this platform yet")
}

pub fn lock() -> Result<()> {
    bail!("locking is not supported on this platform yet")
}

pub fn display_off() -> Result<()> {
    bail!("turning the display off is not supported on this platform yet")
}
