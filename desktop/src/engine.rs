//! Owns the proxy: one worker thread with its own Tokio runtime, startable and
//! stoppable from the tray without restarting the process.
//!
//! Mirrors the Android shim's lifecycle (`crates/android-jni/src/android.rs`)
//! minus the JNI calls: the core's `server::run_with_listen` is the accept loop,
//! a `watch` channel is the stop signal, and the `tg://` link comes back through
//! the `on_listen` callback instead of being scraped from the log.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tracing::{error, info};

use tg_ws_proxy_rs::config::Config;
use tg_ws_proxy_rs::server;

use crate::settings::Settings;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Stopped,
    Starting,
    Running,
    Failed(String),
}

impl State {
    pub fn label(&self) -> String {
        match self {
            Self::Stopped => "Остановлен".into(),
            Self::Starting => "Запуск…".into(),
            Self::Running => "Работает".into(),
            Self::Failed(reason) => format!("Ошибка: {reason}"),
        }
    }
}

struct Shared {
    state: State,
    link: Option<String>,
}

pub struct Engine {
    settings: Settings,
    shared: Arc<Mutex<Shared>>,
    thread: Option<std::thread::JoinHandle<()>>,
    shutdown: Option<watch::Sender<bool>>,
}

impl Engine {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            shared: Arc::new(Mutex::new(Shared {
                state: State::Stopped,
                link: None,
            })),
            thread: None,
            shutdown: None,
        }
    }

    pub fn state(&self) -> State {
        match self.shared.lock() {
            Ok(shared) => shared.state.clone(),
            Err(_) => State::Stopped,
        }
    }

    pub fn link(&self) -> Option<String> {
        self.shared.lock().ok().and_then(|shared| shared.link.clone())
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state(), State::Running | State::Starting)
    }

    /// Take edited settings without restarting.
    ///
    /// The window saves settings and then calls this before `stop`/`start`, so
    /// the new port, host and arguments are what the restarted listener binds.
    /// Applying them to a running proxy would change nothing until the next
    /// start anyway — `Config` is parsed once, at start.
    pub fn apply_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    pub fn start(&mut self) {
        if self.thread.is_some() {
            return;
        }

        let args = self.settings.args_line();
        let config = match Config::try_from_cli_line(&args) {
            Ok(config) => config,
            Err(reason) => {
                error!("cannot parse settings into a proxy config: {reason}");
                self.set(State::Failed(reason), None);
                return;
            }
        };

        info!("starting: {args}");
        self.set(State::Starting, None);

        let shared = self.shared.clone();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let spawned = std::thread::Builder::new()
            .name("painhubproxy-core".into())
            .spawn(move || run_core(config, shutdown_rx, shared));

        match spawned {
            Ok(handle) => {
                self.thread = Some(handle);
                self.shutdown = Some(shutdown_tx);
            }
            Err(e) => {
                error!("cannot spawn the proxy thread: {e}");
                self.set(State::Failed(format!("поток не создан: {e}")), None);
            }
        }
    }

    /// Ask the worker to stop and wait (bounded) for it to release the port.
    ///
    /// The wait is what makes an immediate Start-after-Stop reliable: the bind
    /// would otherwise race the old listener and fail with "address in use".
    pub fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(true);
        }

        if let Some(handle) = self.thread.take() {
            let deadline = Instant::now() + Duration::from_secs(3);
            while !handle.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            if handle.is_finished() {
                let _ = handle.join();
            } else {
                error!("proxy did not stop within 3s; leaving it to wind down");
            }
        }

        self.set(State::Stopped, None);
    }

    fn set(&self, state: State, link: Option<String>) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.state = state;
            shared.link = link;
        }
    }
}

fn run_core(config: Config, shutdown_rx: watch::Receiver<bool>, shared: Arc<Mutex<Shared>>) {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        // Two workers: the accept loop never blocks, but the per-packet
        // AES-CTR/SHA-256 work is real CPU time and a media upload should not
        // compete with the IO driver for the only thread.
        .worker_threads(2)
        .thread_name("proxy-worker")
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            error!("cannot start the Tokio runtime: {e}");
            set_shared(&shared, State::Failed(format!("runtime: {e}")), None);
            return;
        }
    };

    let listen_shared = shared.clone();
    let result = runtime.block_on(async move {
        let shutdown = async move {
            let mut rx = shutdown_rx;
            loop {
                if *rx.borrow() {
                    break;
                }
                if rx.changed().await.is_err() {
                    break;
                }
            }
        };

        server::run_with_listen(config, shutdown, move |listen| {
            info!("listening on {} — {}", listen.addr, listen.tg_link);
            if let Ok(mut shared) = listen_shared.lock() {
                shared.state = State::Running;
                shared.link = Some(listen.tg_link);
            }
        })
        .await
    });

    // Bounded: a plain drop would block on an outstanding `getaddrinfo`.
    runtime.shutdown_timeout(Duration::from_secs(2));

    match result {
        Ok(()) => set_shared(&shared, State::Stopped, None),
        Err(e) => {
            error!("proxy stopped with an error: {e}");
            set_shared(&shared, State::Failed(e.to_string()), None);
        }
    }
}

fn set_shared(shared: &Arc<Mutex<Shared>>, state: State, link: Option<String>) {
    if let Ok(mut shared) = shared.lock() {
        shared.state = state;
        shared.link = link;
    }
}
