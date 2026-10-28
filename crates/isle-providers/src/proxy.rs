//! The network a sandbox has: HTTP and CONNECT through a Unix socket, to granted hosts only
//! (`widgets-2026.md` §1).

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UnixListener, UnixStream},
    task::JoinHandle,
};

/// The longest request head read before giving up.
const HEAD: usize = 16 * 1024;

pub struct Proxy {
    pub path: PathBuf,
    denied: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}

impl Proxy {
    /// Hosts asked for and refused, in order, each once.
    pub fn denied(&self) -> Vec<String> {
        self.denied.lock().unwrap().clone()
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Serves on `path`, letting through `hosts` on `ports`.
pub fn serve(path: &Path, hosts: Vec<String>, ports: Vec<u16>) -> std::io::Result<Proxy> {
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;
    let denied: Arc<Mutex<Vec<String>>> = Arc::default();
    let rules = Arc::new((hosts, ports));
    let task = tokio::spawn({
        let denied = denied.clone();
        async move {
            while let Ok((stream, _)) = listener.accept().await {
                let (rules, denied) = (rules.clone(), denied.clone());
                tokio::spawn(async move {
                    let _ = handle(stream, &rules.0, &rules.1, &denied).await;
                });
            }
        }
    });
    Ok(Proxy {
        path: path.to_owned(),
        denied,
        task,
    })
}

/// A target's host and port: `host:443` for CONNECT, `http://host[:port]/…` otherwise.
fn target(method: &str, target: &str) -> Option<(String, u16, String)> {
    if method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = target.rsplit_once(':')?;
        return Some((host.to_ascii_lowercase(), port.parse().ok()?, String::new()));
    }
    let rest = target.strip_prefix("http://")?;
    let (authority, path) = rest.split_once('/').map_or((rest, "/".to_owned()), |(a, p)| (a, format!("/{p}")));
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, p.parse().ok()?),
        None => (authority, 80),
    };
    Some((host.to_ascii_lowercase(), port, path))
}

async fn handle(
    mut client: UnixStream,
    hosts: &[String],
    ports: &[u16],
    denied: &Mutex<Vec<String>>,
) -> std::io::Result<()> {
    let mut head = Vec::with_capacity(1024);
    let mut buf = [0u8; 4096];
    let end = loop {
        let n = client.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        head.extend_from_slice(&buf[..n]);
        if let Some(at) = head.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        if head.len() > HEAD {
            return Ok(());
        }
    };
    let text = String::from_utf8_lossy(&head[..end]).into_owned();
    let mut lines = text.split("\r\n");
    let first = lines.next().unwrap_or_default();
    let mut parts = first.split(' ');
    let (method, raw, version) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or("HTTP/1.1"));
    let Some((host, port, path)) = target(method, raw) else {
        client.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
        return Ok(());
    };
    if !hosts.iter().any(|h| h.eq_ignore_ascii_case(&host)) || !ports.contains(&port) {
        {
            let mut denied = denied.lock().unwrap();
            if !denied.contains(&host) {
                denied.push(host.clone());
            }
        }
        tracing::info!(host, port, "a provider's connection refused");
        client.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
        return Ok(());
    }
    let Ok(Ok(mut upstream)) = tokio::time::timeout(Duration::from_secs(10), TcpStream::connect((host.as_str(), port))).await
    else {
        client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
        return Ok(());
    };
    if method.eq_ignore_ascii_case("CONNECT") {
        client.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await?;
    } else {
        // The request again in origin form, without the proxy's own headers.
        let mut out = format!("{method} {path} {version}\r\n");
        for line in lines.filter(|l| !l.is_empty()) {
            if !line.to_ascii_lowercase().starts_with("proxy-") {
                out.push_str(line);
                out.push_str("\r\n");
            }
        }
        out.push_str("\r\n");
        upstream.write_all(out.as_bytes()).await?;
    }
    upstream.write_all(&head[end..]).await?;
    tokio::io::copy_bidirectional(&mut client, &mut upstream).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_read_both_forms() {
        assert_eq!(target("CONNECT", "API.example.com:443"), Some(("api.example.com".into(), 443, String::new())));
        assert_eq!(target("GET", "http://example.com/a?b"), Some(("example.com".into(), 80, "/a?b".into())));
        assert_eq!(target("GET", "http://example.com:8080"), Some(("example.com".into(), 8080, "/".into())));
        assert_eq!(target("GET", "/relative"), None);
    }
}
