# Changelog

Versions follow [semantic versioning](https://semver.org/). Each version has a `vX.Y.Z` tag on GitHub; releases come with the Windows installer and the Android app.

## 1.2.1 – 2026-11-27

- **The pairing QR code opens the app.** Read by the phone's own camera, the link used to end in the browser. Older Androids now offer StreamPhone for it; on newer ones the page in the browser shows "Open in the app" and keeps the one-time code unused until you choose. The app asks before it connects to a computer from such a link.
- **Android app, scanner crash fixed:** "Scan QR code" closed the app, because a library the scanner needs for the camera permission (AndroidX Core) was missing from the package.
- Android app: a page process killed by the system no longer closes the app, and if the app does close by itself, the next start shows why (a tap copies the details).

## 1.2.0 – 2026-11-26

- **Windows app** with an installer: a window with the editor, an icon in the notification area, start with the system.
- **Android app**: full screen, display always on, finds the computer over the cable or Wi-Fi by itself, pairing by QR code or by typing the address, page turning with the volume keys.
- **Languages**: English, Czech, Slovak, German, Polish, Spanish and French, in the dashboard, the editor, the Android app, the tray menu and the installer. Each device follows its own language unless one is chosen in the editor. A fresh install names its pages and buttons in the system's language.
- When the Android app is installed, the agent opens it instead of the browser once the cable is plugged in.
- The agent is now also a library, used by the Windows app.
- Hardening: the agent refuses to start without its editor port (the app window can never show another program's page), the app and the preview address the agent by number instead of `localhost`, at most eight unpaired connections are checked at a time, the Android WebView may not read files of the phone, and the Android release build is not debuggable.
- The waiting screen's clock no longer overflows on phones set to a 12-hour format.

## 1.1.0 – 2026-11-26

- **Wi-Fi**: the phone connects without a cable. It is paired with a QR code from the editor (Connection tab); paired phones can be removed.
- **macOS**: the agent can launch apps, press key combinations, control Spotify and Music, the volume, lock the screen and turn the display off. Not yet verified on a real Mac.
- The phone's display stays on over Wi-Fi too, where a browser does not allow it the usual way.
- The HTTPS client used for Discord can be left out of a build (`--no-default-features`).

## 1.0.1 – 2026-11-26

- The project is licensed under MIT.

## 1.0.0 – 2026-11-26

First release.

- An agent for Windows (Rust); the phone connects over USB with nothing to install on it.
- Buttons: launching programs, key combinations, music, volume, locking the PC; four looks, custom icons, text keys.
- Widgets: clock, media player, system load, processes, calendar, focus timer, to-do list.
- AI tools: sessions of Claude Code and Codex, answering a permission request from the phone, local models (LM Studio, Ollama, OpenAI-compatible servers).
- Discord: who is online on a server, through the public server widget.
- Look: ten themes, wallpapers, see-through cards, custom font, night mode.
- An editor in the PC's browser: drag-and-drop layout with a live preview.
