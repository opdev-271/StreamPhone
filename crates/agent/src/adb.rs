//! USB link: forwards the agent's port to every plugged-in Android phone with
//! `adb reverse`, so the phone reaches the PC at `http://localhost:<port>`
//! through the cable. No Wi-Fi or network setup involved.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::sleep;

use crate::App;

pub const DEVICES: &str = "agent.devices";

/// Package name of the Android app (see `android/`).
const APP: &str = "app.streamphone";

fn find_adb() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "adb.exe" } else { "adb" };

    let mut candidates: Vec<PathBuf> = Vec::new();
    // Next to the agent (how a packaged build ships it).
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        candidates.push(dir.join(exe));
        candidates.push(dir.join("platform-tools").join(exe));
    }
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(sdk) = std::env::var(var) {
            candidates.push(PathBuf::from(sdk).join("platform-tools").join(exe));
        }
    }
    if let Some(local) = dirs::data_local_dir() {
        candidates.push(local.join("Android").join("Sdk").join("platform-tools").join(exe));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("Library/Android/sdk/platform-tools").join(exe));
        candidates.push(home.join("Android/Sdk/platform-tools").join(exe));
    }
    if let Some(found) = candidates.into_iter().find(|p| p.is_file()) {
        return Some(found);
    }
    // Finally, anywhere on PATH.
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|dir| dir.join(exe)).find(|p| p.is_file()))
}

fn adb(path: &PathBuf) -> Command {
    let mut cmd = Command::new(path);
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Serial numbers of phones that are connected and authorized.
fn parse_devices(output: &str) -> HashSet<String> {
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            match (parts.next(), parts.next()) {
                (Some(serial), Some("device")) => Some(serial.to_owned()),
                _ => None,
            }
        })
        .collect()
}

/// Opens the dashboard on the phone. A phone that is still booting
/// (or an emulator that was just started) already answers adb but cannot start
/// apps yet, so wait for the boot to finish first.
async fn open_dashboard(path: PathBuf, serial: String, port: u16) {
    for _ in 0..60 {
        let booted = adb(&path)
            .args(["-s", &serial, "shell", "getprop", "sys.boot_completed"])
            .output()
            .await
            .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).trim() == "1");
        if booted {
            // The StreamPhone app if the phone has it, otherwise the browser.
            let has_app = adb(&path)
                .args(["-s", &serial, "shell", "pm", "path", APP])
                .output()
                .await
                .is_ok_and(|out| out.status.success() && String::from_utf8_lossy(&out.stdout).contains("package:"));
            let url = format!("http://localhost:{port}/");
            let component = format!("{APP}/.MainActivity");
            let start: Vec<&str> = if has_app {
                vec!["-s", &serial, "shell", "am", "start", "-n", &component]
            } else {
                vec!["-s", &serial, "shell", "am", "start", "-a", "android.intent.action.VIEW", "-d", &url]
            };
            let opened = adb(&path).args(start).output().await.is_ok_and(|out| out.status.success());
            if opened {
                return;
            }
        }
        sleep(Duration::from_secs(2)).await;
    }
    tracing::warn!("could not open the dashboard on phone {serial}");
}

pub async fn run(app: Arc<App>) {
    let Some(path) = find_adb() else {
        tracing::warn!("adb not found: USB connection is off. Install Android platform-tools or put adb next to the agent.");
        return;
    };
    tracing::info!("using adb at {}", path.display());

    let port = app.port;
    let mut linked: HashSet<String> = HashSet::new();

    loop {
        let online = match adb(&path).arg("devices").output().await {
            Ok(out) => parse_devices(&String::from_utf8_lossy(&out.stdout)),
            Err(err) => {
                tracing::debug!("adb devices failed: {err}");
                HashSet::new()
            }
        };

        linked.retain(|serial| online.contains(serial));

        for serial in &online {
            if linked.contains(serial) {
                continue;
            }
            let spec = format!("tcp:{port}");
            let ok = adb(&path)
                .args(["-s", serial, "reverse", &spec, &spec])
                .status()
                .await
                .is_ok_and(|s| s.success());
            if !ok {
                tracing::warn!("could not forward the port to phone {serial}");
                continue;
            }
            tracing::info!("phone {serial} connected over USB");
            linked.insert(serial.clone());

            if app.config.read().unwrap().open_on_connect {
                tokio::spawn(open_dashboard(path.clone(), serial.clone(), port));
            }
        }

        let mut devices: Vec<&String> = linked.iter().collect();
        devices.sort();
        app.hub.publish(DEVICES, &devices);

        sleep(Duration::from_secs(3)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::parse_devices;

    #[test]
    fn only_authorized_devices_count() {
        let out = "List of devices attached\nR58M123ABC\tdevice\nemulator-5554\toffline\nXYZ\tunauthorized\n\n";
        let devices = parse_devices(out);
        assert_eq!(devices.len(), 1);
        assert!(devices.contains("R58M123ABC"));
    }
}
