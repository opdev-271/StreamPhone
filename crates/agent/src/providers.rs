//! Background loops that feed the hub. Each one idles while its topic has no
//! subscribers, so an unused widget costs nothing.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sysinfo::{Networks, System};
use tokio::task::spawn_blocking;
use tokio::time::sleep;

use crate::platform;
use crate::protocol::SystemStats;
use crate::App;

pub const SYSTEM_STATS: &str = "system.stats";
pub const MEDIA_NOW: &str = "media.now";
pub const AUDIO_VOLUME: &str = "audio.volume";

pub fn spawn(app: Arc<App>) {
    tokio::spawn(system_loop(app.clone()));
    tokio::spawn(media_loop(app));
}

async fn system_loop(app: Arc<App>) {
    let mut sys = System::new();
    let mut networks = Networks::new_with_refreshed_list();
    let mut last = Instant::now();
    let mut warm = false;

    loop {
        sleep(Duration::from_secs(1)).await;
        if !app.hub.has_subscribers(SYSTEM_STATS) {
            warm = false;
            continue;
        }

        sys.refresh_cpu_usage();
        sys.refresh_memory();
        networks.refresh(true);
        let elapsed = last.elapsed().as_secs_f64().max(0.001);
        last = Instant::now();

        // CPU usage and network rates are deltas: the first sample after a
        // pause has nothing to compare against, so skip it.
        if !warm {
            warm = true;
            continue;
        }

        let (rx, tx) = networks
            .iter()
            .fold((0u64, 0u64), |(rx, tx), (_, n)| (rx + n.received(), tx + n.transmitted()));

        app.hub.publish(
            SYSTEM_STATS,
            &SystemStats {
                cpu: (sys.global_cpu_usage() * 10.0).round() / 10.0,
                mem_used: sys.used_memory(),
                mem_total: sys.total_memory(),
                net_rx: (rx as f64 / elapsed) as u64,
                net_tx: (tx as f64 / elapsed) as u64,
            },
        );
    }
}

async fn media_loop(app: Arc<App>) {
    let mut track_key = String::new();
    let mut art_id: Option<String> = None;
    let mut art_retries = 0u8;

    loop {
        // Wake early after an action so the phone sees its effect right away.
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {}
            _ = app.kick.notified() => sleep(Duration::from_millis(120)).await,
        }

        if app.hub.has_subscribers(AUDIO_VOLUME) {
            match spawn_blocking(platform::volume).await {
                Ok(Ok(volume)) => app.hub.publish(AUDIO_VOLUME, &Some(volume)),
                _ => app.hub.publish(AUDIO_VOLUME, &None::<()>),
            }
        }

        if !app.hub.has_subscribers(MEDIA_NOW) {
            continue;
        }
        let mut media = match spawn_blocking(platform::media_now).await {
            Ok(Ok(media)) => media,
            Ok(Err(err)) => {
                tracing::debug!("media: {err:#}");
                None
            }
            Err(_) => None,
        };

        if let Some(now) = media.as_mut() {
            let key = format!("{}\u{1f}{}\u{1f}{}", now.title, now.artist, now.album);
            if key != track_key {
                track_key = key;
                art_id = None;
                art_retries = 5;
            }
            // Players often publish the cover a moment after the title changes.
            if art_id.is_none() && art_retries > 0 {
                art_retries -= 1;
                if let Ok(Ok(Some(bytes))) = spawn_blocking(platform::media_art).await {
                    let mut hasher = DefaultHasher::new();
                    track_key.hash(&mut hasher);
                    bytes.len().hash(&mut hasher);
                    let id = format!("{:016x}", hasher.finish());
                    *app.art.lock().unwrap() = Some((id.clone(), bytes));
                    art_id = Some(id);
                }
            }
            now.art = art_id.clone();
        } else {
            track_key.clear();
            art_id = None;
        }

        app.hub.publish(MEDIA_NOW, &media);
    }
}
