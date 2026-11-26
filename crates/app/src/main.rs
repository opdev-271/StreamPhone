//! StreamPhone for the desktop.
//!
//! The same agent as the command-line program, wrapped the way a desktop app
//! is expected to be: a window with the editor, an icon in the notification
//! area that keeps it running after the window is closed, start with Windows,
//! and an installer.

// No console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use streamphone_agent::Options;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// Started by Windows at sign-in: stay in the notification area.
const HIDDEN: &str = "--hidden";

fn show(app: &AppHandle) {
    for label in ["main", "problem"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn open_window(app: &AppHandle, label: &str, url: WebviewUrl, visible: bool) {
    let built = WebviewWindowBuilder::new(app, label, url)
        .title("StreamPhone")
        .inner_size(1360.0, 840.0)
        .min_inner_size(1000.0, 640.0)
        .visible(visible)
        .build();
    if let Err(err) = built {
        eprintln!("could not open the window: {err}");
    }
}

/// The tray menu: open, start with the system, quit.
fn tray_texts(language: &str) -> [&'static str; 3] {
    match language {
        "cs" => ["Otevřít StreamPhone", "Spouštět po startu systému", "Ukončit"],
        "sk" => ["Otvoriť StreamPhone", "Spúšťať po štarte systému", "Ukončiť"],
        "de" => ["StreamPhone öffnen", "Mit dem System starten", "Beenden"],
        "pl" => ["Otwórz StreamPhone", "Uruchamiaj przy starcie systemu", "Zakończ"],
        "es" => ["Abrir StreamPhone", "Iniciar con el sistema", "Salir"],
        "fr" => ["Ouvrir StreamPhone", "Lancer au démarrage", "Quitter"],
        _ => ["Open StreamPhone", "Start with the system", "Quit"],
    }
}

fn tray(app: &AppHandle) -> tauri::Result<()> {
    let [open_text, autostart_text, quit_text] = tray_texts(streamphone_agent::language());
    let open = MenuItem::with_id(app, "open", open_text, true, None::<&str>)?;
    let starts = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart = CheckMenuItem::with_id(app, "autostart", autostart_text, true, starts, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_text, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &autostart, &PredefinedMenuItem::separator(app)?, &quit])?;

    let mut icon = TrayIconBuilder::with_id("main").tooltip("StreamPhone").menu(&menu).show_menu_on_left_click(false);
    if let Some(image) = app.default_window_icon() {
        icon = icon.icon(image.clone());
    }
    icon.on_menu_event(move |app, event| match event.id().as_ref() {
        "open" => show(app),
        "autostart" => {
            let launcher = app.autolaunch();
            let now = launcher.is_enabled().unwrap_or(false);
            let _ = if now { launcher.disable() } else { launcher.enable() };
            let _ = autostart.set_checked(launcher.is_enabled().unwrap_or(!now));
        }
        // The agent runs on a thread of this process and ends with it.
        "quit" => app.exit(0),
        _ => {}
    })
    .on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            show(tray.app_handle());
        }
    })
    .build(app)?;
    Ok(())
}

fn main() {
    // Installed next to the agent's command-line program, but answer `hook`
    // and `report` here too, so either file works in a tool's settings.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if streamphone_agent::run_cli(&args) {
        return;
    }
    let hidden = args.iter().any(|arg| arg == HIDDEN);

    tauri::Builder::default()
        // A second start only brings the running app forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![HIDDEN])))
        .setup(move |app| {
            tray(app.handle())?;

            // The agent gets a thread and a runtime of its own and tells us
            // the port once the dashboard and the editor are being served.
            let (ready, started) = tokio::sync::oneshot::channel::<u16>();
            std::thread::Builder::new().name("agent".into()).spawn(move || {
                let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
                if let Err(err) = runtime.block_on(streamphone_agent::serve(Options::default(), Some(ready))) {
                    eprintln!("the agent stopped: {err:#}");
                }
            })?;

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match started.await {
                    // The editor lives on the port after the dashboard's.
                    // By number, not `localhost`: that name may resolve to an address
                    // some other program is listening on.
                    Ok(port) => match format!("http://127.0.0.1:{}/", port.wrapping_add(1)).parse() {
                        Ok(url) => open_window(&handle, "main", WebviewUrl::External(url), !hidden),
                        Err(err) => eprintln!("bad editor address: {err}"),
                    },
                    // The agent ended before it could serve: say why.
                    Err(_) => open_window(&handle, "problem", WebviewUrl::App("index.html".into()), true),
                }
            });
            Ok(())
        })
        // Closing the window keeps the agent (and the phone) going; Quit is in the tray menu.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("the app could not start");
}
