//! Persisted settings of the desktop app.
//!
//! The secret is generated once and kept for the lifetime of the installation:
//! Telegram stores a proxy by its full `tg://` link, and a new secret on every
//! start would silently invalidate the entry the user already added — every
//! handshake would be answered with a drain and the client would sit in
//! "Connecting…" forever.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::platform;

/// Works on the network this app was written for: the ISP drops TLS whose SNI
/// starts with `kws`, so the Cloudflare proxy tier is reached over `ws://` on
/// port 80 instead. On an unrestricted network the plain ladder
/// (`--default-domains --cf-balance`) is the better choice — see
/// `docs/Desktop.md`.
const DEFAULT_EXTRA_ARGS: &str =
    "--default-domains --cf-balance --pinned-upstream cfproxy --cf-disable-tls";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Port the MTProto listener binds. The LAN link advertises the real one.
    pub port: u16,
    /// `0.0.0.0` so phones on the same Wi-Fi can reach the listener.
    pub host: String,
    /// Address written into the `tg://` link. Empty means "auto-detect".
    pub link_ip: String,
    /// 32 hex chars, optionally prefixed with `dd` (padded MTProto) or `ee`.
    pub secret: String,
    /// Everything else is handed to the core as-is, so advanced routing stays
    /// reachable without this app growing a UI for every flag.
    pub extra_args: String,
    pub autostart: bool,
    pub verbose: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port: 1443,
            host: "0.0.0.0".into(),
            link_ip: String::new(),
            secret: String::new(),
            extra_args: DEFAULT_EXTRA_ARGS.into(),
            autostart: false,
            verbose: false,
        }
    }
}

impl Settings {
    /// Read the config from disk, filling in what can only be decided locally
    /// (secret, LAN address), and write it back when anything was filled in.
    pub fn load_or_create() -> Self {
        let path = platform::config_path();
        let existed = path.exists();

        let mut settings: Settings = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!(
                    "cannot parse {} ({e}); starting from defaults",
                    path.display()
                );
                Settings::default()
            }),
            Err(_) => Settings::default(),
        };

        let mut changed = !existed;

        if settings.link_ip.trim().is_empty() {
            if let Some(ip) = platform::detect_lan_ip() {
                settings.link_ip = ip.to_string();
                changed = true;
            }
        }

        if !is_valid_secret(&settings.secret) {
            settings.secret = generate_secret();
            changed = true;
        }

        if changed {
            settings.save();
        }
        settings
    }

    pub fn save(&self) {
        let path = platform::config_path();
        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = fs::write(&path, text) {
                    tracing::warn!("cannot write {}: {e}", path.display());
                }
            }
            Err(e) => tracing::warn!("cannot serialise settings: {e}"),
        }
    }

    /// Command line handed to `Config::try_from_cli_line`.
    ///
    /// `--port 0` deliberately stays as-is: the core reports the real port in
    /// `ListenInfo`, and the link is built from that, not from this value.
    pub fn args_line(&self) -> String {
        let mut parts = vec![
            "--host".to_string(),
            self.host.clone(),
            "--port".to_string(),
            self.port.to_string(),
        ];

        let link_ip = self.link_ip.trim();
        if !link_ip.is_empty() {
            parts.push("--link-ip".into());
            parts.push(link_ip.to_string());
        }

        parts.push("--secret".into());
        parts.push(self.secret.trim().to_string());

        let extra = self.extra_args.trim();
        if !extra.is_empty() {
            parts.push(extra.to_string());
        }

        parts.join(" ")
    }

    pub fn config_path(&self) -> PathBuf {
        platform::config_path()
    }

    /// Replace the secret with a fresh random one.
    ///
    /// The caller is expected to warn the user: Telegram keeps the *old* link,
    /// so changing the secret invalidates the proxy entry on every device.
    pub fn regenerate_secret(&mut self) {
        self.secret = generate_secret();
    }

    pub fn log_path(&self) -> PathBuf {
        platform::log_path()
    }
}

fn generate_secret() -> String {
    hex::encode(rand::random::<[u8; 16]>())
}

fn is_valid_secret(secret: &str) -> bool {
    let secret = secret.trim();
    if secret.len() == 32 {
        return secret.chars().all(|c| c.is_ascii_hexdigit());
    }
    if secret.len() == 34 && (secret.starts_with("dd") || secret.starts_with("ee")) {
        return secret[2..].chars().all(|c| c.is_ascii_hexdigit());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_secret_shapes() {
        assert!(is_valid_secret(&"a".repeat(32)));
        assert!(is_valid_secret(&format!("dd{}", "b".repeat(32))));
        assert!(is_valid_secret(&format!("ee{}", "c".repeat(32))));
        assert!(!is_valid_secret(""));
        assert!(!is_valid_secret(&"z".repeat(32)));
        assert!(!is_valid_secret(&"a".repeat(31)));
    }

    #[test]
    fn args_line_keeps_secret_and_flags() {
        let settings = Settings {
            port: 1443,
            host: "0.0.0.0".into(),
            link_ip: "192.168.1.10".into(),
            secret: "ab".repeat(16),
            extra_args: "--default-domains".into(),
            ..Settings::default()
        };
        let args = settings.args_line();
        assert_eq!(
            args,
            format!(
                "--host 0.0.0.0 --port 1443 --link-ip 192.168.1.10 --secret {} --default-domains",
                "ab".repeat(16)
            )
        );
        let config = tg_ws_proxy_rs::config::Config::try_from_cli_line(&args).expect("parses");
        assert_eq!(config.port, 1443);
        assert_eq!(config.secrets.len(), 1);
    }
}
