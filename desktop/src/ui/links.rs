//! Per-interface `tg://` links.
//!
//! The proxy listens on every interface at once (`--host 0.0.0.0`), but a phone
//! reaches it through one concrete address: the Wi-Fi one, the Ethernet one, or
//! loopback when Telegram runs on this very machine. Telegram stores a proxy by
//! its full link, so each address needs its own link — and therefore its own QR
//! code. That is why the window lists one row per address instead of showing a
//! single "the" link.

/// What kind of interface an address belongs to, used only for the hint text.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Local,
    Wifi,
    Wired,
    Other,
}

impl Kind {
    pub fn hint(self) -> &'static str {
        match self {
            Self::Local => "Telegram на этом компьютере",
            Self::Wifi => "телефон в той же Wi-Fi",
            Self::Wired => "телефон в той же локальной сети",
            Self::Other => "другой интерфейс",
        }
    }
}

/// One address a phone can dial.
pub struct Endpoint {
    pub ip: String,
    pub interface: String,
    pub kind: Kind,
}

impl Endpoint {
    pub fn label(&self) -> String {
        match self.kind {
            Kind::Local => "Этот компьютер".to_string(),
            _ => self.interface.clone(),
        }
    }
}

/// Secret in the shape Telegram expects inside a link.
///
/// A bare 32-hex secret means "padded MTProto", which the core advertises with
/// the `dd` prefix (`link_secret_for` in `src/config.rs`). An explicit `dd`/`ee`
/// prefix is kept as written, because `ee` means FakeTLS and must not be
/// rewritten.
pub fn secret_for_link(secret: &str) -> String {
    let secret = secret.trim().to_ascii_lowercase();
    if secret.starts_with("dd") || secret.starts_with("ee") {
        secret
    } else if secret.len() == 32 {
        format!("dd{secret}")
    } else {
        secret
    }
}

pub fn build_link(ip: &str, port: u16, secret: &str) -> String {
    format!(
        "tg://proxy?server={ip}&port={port}&secret={}",
        secret_for_link(secret)
    )
}

/// Loopback first (always available), then every real IPv4 address.
///
/// IPv6 is deliberately skipped: Telegram's proxy entry accepts a host name, but
/// phones on a home network reach the machine over IPv4 far more reliably, and a
/// link that silently does not work is worse than one that is not offered.
pub fn endpoints() -> Vec<Endpoint> {
    let mut out = vec![Endpoint {
        ip: "127.0.0.1".to_string(),
        interface: "loopback".to_string(),
        kind: Kind::Local,
    }];
    let mut seen: Vec<String> = vec!["127.0.0.1".to_string()];

    let Ok(interfaces) = if_addrs::get_if_addrs() else {
        return out;
    };

    for interface in interfaces {
        let ip = match interface.addr {
            if_addrs::IfAddr::V4(v4) => v4.ip,
            if_addrs::IfAddr::V6(_) => continue,
        };
        if !is_usable(ip) {
            continue;
        }
        let text = ip.to_string();
        if seen.contains(&text) {
            continue;
        }
        seen.push(text.clone());
        out.push(Endpoint {
            ip: text,
            interface: interface.name.clone(),
            kind: classify(&interface.name),
        });
    }

    out
}

fn is_usable(ip: std::net::Ipv4Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_broadcast() {
        return false;
    }
    // 169.254.0.0/16 is the "no DHCP" fallback: reachable at most by accident.
    let octets = ip.octets();
    !(octets[0] == 169 && octets[1] == 254)
}

fn classify(name: &str) -> Kind {
    let lower = name.to_lowercase();
    if lower.contains("wi-fi") || lower.contains("wifi") || lower.contains("wlan") {
        Kind::Wifi
    } else if lower.contains("ethernet") || lower.contains("eth") {
        Kind::Wired
    } else {
        Kind::Other
    }
}

/// Human-readable address, used in the tray tooltip.
pub fn address_of(ip: &str, port: u16) -> String {
    format!("{ip}:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_secret_becomes_padded() {
        let secret = "ab".repeat(16);
        assert_eq!(secret_for_link(&secret), format!("dd{secret}"));
    }

    #[test]
    fn explicit_prefix_is_kept() {
        assert_eq!(
            secret_for_link(&format!("ee{}", "cd".repeat(16))),
            format!("ee{}", "cd".repeat(16))
        );
        assert_eq!(
            secret_for_link(&format!("dd{}", "ef".repeat(16))),
            format!("dd{}", "ef".repeat(16))
        );
    }

    #[test]
    fn link_is_well_formed() {
        let link = build_link("192.168.1.5", 1443, &"11".repeat(16));
        assert_eq!(
            link,
            format!(
                "tg://proxy?server=192.168.1.5&port=1443&secret=dd{}",
                "11".repeat(16)
            )
        );
        assert!(link.starts_with("tg://proxy?server="));
    }

    #[test]
    fn loopback_is_always_offered() {
        let list = endpoints();
        assert_eq!(list[0].ip, "127.0.0.1");
        assert!(list.iter().all(|e| e.ip != "169.254.1.1"));
    }
}
