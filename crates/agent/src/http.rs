//! A very small blocking HTTP/1.1 client for talking to servers on this
//! machine or the local network: the agent itself (from the `hook` and
//! `report` commands) and local model servers. Plain `http://` only.

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

fn bad(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, text.to_owned())
}

/// `http://host[:port]/path` → (`host:port`, `/path`).
fn split(url: &str) -> io::Result<(String, String)> {
    let rest = url.strip_prefix("http://").ok_or_else(|| bad("only http:// addresses are supported"))?;
    let (host, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    if host.is_empty() {
        return Err(bad("the address has no host"));
    }
    // An IPv6 literal carries its own colons: `[::1]:1234`.
    let has_port = host.rsplit_once(':').is_some_and(|(_, port)| !port.contains(']'));
    let host = if has_port { host.to_owned() } else { format!("{host}:80") };
    Ok((host, path.to_owned()))
}

fn unchunk(mut data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(end) = data.windows(2).position(|w| w == b"\r\n") {
        let size = std::str::from_utf8(&data[..end]).ok().and_then(|line| {
            usize::from_str_radix(line.split(';').next().unwrap_or("").trim(), 16).ok()
        });
        let Some(size) = size else { break };
        let start = end + 2;
        if size == 0 || data.len() < start + size {
            break;
        }
        out.extend_from_slice(&data[start..start + size]);
        data = data.get(start + size + 2..).unwrap_or(&[]);
    }
    out
}

/// Sends one request and reads the whole answer. `body` is sent as JSON.
pub fn request(method: &str, url: &str, body: Option<&[u8]>, connect: Duration, read: Duration) -> io::Result<Response> {
    let (host, path) = split(url)?;
    // `localhost` may resolve to IPv6 first while the server listens on IPv4
    // only (or the other way round): try every address.
    let mut stream = None;
    let mut last = bad("the address did not resolve");
    for addr in host.to_socket_addrs()? {
        match TcpStream::connect_timeout(&addr, connect) {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(err) => last = err,
        }
    }
    let mut stream = stream.ok_or(last)?;
    stream.set_read_timeout(Some(read))?;
    stream.set_write_timeout(Some(connect.max(Duration::from_secs(5))))?;

    let mut head = format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\nAccept: application/json\r\nConnection: close\r\n");
    if let Some(body) = body {
        head.push_str(&format!("Content-Type: application/json\r\nContent-Length: {}\r\n", body.len()));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if let Some(body) = body {
        stream.write_all(body)?;
    }

    let mut raw = Vec::new();
    // Servers that ignore `Connection: close` end the read with a timeout;
    // whatever arrived by then is still a complete answer if it parses.
    if let Err(err) = stream.take(16 * 1024 * 1024).read_to_end(&mut raw) {
        if raw.is_empty() {
            return Err(err);
        }
    }
    let split_at = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| bad("not an HTTP answer"))?;
    let head = String::from_utf8_lossy(&raw[..split_at]).to_ascii_lowercase();
    let status = head.split_whitespace().nth(1).and_then(|code| code.parse().ok()).ok_or_else(|| bad("no status"))?;
    let payload = &raw[split_at + 4..];
    let chunked = head.lines().any(|line| line.starts_with("transfer-encoding:") && line.contains("chunked"));
    Ok(Response { status, body: if chunked { unchunk(payload) } else { payload.to_vec() } })
}

pub fn get_json(url: &str, wait: Duration) -> io::Result<serde_json::Value> {
    let res = request("GET", url, None, Duration::from_millis(500), wait)?;
    if !(200..300).contains(&res.status) {
        return Err(bad(&format!("status {}", res.status)));
    }
    serde_json::from_slice(&res.body).map_err(|_| bad("the answer is not JSON"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_and_chunks() {
        assert_eq!(split("http://127.0.0.1:1234/api/v1/models").unwrap(), ("127.0.0.1:1234".into(), "/api/v1/models".into()));
        assert_eq!(split("http://localhost").unwrap(), ("localhost:80".into(), "/".into()));
        assert_eq!(split("http://[::1]:11434/api/ps").unwrap().0, "[::1]:11434");
        assert!(split("https://example.com").is_err());
        assert_eq!(unchunk(b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n"), b"Wikipedia");
    }
}
