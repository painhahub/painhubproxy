//! Round trip to the Cloudflare edge the core actually dials.
//!
//! Telegram's own ping is a full path — phone → this machine → Cloudflare →
//! Telegram DC — and none of it is observable from here. What *is* observable
//! is the leg this app is responsible for: the TCP handshake to the pinned
//! `--cf-ip` addresses, on the port the core really opens (80 while
//! `--cf-disable-tls` is set, 443 otherwise). So the number in the window is
//! the round trip the proxy pays, and comparing it across machines is
//! meaningful.
//!
//! Measuring is one connect every few seconds on a background thread: a few
//! SYN/ACK pairs, no TLS, no payload.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How often the probe re-dials. Cheap enough to look live, slow enough not to
/// show up as traffic.
const INTERVAL: Duration = Duration::from_secs(5);

/// A single connect attempt is abandoned after this. A filtered address does
/// not refuse — it drops, so without a timeout the probe would stall.
const TIMEOUT: Duration = Duration::from_millis(1200);

#[derive(Clone, Debug)]
pub struct Sample {
    /// The address that answered fastest.
    pub target: String,
    pub ms: f64,
    /// Port the core dials: 80 under `--cf-disable-tls`, 443 otherwise.
    pub port: u16,
    /// How many of the configured addresses answered at all.
    pub answered: usize,
    pub total: usize,
    pub at: Instant,
}

struct Shared {
    targets: Vec<String>,
    port: u16,
    best: Option<Sample>,
}

/// Owns the measurement thread. Dropping the handle does not stop the thread —
/// it lives as long as the process, like the tray icon — but the shared state
/// is only read while the window is open.
pub struct Prober {
    shared: Arc<Mutex<Shared>>,
}

impl Prober {
    pub fn start(targets: Vec<String>, port: u16) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            targets,
            port,
            best: None,
        }));

        let worker = shared.clone();
        let spawned = std::thread::Builder::new()
            .name("latency-probe".into())
            .spawn(move || loop {
                let (targets, port) = match worker.lock() {
                    Ok(state) => (state.targets.clone(), state.port),
                    Err(_) => return,
                };

                if targets.is_empty() {
                    std::thread::sleep(INTERVAL);
                    continue;
                }

                let mut best: Option<Sample> = None;
                let mut answered = 0usize;

                for target in &targets {
                    let Some(ms) = ping(target, port) else {
                        continue;
                    };
                    answered += 1;
                    let better = best.as_ref().map(|s| ms < s.ms).unwrap_or(true);
                    if better {
                        best = Some(Sample {
                            target: target.clone(),
                            ms,
                            port,
                            answered: 0,
                            total: 0,
                            at: Instant::now(),
                        });
                    }
                }

                if let Some(sample) = best.as_mut() {
                    sample.answered = answered;
                    sample.total = targets.len();
                }

                if let Ok(mut state) = worker.lock() {
                    state.best = best;
                }

                std::thread::sleep(INTERVAL);
            });

        if spawned.is_err() {
            tracing::warn!("cannot start the latency probe thread");
        }

        Self { shared }
    }

    /// Point the probe at a new set of addresses and port (settings were saved).
    pub fn retarget(&self, targets: Vec<String>, port: u16) {
        if let Ok(mut state) = self.shared.lock() {
            if state.targets == targets && state.port == port {
                return;
            }
            state.targets = targets;
            state.port = port;
            state.best = None;
        }
    }

    pub fn best(&self) -> Option<Sample> {
        self.shared.lock().ok().and_then(|state| state.best.clone())
    }
}

/// One TCP handshake, in milliseconds.
fn ping(target: &str, port: u16) -> Option<f64> {
    let addr: SocketAddr = resolve(target, port)?;
    let started = Instant::now();
    TcpStream::connect_timeout(&addr, TIMEOUT).ok()?;
    Some(started.elapsed().as_secs_f64() * 1000.0)
}

/// `host:port` for a literal address, bracketing a bare IPv6.
fn resolve(target: &str, port: u16) -> Option<SocketAddr> {
    let host = if target.contains(':') && !target.starts_with('[') {
        format!("[{target}]:{port}")
    } else {
        format!("{target}:{port}")
    };
    host.to_socket_addrs().ok()?.next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_ipv6_gets_brackets() {
        assert_eq!(resolve("::1", 443).map(|a| a.port()), Some(443));
        assert_eq!(resolve("2606:4700::1", 80).map(|a| a.port()), Some(80));
    }

    #[test]
    fn ipv4_and_bracketed_forms_parse() {
        let v4 = resolve("104.18.0.1", 80).expect("ipv4");
        assert_eq!(v4.port(), 80);
        let v6 = resolve("[2606:4700::1]", 443).expect("bracketed");
        assert_eq!(v6.port(), 443);
    }

    #[test]
    fn loopback_does_not_hang() {
        // Nothing listens here in the test process; a refused connect still
        // proves the probe returns instead of blocking on the timeout.
        let _ = ping("127.0.0.1", 9);
    }
}
