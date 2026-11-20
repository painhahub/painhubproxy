//! Everything that talks to the operating system: where files live, the Windows
//! message pump the tray icon needs, clipboard, autostart and URL opening.
//!
//! Only the Windows path is exercised in practice; the few `cfg(not(windows))`
//! arms exist so the crate still builds elsewhere instead of failing on a
//! missing symbol.

use std::path::{Path, PathBuf};

/// Directory that holds `config.json` and `proxy.log`.
pub fn data_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let dir = base.join("painhubproxy");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn log_path() -> PathBuf {
    data_dir().join("proxy.log")
}

/// The log is appended to across runs; rotate once it grows past a few MB so a
/// long-lived tray icon cannot fill the disk.
const MAX_LOG_BYTES: u64 = 4 * 1024 * 1024;

/// Install a global tracing subscriber that writes to `path`.
///
/// The core logs through `tracing`, so this single writer captures both the
/// proxy's lines and this app's own. Called once, before anything else starts.
pub fn init_logging(path: &Path, verbose: bool) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_LOG_BYTES {
            let rotated = path.with_extension("log.1");
            let _ = std::fs::remove_file(&rotated);
            let _ = std::fs::rename(path, rotated);
        }
    }

    let level = if verbose { "debug" } else { "info" };
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| level.into());

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok();

    match file {
        Some(file) => {
            let writer = FileWriter {
                file: std::sync::Arc::new(std::sync::Mutex::new(file)),
            };
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_target(false)
                        .with_writer(writer),
                )
                .try_init();
        }
        None => {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .try_init();
        }
    }
}

#[derive(Clone)]
struct FileWriter {
    file: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileWriter {
    type Writer = FileGuard;

    fn make_writer(&'a self) -> Self::Writer {
        FileGuard {
            file: self.file.clone(),
        }
    }
}

struct FileGuard {
    file: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
}

impl std::io::Write for FileGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.file.lock() {
            Ok(mut file) => file.write_all(buf).map(|()| buf.len()),
            Err(_) => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
        Ok(())
    }
}

/// Best-effort local address of the interface that would reach the internet.
///
/// `connect` on a UDP socket sends nothing; it only makes the OS pick a route,
/// which is exactly the "which of my addresses is the LAN one" question.
pub fn detect_lan_ip() -> Option<std::net::IpAddr> {
    use std::net::UdpSocket;

    for probe in ["8.8.8.8:53", "1.1.1.1:53"] {
        let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
            continue;
        };
        if socket.connect(probe).is_err() {
            continue;
        }
        if let Ok(local) = socket.local_addr() {
            let ip = local.ip();
            if !ip.is_loopback() && !ip.is_unspecified() {
                return Some(ip);
            }
        }
    }
    None
}

/// Split a `tg://proxy?...` link into `server:port` for the tray tooltip.
pub fn address_of_link(link: &str) -> Option<String> {
    let query = link.split_once('?')?.1;
    let mut server = None;
    let mut port = None;
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("server=") {
            server = Some(value);
        } else if let Some(value) = pair.strip_prefix("port=") {
            port = Some(value);
        }
    }
    Some(format!("{}:{}", server?, port?))
}

#[cfg(windows)]
mod os {
    use std::path::Path;
    use std::process::Command;

    type Handle = *mut core::ffi::c_void;

    pub fn claim_single_instance() -> bool {
        use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
        use windows_sys::Win32::System::Threading::CreateMutexW;

        let name: Vec<u16> = "painhubproxy-desktop-single\0".encode_utf16().collect();
        // The handle is never closed: a named mutex disappears with its last
        // handle, and this must outlive every other window of the process.
        let handle: Handle = unsafe { CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr()) };
        if handle.is_null() {
            return true;
        }
        // Must be read before any other API call: `GetLastError` is per-thread
        // and every intervening call would overwrite it.
        (unsafe { GetLastError() }) != ERROR_ALREADY_EXISTS
    }

    /// Dispatch every pending window message; the tray icon and its menu live on
    /// hidden windows of this thread, so this is what makes clicks arrive.
    pub fn pump_messages() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
        };

        let mut msg: MSG = unsafe { std::mem::zeroed() };
        loop {
            let pending =
                unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) };
            if pending == 0 {
                break;
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn copy_text(text: &str) {
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
        };
        use windows_sys::Win32::System::Memory::{
            GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock,
        };

        /// `CF_UNICODETEXT`, the clipboard format for UTF-16 text.
        const CF_UNICODETEXT: u32 = 13;

        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return;
            }
            EmptyClipboard();

            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let handle = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2);
            if !handle.is_null() {
                let target = GlobalLock(handle) as *mut u16;
                if !target.is_null() {
                    std::ptr::copy_nonoverlapping(wide.as_ptr(), target, wide.len());
                    GlobalUnlock(handle);
                    // Ownership of the block moves to the clipboard here.
                    SetClipboardData(CF_UNICODETEXT, handle);
                }
            }
            CloseClipboard();
        }
    }

    pub fn open_url(url: &str) {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let operation: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = format!("{url}\0").encode_utf16().collect();
        unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            );
        }
    }

    pub fn open_path(path: &Path) {
        if !path.exists() {
            notify("painhubproxy", &format!("Файл не найден:\n{}", path.display()));
            return;
        }
        open_url(&path.to_string_lossy());
    }

    pub fn notify(title: &str, text: &str) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            MB_ICONINFORMATION, MB_OK, MessageBoxW,
        };

        let caption: Vec<u16> = format!("{title}\0").encode_utf16().collect();
        let body: Vec<u16> = format!("{text}\0").encode_utf16().collect();
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                body.as_ptr(),
                caption.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN_VALUE: &str = "painhubproxy";

    fn hidden(program: &str) -> Command {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = Command::new(program);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }

    pub fn set_autostart(enabled: bool) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let quoted = format!("\"{}\"", exe.display());

        let output = if enabled {
            hidden("reg")
                .args([
                    "add",
                    RUN_KEY,
                    "/v",
                    RUN_VALUE,
                    "/t",
                    "REG_SZ",
                    "/d",
                    quoted.as_str(),
                    "/f",
                ])
                .output()
        } else {
            hidden("reg")
                .args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
                .output()
        }
        .map_err(|e| format!("reg.exe: {e}"))?;

        if output.status.success() || !enabled {
            return Ok(());
        }
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(not(windows))]
mod os {
    use std::path::Path;
    use std::process::Command;

    pub fn claim_single_instance() -> bool {
        true
    }

    pub fn pump_messages() {}

    pub fn copy_text(text: &str) {
        let candidates: [(&str, &[&str]); 3] = [
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("pbcopy", &[]),
        ];
        for (program, args) in candidates {
            let child = Command::new(program).args(args).spawn();
            if let Ok(mut child) = child {
                use std::io::Write as _;
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                return;
            }
        }
    }

    pub fn open_url(url: &str) {
        let program = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        let _ = Command::new(program).arg(url).spawn();
    }

    pub fn open_path(path: &Path) {
        open_url(&path.to_string_lossy());
    }

    pub fn notify(title: &str, text: &str) {
        eprintln!("{title}: {text}");
    }

    pub fn set_autostart(_enabled: bool) -> Result<(), String> {
        Err("autostart is only implemented on Windows".into())
    }
}

pub use os::{claim_single_instance, copy_text, notify, open_path, open_url, pump_messages, set_autostart};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_address_is_extracted() {
        let link = "tg://proxy?server=192.168.1.5&port=1443&secret=dd00";
        assert_eq!(address_of_link(link).as_deref(), Some("192.168.1.5:1443"));
        assert_eq!(address_of_link("tg://proxy"), None);
    }
}
