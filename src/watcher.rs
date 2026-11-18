//! Loop that runs the monitor: 30 seconds after opening and then every 15 minutes, never two
//! cycles at once. Anyone can request an early cycle with `request_cycle`.

use std::any::Any;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;

use crate::desktop;
use crate::monitor::{self, Memory, Output};
use crate::session::AppState;

/// Wait before the first cycle.
pub const INITIAL_DELAY: Duration = Duration::from_secs(30);
/// Time between cycles.
pub const INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Why the wait ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Requested,
    Timeout,
}

/// Waits until `until` or until someone requests a cycle.
pub async fn wait(signal: &Notify, until: Duration) -> Reason {
    match tokio::time::timeout(until, signal.notified()).await {
        Ok(()) => Reason::Requested,
        Err(_) => Reason::Timeout,
    }
}

/// Lets any part of the app request a monitor cycle.
#[derive(Default)]
pub struct Watcher {
    signal: Arc<Notify>,
}

impl Watcher {
    pub fn new() -> Self {
        Watcher { signal: Arc::new(Notify::new()) }
    }

    /// Requests a cycle; several consecutive requests merge into one.
    pub fn request(&self) {
        self.signal.notify_one();
    }
}

/// Requests a cycle if the watcher is already in the Tauri state.
pub fn request_cycle(app: &AppHandle) {
    if let Some(watcher) = app.try_state::<Watcher>() {
        watcher.request();
    }
}

/// Real output of the monitor: tray tooltip and Windows notifications.
pub struct TauriOutput(pub AppHandle);

impl Output for TauriOutput {
    fn tooltip(&self, text: &str) {
        desktop::set_tooltip(&self.0, text);
    }

    fn notify(&self, title: &str, body: &str) {
        if let Err(e) = self.0.notification().builder().title(title).body(body).show() {
            eprintln!("watcher: could not show the notification: {e}");
        }
    }
}

/// Launches the monitor loop in the background.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let signal = match app.try_state::<Watcher>() {
            Some(w) => w.signal.clone(),
            None => {
                eprintln!("watcher: no watcher in the state, the monitor does not start");
                return;
            }
        };
        let state = app.state::<AppState>();
        let output = TauriOutput(app.clone());
        run_loop(&state, &output, &signal, INITIAL_DELAY, INTERVAL, None).await;
    });
}

const MAX_PANIC_MESSAGE: usize = 200;

fn panic_message(payload: &(dyn Any + Send)) -> String {
    let text = if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        "no message"
    };
    text.chars().take(MAX_PANIC_MESSAGE).collect()
}

/// Runs `f`; if it panics, returns `Err` with the panic message.
pub async fn guarded_cycle<F: Future>(f: F) -> Result<F::Output, String> {
    AssertUnwindSafe(f).catch_unwind().await.map_err(|p| panic_message(p.as_ref()))
}

/// Monitor loop: waits, runs a cycle and repeats. A cycle panic is logged, shown in the tooltip
/// and does not stop the loop. `max_cycles` is `None` in production.
pub async fn run_loop(
    state: &AppState,
    output: &dyn Output,
    signal: &Notify,
    initial_delay: Duration,
    interval: Duration,
    max_cycles: Option<usize>,
) {
    let mut memory = Memory::default();
    let mut delay = initial_delay;
    let mut done = 0usize;
    while max_cycles.map_or(true, |max| done < max) {
        wait(signal, delay).await;
        match guarded_cycle(monitor::cycle(state, output, monitor::now_ecuador(), &mut memory)).await {
            Ok(r) => eprintln!("watcher: cycle finished: {r:?}"),
            Err(message) => {
                eprintln!("watcher: the cycle panicked: {message}");
                let text = monitor::internal_error_text(state.simulated);
                if catch_unwind(AssertUnwindSafe(|| output.tooltip(&text))).is_err() {
                    eprintln!("watcher: could not set the internal error tooltip");
                }
            }
        }
        done += 1;
        delay = interval;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use super::*;
    use crate::secret_store::MemorySecretStore;

    #[tokio::test]
    async fn without_request_ends_by_timeout() {
        let signal = Notify::new();
        assert_eq!(wait(&signal, Duration::from_millis(50)).await, Reason::Timeout);
    }

    #[tokio::test]
    async fn earlier_request_is_served_instantly() {
        let signal = Notify::new();
        signal.notify_one();
        assert_eq!(wait(&signal, Duration::from_secs(5)).await, Reason::Requested);
    }

    #[tokio::test]
    async fn two_consecutive_requests_merge_into_one() {
        let signal = Notify::new();
        signal.notify_one();
        signal.notify_one();
        assert_eq!(wait(&signal, Duration::from_secs(5)).await, Reason::Requested);
        assert_eq!(wait(&signal, Duration::from_millis(50)).await, Reason::Timeout);
    }

    #[test]
    fn fixed_times() {
        assert_eq!(INTERVAL.as_secs(), 900);
        assert_eq!(INITIAL_DELAY.as_secs(), 30);
    }

    #[tokio::test]
    async fn watcher_wakes_the_wait() {
        let w = Watcher::new();
        w.request();
        assert_eq!(wait(&w.signal, Duration::from_secs(5)).await, Reason::Requested);
    }

    #[tokio::test]
    async fn guarded_cycle_returns_the_value_or_the_panic_message() {
        assert_eq!(guarded_cycle(async { 7 }).await, Ok(7));
        let r = guarded_cycle(async {
            if true {
                panic!("boom");
            }
            0
        })
        .await;
        assert!(r.unwrap_err().contains("boom"));
    }

    #[tokio::test]
    async fn panic_message_is_truncated() {
        let long = "x".repeat(500);
        let r = guarded_cycle(async move {
            if true {
                panic!("{long}");
            }
        })
        .await;
        assert_eq!(r.unwrap_err().chars().count(), MAX_PANIC_MESSAGE);
    }

    struct OutputFailingOnce {
        calls: AtomicUsize,
        tooltips: Mutex<Vec<String>>,
    }

    impl Output for OutputFailingOnce {
        fn tooltip(&self, text: &str) {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                panic!("failure caused in the test");
            }
            self.tooltips.lock().unwrap().push(text.to_string());
        }
        fn notify(&self, _title: &str, _body: &str) {}
    }

    #[tokio::test]
    async fn loop_survives_a_cycle_panic() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(
            true,
            dir.path().to_path_buf(),
            dir.path().join("data"),
            Box::new(MemorySecretStore::default()),
        );
        let output = OutputFailingOnce { calls: AtomicUsize::new(0), tooltips: Mutex::new(Vec::new()) };
        let signal = Notify::new();
        let ms = Duration::from_millis(5);
        tokio::time::timeout(Duration::from_secs(5), run_loop(&state, &output, &signal, ms, ms, Some(2)))
            .await
            .expect("the loop did not finish");
        let tooltips = output.tooltips.lock().unwrap().clone();
        assert_eq!(tooltips, vec![monitor::internal_error_text(true), monitor::no_session_text(true)]);
        assert!(tooltips[0].ends_with(" (simulado)"));
    }

    struct AlwaysFailingOutput;

    impl Output for AlwaysFailingOutput {
        fn tooltip(&self, _text: &str) {
            panic!("broken tooltip");
        }
        fn notify(&self, _title: &str, _body: &str) {}
    }

    #[tokio::test]
    async fn second_panic_setting_the_tooltip_does_not_end_the_loop_either() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(
            true,
            dir.path().to_path_buf(),
            dir.path().join("data"),
            Box::new(MemorySecretStore::default()),
        );
        let signal = Notify::new();
        let ms = Duration::from_millis(5);
        tokio::time::timeout(Duration::from_secs(5), run_loop(&state, &AlwaysFailingOutput, &signal, ms, ms, Some(3)))
            .await
            .expect("the loop did not finish");
    }
}
