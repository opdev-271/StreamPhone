use anyhow::Result;

use crate::platform::{self, MediaCmd};
use crate::protocol::Action;

/// Runs an action. Blocking: call through `spawn_blocking`.
pub fn run(action: &Action) -> Result<()> {
    match action {
        Action::Launch { target, args } => platform::launch(target, args.as_deref()),
        Action::Keys { combo } => platform::send_keys(combo),
        Action::MediaPlayPause => platform::media_command(MediaCmd::PlayPause),
        Action::MediaNext => platform::media_command(MediaCmd::Next),
        Action::MediaPrev => platform::media_command(MediaCmd::Prev),
        Action::MuteToggle => platform::toggle_mute(),
        Action::SetVolume { level } => platform::set_volume(*level),
        Action::Lock => platform::lock(),
        Action::DisplayOff => platform::display_off(),
        // Timer, to-do and process actions are handled by `tools`.
        other => anyhow::bail!("not a system action: {other:?}"),
    }
}
