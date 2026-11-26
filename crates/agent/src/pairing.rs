//! Phones on Wi-Fi.
//!
//! Over USB the phone reaches the agent through a port forward, so only a
//! phone that is physically plugged in can talk to it. Over Wi-Fi anybody on
//! the network could, so a phone has to be paired first: the editor on the PC
//! shows a QR code with a one-time code, the phone trades the code for a
//! secret of its own, and from then on proves it knows the secret each time it
//! connects (HMAC over a fresh nonce, so the secret itself is not sent again).
//!
//! The connection is plain HTTP: this keeps out other devices on a home
//! network, not an attacker who can tamper with the network itself.

use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::App;

const CODE_LIFETIME: Duration = Duration::from_secs(10 * 60);
const MAX_DEVICES: usize = 16;
/// Wrong guesses before the pairing code is thrown away.
const MAX_MISSES: u32 = 8;

#[derive(Clone, Serialize, Deserialize)]
struct Device {
    id: String,
    name: String,
    secret: String,
    /// Milliseconds since the Unix epoch.
    created: u64,
}

struct Code {
    text: String,
    made: Instant,
    misses: u32,
}

pub struct Pairing {
    path: PathBuf,
    devices: Mutex<Vec<Device>>,
    code: Mutex<Code>,
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes).expect("the system has a random number source");
    bytes
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok()).collect()
}

/// Eight characters from an alphabet without look-alikes (no 0/O, 1/I/L).
fn new_code() -> Code {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    // Rejection sampling keeps every character equally likely.
    let mut text = String::new();
    while text.len() < 8 {
        for byte in random_bytes::<16>() {
            let limit = 256 - 256 % ALPHABET.len();
            if (byte as usize) < limit && text.len() < 8 {
                text.push(ALPHABET[byte as usize % ALPHABET.len()] as char);
            }
        }
    }
    Code { text, made: Instant::now(), misses: 0 }
}

/// The address other devices on the local network reach this PC at. Only
/// private addresses count: the dashboard is never offered on a public one.
pub fn lan_ip() -> Option<Ipv4Addr> {
    // Connecting a UDP socket sends nothing; it only makes the system pick
    // the interface it would use to reach the outside.
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if ip.is_private() => Some(ip),
        _ => None,
    }
}

impl Pairing {
    pub fn new(config_dir: &std::path::Path) -> Self {
        let path = config_dir.join("devices.json");
        let devices = std::fs::read_to_string(&path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
        Self { path, devices: Mutex::new(devices), code: Mutex::new(new_code()) }
    }

    fn save(&self, devices: &[Device]) {
        if let Ok(text) = serde_json::to_string_pretty(devices) {
            if let Err(err) = std::fs::write(&self.path, text) {
                tracing::warn!("paired phones were not saved: {err}");
            }
        }
    }

    /// The current pairing code and how many seconds it stays valid.
    pub fn code(&self) -> (String, u64) {
        let mut code = self.code.lock().unwrap();
        if code.made.elapsed() >= CODE_LIFETIME {
            *code = new_code();
        }
        (code.text.clone(), (CODE_LIFETIME - code.made.elapsed()).as_secs())
    }

    pub fn renew_code(&self) {
        *self.code.lock().unwrap() = new_code();
    }

    /// Uses the code up if `given` is it.
    fn redeem(&self, given: &str) -> bool {
        let mut code = self.code.lock().unwrap();
        if code.made.elapsed() >= CODE_LIFETIME {
            *code = new_code();
            return false;
        }
        // Compare without leaking how many leading characters matched.
        let same = given.len() == code.text.len()
            && given.bytes().zip(code.text.bytes()).fold(0u8, |acc, (a, b)| acc | (a.to_ascii_uppercase() ^ b)) == 0;
        if same {
            *code = new_code();
        } else {
            code.misses += 1;
            if code.misses >= MAX_MISSES {
                *code = new_code();
            }
        }
        same
    }

    fn add(&self, name: &str) -> Device {
        let name: String = name.chars().filter(|c| !c.is_control()).take(60).collect();
        let device = Device {
            id: hex(&random_bytes::<8>()),
            name: if name.trim().is_empty() { "Telefon".to_owned() } else { name.trim().to_owned() },
            secret: hex(&random_bytes::<32>()),
            created: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64),
        };
        let mut devices = self.devices.lock().unwrap();
        if devices.len() >= MAX_DEVICES {
            devices.remove(0);
        }
        devices.push(device.clone());
        self.save(&devices);
        device
    }

    pub fn knows(&self, id: &str) -> bool {
        self.devices.lock().unwrap().iter().any(|d| d.id == id)
    }

    /// Whether `proof` is HMAC-SHA256(device secret, nonce).
    fn proves(&self, id: &str, nonce: &str, proof: &str) -> bool {
        let Some(secret) = self.devices.lock().unwrap().iter().find(|d| d.id == id).map(|d| d.secret.clone()) else {
            return false;
        };
        let (Some(proof), Ok(mut mac)) = (unhex(proof), Hmac::<Sha256>::new_from_slice(secret.as_bytes())) else {
            return false;
        };
        mac.update(nonce.as_bytes());
        mac.verify_slice(&proof).is_ok()
    }

    /// Paired phones, without their secrets.
    pub fn list(&self) -> Value {
        let devices = self.devices.lock().unwrap();
        Value::Array(devices.iter().map(|d| json!({ "id": d.id, "name": d.name, "created": d.created })).collect())
    }

    pub fn remove(&self, id: &str) -> bool {
        let mut devices = self.devices.lock().unwrap();
        let before = devices.len();
        devices.retain(|d| d.id != id);
        self.save(&devices);
        devices.len() != before
    }
}

/// Connections from the network that have not proved a pairing yet.
static WAITING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
const MAX_WAITING: usize = 8;

/// A place among the connections still being checked; freed when dropped.
pub struct Slot;

impl Slot {
    pub fn take() -> Option<Self> {
        use std::sync::atomic::Ordering::SeqCst;
        if WAITING.fetch_add(1, SeqCst) >= MAX_WAITING {
            WAITING.fetch_sub(1, SeqCst);
            return None;
        }
        Some(Slot)
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        WAITING.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

type Sink = SplitSink<WebSocket, Message>;
type Stream = SplitStream<WebSocket>;

/// Lets a phone on the network in: asks it to prove a pairing, or to pair now
/// with the code from the QR. Returns the device id once it is in.
pub async fn admit(app: &App, tx: &mut Sink, rx: &mut Stream) -> Option<String> {
    let nonce = hex(&random_bytes::<16>());
    let hello = json!({ "t": "hello", "auth": "required", "nonce": nonce });
    tx.send(Message::Text(hello.to_string().into())).await.ok()?;

    let first = tokio::time::timeout(Duration::from_secs(20), rx.next()).await.ok()??.ok()?;
    let Message::Text(text) = first else { return None };
    let msg: Value = serde_json::from_str(text.as_str()).ok()?;
    let field = |key: &str| msg.get(key).and_then(Value::as_str).unwrap_or("");

    let reason = match field("t") {
        "auth" => {
            if app.pairing.proves(field("device"), &nonce, field("proof")) {
                return Some(field("device").to_owned());
            }
            "unknown"
        }
        "pair" => {
            if app.pairing.redeem(field("code")) {
                let device = app.pairing.add(field("name"));
                tracing::info!("paired a phone over Wi-Fi: {}", device.name);
                let paired = json!({ "t": "paired", "device": device.id, "secret": device.secret });
                tx.send(Message::Text(paired.to_string().into())).await.ok()?;
                return Some(device.id);
            }
            "code"
        }
        _ => "unpaired",
    };
    // Slow guessing down.
    tokio::time::sleep(Duration::from_secs(1)).await;
    let _ = tx.send(Message::Text(json!({ "t": "denied", "reason": reason }).to_string().into())).await;
    None
}

/// The QR code for `text`, as an SVG picture.
pub fn qr_svg(text: &str) -> Option<String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    Some(code.render::<svg::Color>().min_dimensions(220, 220).dark_color(svg::Color("#000000")).light_color(svg::Color("#ffffff")).build())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Pairing {
        let dir = std::env::temp_dir().join(format!("streamphone-test-{}", hex(&random_bytes::<6>())));
        std::fs::create_dir_all(&dir).unwrap();
        Pairing::new(&dir)
    }

    #[test]
    fn a_code_pairs_one_phone_once() {
        let pairing = store();
        let (code, left) = pairing.code();
        assert_eq!(code.len(), 8);
        assert!(left > 590);
        assert!(!pairing.redeem("WRONGONE"));
        assert!(pairing.redeem(&code.to_lowercase()));
        assert!(!pairing.redeem(&code), "a used code is gone");
        assert_ne!(pairing.code().0, code);
    }

    #[test]
    fn too_many_wrong_guesses_replace_the_code() {
        let pairing = store();
        let code = pairing.code().0;
        for _ in 0..MAX_MISSES {
            assert!(!pairing.redeem("AAAAAAAA"));
        }
        assert!(!pairing.redeem(&code));
    }

    #[test]
    fn a_paired_phone_proves_itself_and_can_be_removed() {
        let pairing = store();
        let device = pairing.add("Pixel");
        let mut mac = Hmac::<Sha256>::new_from_slice(device.secret.as_bytes()).unwrap();
        mac.update(b"nonce-1");
        let proof = hex(&mac.finalize().into_bytes());

        assert!(pairing.proves(&device.id, "nonce-1", &proof));
        assert!(!pairing.proves(&device.id, "nonce-2", &proof), "a proof is good for one nonce only");
        assert!(!pairing.proves("nobody", "nonce-1", &proof));
        assert!(pairing.list()[0].get("secret").is_none(), "secrets never leave the store");

        assert!(pairing.remove(&device.id));
        assert!(!pairing.knows(&device.id));
        assert!(!pairing.proves(&device.id, "nonce-1", &proof));
    }

    #[test]
    fn hex_round_trips() {
        assert_eq!(unhex("00ff10"), Some(vec![0, 255, 16]));
        assert_eq!(unhex("0g"), None);
        assert_eq!(unhex("abc"), None);
        assert_eq!(hex(&[0, 255, 16]), "00ff10");
    }
}
