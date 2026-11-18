//! One monitor cycle: reads the hours, sets the tray tooltip and decides the notifications.
//! Pure logic (no Tauri or clock of its own): the real output is connected through `Output`.

use chrono::{Duration, NaiveDate, NaiveDateTime, Timelike, Utc};

use crate::error::{AppError, AppResult};
use crate::jira;
use crate::notifications::{self, MonitorState};
use crate::preferences::{self, Preferences};
use crate::session::AppState;
use crate::summary::{self, HoursSummary, MAX_TRAY_TEXT};

/// Destination of what the cycle produces (the tray and the Windows notifications).
pub trait Output: Send + Sync {
    fn tooltip(&self, text: &str);
    fn notify(&self, title: &str, body: &str);
}

/// Last good data: the summary and the Ecuador time at which it was read.
#[derive(Debug, Default)]
pub struct Memory {
    pub last: Option<(HoursSummary, NaiveDateTime)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleResult {
    Data,
    NoSession,
    Unauthorized,
    Failed,
}

/// Ecuador time (UTC-5, no daylight saving).
pub fn now_ecuador() -> NaiveDateTime {
    (Utc::now() - Duration::hours(5)).naive_utc()
}

fn truncate(text: String) -> String {
    text.chars().take(MAX_TRAY_TEXT).collect()
}

fn with_simulated_mark(mut text: String, simulated: bool) -> String {
    if simulated {
        text.push_str(" (simulado)");
    }
    truncate(text)
}

/// Tooltip text when there is no session.
pub fn no_session_text(simulated: bool) -> String {
    with_simulated_mark("Signal Aduana · Sin sesión".into(), simulated)
}

/// Tooltip text when a cycle panicked.
pub fn internal_error_text(simulated: bool) -> String {
    with_simulated_mark("Signal Aduana · Error interno, se reintenta".into(), simulated)
}

async fn read_summary(
    source: &jira::Source,
    account: &str,
    today: NaiveDate,
    prefs: &Preferences,
) -> AppResult<HoursSummary> {
    let issues = jira::load_for_summary(source, Some(account), today).await?;
    summary::summarize(issues, today, &prefs.extra_holidays)
}

/// One monitor pass. The time `now` (Ecuador) comes in as a parameter.
pub async fn cycle(
    state: &AppState,
    output: &dyn Output,
    now: NaiveDateTime,
    memory: &mut Memory,
) -> CycleResult {
    let today = now.date();
    let prefs = preferences::load(&state.config_dir, state.simulated);
    let mut log = notifications::load_log(&state.data_dir, state.simulated);

    let mut read: Option<HoursSummary> = None;
    let (tooltip, result) = match state.current().await {
        Err(_) => (no_session_text(state.simulated), CycleResult::NoSession),
        Ok(s) => match read_summary(&s.source, &s.user.account_id, today, &prefs).await {
            Ok(r) => {
                memory.last = Some((r.clone(), now));
                let text = summary::tray_text(Some(&r), state.simulated);
                read = Some(r);
                (text, CycleResult::Data)
            }
            Err(AppError::Unauthorized) => {
                state.clear().await;
                (
                    with_simulated_mark("Signal Aduana · Sin conexión con Jira".into(), state.simulated),
                    CycleResult::Unauthorized,
                )
            }
            Err(e) => {
                eprintln!("monitor: cycle failed: {e}");
                let text = match &memory.last {
                    Some((r, t)) => truncate(format!(
                        "{} · último dato {:02}:{:02}",
                        summary::tray_text(Some(r), state.simulated),
                        t.hour(),
                        t.minute()
                    )),
                    None => summary::tray_text(None, state.simulated),
                };
                (text, CycleResult::Failed)
            }
        },
    };

    output.tooltip(&tooltip);

    let monitor_state = match (&read, result) {
        (Some(r), _) => MonitorState::Summary(r),
        (None, CycleResult::NoSession) => MonitorState::NoSession,
        (None, CycleResult::Unauthorized) => MonitorState::Unauthorized,
        (None, _) => MonitorState::Error,
    };
    let due = notifications::decide(monitor_state, now, &prefs, &log, state.simulated);
    for n in &due {
        output.notify(&n.title, &n.body);
    }
    if !due.is_empty() {
        notifications::mark_sent(&mut log, &due, today);
        if let Err(e) = notifications::save_log(&state.data_dir, state.simulated, &log) {
            eprintln!("monitor: could not save the notification log: {e}");
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::NaiveDate;
    use secrecy::SecretString;
    use url::Url;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::secret_store::MemorySecretStore;
    use crate::auth::{Credentials, Site};
    use crate::jira::http::JiraHttp;
    use crate::jira::simulated::SimulatedJira;
    use crate::jira::Source;
    use crate::session::Session;

    #[derive(Default)]
    struct Recorder {
        tooltips: Mutex<Vec<String>>,
        notifications: Mutex<Vec<(String, String)>>,
    }

    impl Output for Recorder {
        fn tooltip(&self, text: &str) {
            self.tooltips.lock().unwrap().push(text.to_string());
        }
        fn notify(&self, title: &str, body: &str) {
            self.notifications.lock().unwrap().push((title.to_string(), body.to_string()));
        }
    }

    impl Recorder {
        fn tooltips(&self) -> Vec<String> {
            self.tooltips.lock().unwrap().clone()
        }
        fn notifications(&self) -> Vec<(String, String)> {
            self.notifications.lock().unwrap().clone()
        }
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn now(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn state(dir: &tempfile::TempDir) -> AppState {
        AppState::new(
            true,
            dir.path().join("config"),
            dir.path().join("data"),
            Box::new(MemorySecretStore::default()),
        )
    }

    fn site() -> Site {
        Site::parse("prueba.atlassian.net").unwrap()
    }

    async fn set_session(state: &AppState, source: Source, user: crate::jira::model::User) {
        let s = Session { source, user, site: site(), email: "persona@ejemplo.test".into() };
        *state.session.write().await = Some(Arc::new(s));
    }

    async fn simulated_session(state: &AppState, today: &str, valid_token: bool) {
        let valid = Source::Simulated(SimulatedJira::new("persona@ejemplo.test", true, date(today)));
        let user = valid.myself().await.unwrap();
        let source = if valid_token {
            valid
        } else {
            Source::Simulated(SimulatedJira::new("persona@ejemplo.test", false, date(today)))
        };
        set_session(state, source, user).await;
    }

    fn all_within_limit(r: &Recorder) {
        for t in r.tooltips() {
            assert!(t.chars().count() <= MAX_TRAY_TEXT, "tooltip too long: {t}");
        }
    }

    #[test]
    fn no_session_text_with_and_without_simulated_mark() {
        let real = no_session_text(false);
        assert!(real.ends_with("Sin sesión"));
        let sim = no_session_text(true);
        assert!(sim.ends_with(" (simulado)"));
        assert!(real.chars().count() <= MAX_TRAY_TEXT);
        assert!(sim.chars().count() <= MAX_TRAY_TEXT);
    }

    #[test]
    fn internal_error_text_with_and_without_simulated_mark() {
        let real = internal_error_text(false);
        assert!(!real.ends_with(" (simulado)"));
        assert!(real.contains("Error interno"));
        let sim = internal_error_text(true);
        assert!(sim.ends_with(" (simulado)"));
        assert!(real.chars().count() <= MAX_TRAY_TEXT);
        assert!(sim.chars().count() <= MAX_TRAY_TEXT);
    }

    #[tokio::test]
    async fn no_session_shows_tooltip_and_does_not_notify() {
        let dir = tempfile::tempdir().unwrap();
        let e = state(&dir);
        let output = Recorder::default();
        let mut mem = Memory::default();
        let r = cycle(&e, &output, now("2026-11-13 17:00"), &mut mem).await;
        assert_eq!(r, CycleResult::NoSession);
        let t = output.tooltips();
        assert_eq!(t.len(), 1);
        assert!(t[0].contains("Sin sesión"));
        assert!(t[0].ends_with(" (simulado)"));
        assert!(output.notifications().is_empty());
        assert!(mem.last.is_none());
        all_within_limit(&output);
    }

    #[tokio::test]
    async fn simulated_session_gives_data_and_tooltip() {
        let dir = tempfile::tempdir().unwrap();
        let e = state(&dir);
        simulated_session(&e, "2026-11-13", true).await;
        let output = Recorder::default();
        let mut mem = Memory::default();
        let r = cycle(&e, &output, now("2026-11-13 10:00"), &mut mem).await;
        assert_eq!(r, CycleResult::Data);
        let t = output.tooltips();
        assert_eq!(t.len(), 1);
        assert!(t[0].starts_with("Signal Aduana · Semana:"));
        assert!(t[0].contains("(simulado)"));
        let (res, time) = mem.last.as_ref().unwrap();
        assert_eq!(res.today, "2026-11-13");
        assert_eq!(*time, now("2026-11-13 10:00"));
        all_within_limit(&output);
    }

    async fn expected_summary(today: &str) -> HoursSummary {
        let f = Source::Simulated(SimulatedJira::new("persona@ejemplo.test", true, date(today)));
        let me = f.myself().await.unwrap().account_id;
        let issues = jira::load_for_summary(&f, Some(&me), date(today)).await.unwrap();
        summary::summarize(issues, date(today), &Preferences::default().extra_holidays).unwrap()
    }

    #[tokio::test]
    async fn end_of_day_matches_decide_and_is_not_repeated() {
        let dir = tempfile::tempdir().unwrap();
        let e = state(&dir);
        simulated_session(&e, "2026-11-13", true).await;
        let output = Recorder::default();
        let mut mem = Memory::default();
        let t = now("2026-11-13 17:00");

        let r = cycle(&e, &output, t, &mut mem).await;
        assert_eq!(r, CycleResult::Data);

        let res = expected_summary("2026-11-13").await;
        let expected = notifications::decide(
            MonitorState::Summary(&res),
            t,
            &Preferences::default(),
            &notifications::NotificationLog::default(),
            true,
        );
        let sent = output.notifications();
        assert_eq!(sent.len(), expected.len());
        for (sent_one, expected_one) in sent.iter().zip(&expected) {
            assert_eq!(sent_one.0, expected_one.title);
            assert_eq!(sent_one.1, expected_one.body);
        }
        assert!(res.today_secs < 8 * 3600);
        assert_eq!(sent.len(), 1);
        assert!(sent[0].0.starts_with("[Simulado] "));

        // The log stayed on disk: the second cycle of the same day does not repeat.
        let r = cycle(&e, &output, now("2026-11-13 17:15"), &mut mem).await;
        assert_eq!(r, CycleResult::Data);
        assert_eq!(output.notifications().len(), sent.len());
        assert_eq!(output.tooltips().len(), 2);
        all_within_limit(&output);
    }

    #[tokio::test]
    async fn disabled_notifications_do_not_notify() {
        let dir = tempfile::tempdir().unwrap();
        let e = state(&dir);
        let mut p = Preferences::default();
        p.notifications.enabled = false;
        preferences::save(&e.config_dir, true, &p).unwrap();
        simulated_session(&e, "2026-11-13", true).await;
        let output = Recorder::default();
        let mut mem = Memory::default();
        let r = cycle(&e, &output, now("2026-11-13 17:00"), &mut mem).await;
        assert_eq!(r, CycleResult::Data);
        assert!(output.notifications().is_empty());
        assert_eq!(output.tooltips().len(), 1);
        all_within_limit(&output);
    }

    #[tokio::test]
    async fn rejected_token_clears_the_session_and_notifies_once() {
        let dir = tempfile::tempdir().unwrap();
        let e = state(&dir);
        simulated_session(&e, "2026-11-13", false).await;
        let output = Recorder::default();
        let mut mem = Memory::default();

        let r = cycle(&e, &output, now("2026-11-13 10:00"), &mut mem).await;
        assert_eq!(r, CycleResult::Unauthorized);
        assert!(matches!(e.current().await, Err(AppError::NoSession)));
        let t = output.tooltips();
        assert_eq!(t.len(), 1);
        assert!(t[0].contains("Sin conexión con Jira"));
        let n = output.notifications();
        assert_eq!(n.len(), 1);
        assert!(n[0].0.starts_with("[Simulado] "));
        assert!(n[0].0.contains("Vuelve a conectar"));

        // Another rejected session the same day: the notification is not repeated.
        simulated_session(&e, "2026-11-13", false).await;
        let r = cycle(&e, &output, now("2026-11-13 10:15"), &mut mem).await;
        assert_eq!(r, CycleResult::Unauthorized);
        assert_eq!(output.notifications().len(), 1);
        assert_eq!(output.tooltips().len(), 2);
        all_within_limit(&output);
    }

    #[tokio::test]
    async fn network_failure_keeps_the_session_and_shows_the_last_data() {
        let dir = tempfile::tempdir().unwrap();
        let e = state(&dir);
        simulated_session(&e, "2026-11-13", true).await;
        let output = Recorder::default();
        let mut mem = Memory::default();
        assert_eq!(cycle(&e, &output, now("2026-11-13 10:00"), &mut mem).await, CycleResult::Data);

        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::any()).respond_with(ResponseTemplate::new(500)).mount(&server).await;
        let cred = Credentials::classic_token(site(), "persona@ejemplo.test", SecretString::from("tok")).unwrap();
        let base = Url::parse(&format!("{}/", server.uri())).unwrap();
        let user = Source::Simulated(SimulatedJira::new("persona@ejemplo.test", true, date("2026-11-13")))
            .myself()
            .await
            .unwrap();
        set_session(&e, Source::Http(JiraHttp::with_base(cred, base)), user).await;

        let r = cycle(&e, &output, now("2026-11-13 10:15"), &mut mem).await;
        assert_eq!(r, CycleResult::Failed);
        assert!(e.current().await.is_ok());
        let t = output.tooltips();
        assert_eq!(t.len(), 2);
        assert!(t[1].contains("último dato 10:00"), "tooltip: {}", t[1]);

        // Without a previous reading, the tooltip is the "no data" one.
        let mut empty = Memory::default();
        let r = cycle(&e, &output, now("2026-11-13 10:30"), &mut empty).await;
        assert_eq!(r, CycleResult::Failed);
        assert!(output.tooltips()[2].contains("Sin datos"));
        assert!(output.notifications().is_empty());
        all_within_limit(&output);
    }
}
