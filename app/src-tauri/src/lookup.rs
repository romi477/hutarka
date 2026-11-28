//! One translator lookup: selection in, streamed answer out to the popup,
//! a journal line at the end.

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use hutarka_llm::{Client, DEFAULT_BASE_URL};
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};

use crate::journal::{LookupRecord, append_lookup, journal_root};
use crate::selection::{Origin, SelectionError, current_origin, read_selection};
use crate::session::Sessions;
use crate::settings::{self, Settings, build_request};
use crate::translator::{Kind, Lang, TextError, kind_for, prepare_text};
use crate::{paste, popup};

/// Why a lookup produced no answer; the popup picks its message and button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    NoKey,
    NotTrusted,
    TooLong,
    Api,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Idle,
    Streaming,
    Done,
    Error,
}

/// Everything the popup shows; sent whole on every change, so the page
/// never depends on having seen earlier events.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub id: u64,
    pub status: Status,
    pub text: String,
    /// The language this lookup translates into.
    pub to: Lang,
    /// Every target card, in Settings order: the popup's menu and ⌘1…⌘9.
    pub targets: Vec<Lang>,
    pub kind: Kind,
    pub explain: bool,
    pub result: String,
    pub model: String,
    pub ms: u64,
    pub error_kind: Option<ErrorKind>,
    pub error: String,
    /// The text came from a selection (Replace can paste over it), not typed
    /// into the popup.
    pub from_selection: bool,
    /// Set when the answer could not be written to the lookup journal.
    pub journal_error: String,
}

pub struct AppState {
    pub sessions: Sessions,
    pub settings: Mutex<Settings>,
    snapshot: Mutex<Snapshot>,
    task: Mutex<Option<JoinHandle<()>>>,
    /// App and window of the last selection read: Replace pastes only there.
    origin: Mutex<Option<Origin>>,
    /// One HTTP client per API key, reused so a lookup skips the TCP and TLS
    /// handshake with the API.
    client: Mutex<Option<(String, Client)>>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        let snapshot = fresh(0, &settings, settings.default_target(), Status::Idle);
        AppState {
            sessions: Sessions::default(),
            settings: Mutex::new(settings),
            snapshot: Mutex::new(snapshot),
            task: Mutex::new(None),
            origin: Mutex::new(None),
            client: Mutex::new(None),
        }
    }

    pub fn snapshot(&self) -> Option<Snapshot> {
        self.snapshot.lock().ok().map(|s| s.clone())
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Changes the snapshot of lookup `id` and sends it. Ignored unless `id` is
    /// the current lookup and the popup already shows it; both are checked under
    /// the lock, so a delta of an old lookup never lands in a new one.
    fn update(&self, app: &AppHandle, id: u64, change: impl FnOnce(&mut Snapshot)) {
        if let Some(snap) = self.apply(id, change) {
            let _ = app.emit_to(popup::LABEL, "popup:snapshot", snap);
        }
    }

    fn apply(&self, id: u64, change: impl FnOnce(&mut Snapshot)) -> Option<Snapshot> {
        let mut snap = self.snapshot.lock().ok()?;
        if snap.id != id || !self.sessions.is_current(id) {
            return None;
        }
        change(&mut snap);
        Some(snap.clone())
    }

    /// Shows a new lookup (`next.id`) and sends it, unless a newer one began.
    fn reset(&self, app: &AppHandle, next: Snapshot) {
        if let Some(snap) = self.replace(next) {
            let _ = app.emit_to(popup::LABEL, "popup:snapshot", snap);
        }
    }

    fn replace(&self, next: Snapshot) -> Option<Snapshot> {
        let mut snap = self.snapshot.lock().ok()?;
        if !self.sessions.is_current(next.id) {
            return None;
        }
        *snap = next;
        Some(snap.clone())
    }

    /// Sets the language and Explain of the lookup the popup shows, for a
    /// popup with nothing to translate yet: the first translation uses them.
    fn choose(&self, to: Lang, explain: bool) -> Option<Snapshot> {
        let mut snap = self.snapshot.lock().ok()?;
        if !self.sessions.is_current(snap.id) {
            return None;
        }
        snap.to = to;
        snap.explain = explain;
        Some(snap.clone())
    }

    /// The client for `key`: the kept one, or a new one when the key changed.
    fn client_for(&self, key: &str) -> Result<Client, hutarka_llm::Error> {
        let mut slot = self
            .client
            .lock()
            .map_err(|_| hutarka_llm::Error::Setup("client lock poisoned".into()))?;
        if let Some((k, client)) = slot.as_ref()
            && k == key
        {
            return Ok(client.clone());
        }
        let client = Client::new(key.to_string(), DEFAULT_BASE_URL.to_string())?;
        *slot = Some((key.to_string(), client.clone()));
        Ok(client)
    }

    fn abort_task(&self) {
        if let Ok(mut task) = self.task.lock()
            && let Some(handle) = task.take()
        {
            handle.abort();
        }
    }
}

/// Hotkey or menu: read the selection, open the popup, translate.
pub fn trigger(app: &AppHandle) {
    let app = app.clone();
    // Reading the selection sleeps and sends keys: keep it off the main thread.
    // The press takes its id now: a slower read of an earlier press (⌘C
    // round trip) finishing later must not replace this one.
    let press = app.state::<AppState>().sessions.begin();
    thread::spawn(move || {
        let selection = read_selection(&app);
        let state = app.state::<AppState>();
        if !state.sessions.is_current(press) {
            log::info!("hotkey press {press} superseded while reading the selection");
            return;
        }
        let settings = state.settings();
        match selection {
            Err(SelectionError::NotTrusted) => {
                log::info!("lookup not started: no Accessibility permission");
                popup::show_near_cursor(&app);
                fail(
                    &app,
                    ErrorKind::NotTrusted,
                    "Hutarka needs Accessibility permission to read the selection.".to_string(),
                );
            }
            Err(SelectionError::Nothing) => compose(&app),
            Ok(raw) => match prepare_text(&raw) {
                Ok(text) => {
                    if let Ok(mut origin) = state.origin.lock() {
                        *origin = current_origin();
                    }
                    let kind = kind_for(&text, settings.explain);
                    popup::show_near_cursor(&app);
                    start(&app, text, settings.default_target(), kind, true);
                }
                Err(TextError::Blank) => compose(&app),
                Err(e @ TextError::TooLong(_)) => {
                    popup::show_near_cursor(&app);
                    fail(&app, ErrorKind::TooLong, e.to_string());
                }
            },
        }
    });
}

/// An empty snapshot of lookup `id` into `to`, with the current cards.
fn fresh(id: u64, settings: &Settings, to: Lang, status: Status) -> Snapshot {
    Snapshot {
        id,
        status,
        text: String::new(),
        to,
        targets: settings.targets.iter().map(|t| t.lang).collect(),
        kind: Kind::Text,
        explain: settings.explain,
        result: String::new(),
        model: String::new(),
        ms: 0,
        error_kind: None,
        error: String::new(),
        from_selection: false,
        journal_error: String::new(),
    }
}

/// Opens the popup with an empty source field to type into.
fn compose(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.abort_task();
    let id = state.sessions.begin();
    let settings = state.settings();
    popup::show_near_cursor(app);
    state.reset(
        app,
        fresh(id, &settings, settings.default_target(), Status::Idle),
    );
}

/// A lookup that ends before any request (no selection, no permission).
fn fail(app: &AppHandle, kind: ErrorKind, message: String) {
    let state = app.state::<AppState>();
    state.abort_task();
    let id = state.sessions.begin();
    let settings = state.settings();
    let to = state
        .snapshot()
        .map_or_else(|| settings.default_target(), |s| s.to);
    state.reset(
        app,
        Snapshot {
            error_kind: Some(kind),
            error: message,
            ..fresh(id, &settings, to, Status::Error)
        },
    );
}

/// Starts (or restarts) a lookup of `text`; any running one is cancelled.
pub fn start(app: &AppHandle, text: String, to: Lang, kind: Kind, from_selection: bool) {
    let state = app.state::<AppState>();
    state.abort_task();
    let id = state.sessions.begin();
    let settings = state.settings();
    let req = build_request(&settings, &text, to, kind);
    state.reset(
        app,
        Snapshot {
            text: text.clone(),
            kind,
            model: req.model.clone(),
            from_selection,
            ..fresh(id, &settings, to, Status::Streaming)
        },
    );
    log::info!(
        "lookup {id}: into {to:?} {kind:?}, {} chars, model {}",
        text.chars().count(),
        req.model
    );
    let key = settings.api_key.trim().to_string();
    if key.is_empty() {
        log::warn!("lookup {id}: no API key");
        state.update(app, id, |s| {
            s.status = Status::Error;
            s.error_kind = Some(ErrorKind::NoKey);
            s.error = "No Anthropic API key".into();
        });
        return;
    }
    let client = match state.client_for(&key) {
        Ok(client) => client,
        Err(e) => {
            log::error!("lookup {id}: {e}");
            state.update(app, id, |s| {
                s.status = Status::Error;
                s.error_kind = Some(ErrorKind::Api);
                s.error = e.to_string();
            });
            return;
        }
    };
    let handle = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let state = handle.state::<AppState>();
        let outcome = client
            .stream(&req, |piece| {
                state.update(&handle, id, |s| s.result.push_str(piece));
            })
            .await;
        let ms = started.elapsed().as_millis() as u64;
        match outcome {
            Ok(result) => {
                log::info!("lookup {id}: done in {ms} ms");
                // Written before the popup says Done, so it can say when the
                // answer did not reach the journal.
                let record = LookupRecord {
                    ts: chrono::Local::now().to_rfc3339(),
                    kind: "lookup",
                    mode: kind,
                    to,
                    text,
                    result: result.clone(),
                    model: req.model.clone(),
                    ms,
                };
                let journal_error =
                    append_lookup(&journal_root(), &record, chrono::Local::now().date_naive())
                        .err()
                        .map(|e| {
                            log::error!("cannot write the lookup journal: {e}");
                            format!("Not saved to history: {e}")
                        })
                        .unwrap_or_default();
                state.update(&handle, id, |s| {
                    s.status = Status::Done;
                    s.result = result;
                    s.ms = ms;
                    s.journal_error = journal_error;
                });
            }
            Err(e) => {
                log::warn!("lookup {id}: failed after {ms} ms: {e}");
                state.update(&handle, id, |s| {
                    s.status = Status::Error;
                    s.error_kind = Some(ErrorKind::Api);
                    s.error = e.to_string();
                })
            }
        }
    });
    if let Ok(mut slot) = state.task.lock() {
        *slot = Some(task);
    }
}

/// Applies `change` and writes the settings. Saved first: on a failed save
/// the settings stay as they were and the error comes back.
pub(crate) fn save_settings(
    app: &AppHandle,
    change: impl FnOnce(&mut Settings) -> Result<(), String>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?;
    let mut next = settings.clone();
    change(&mut next)?;
    if let Err(e) = settings::save(app, &next) {
        log::error!("{e}");
        return Err(e);
    }
    *settings = next;
    log::info!("settings saved");
    Ok(())
}

/// The popup's Explain switch: remembered for the next lookup when it can
/// be saved; the lookup itself runs either way.
fn remember_explain(app: &AppHandle, explain: bool) {
    let _ = save_settings(app, |s| {
        s.explain = explain;
        Ok(())
    });
}

#[tauri::command]
pub fn popup_snapshot(state: tauri::State<'_, AppState>) -> Option<Snapshot> {
    state.snapshot()
}

/// Same text again into another language or with the Explain switch. The
/// language applies to this lookup only; the next starts with the default.
#[tauri::command]
pub fn rerun(app: AppHandle, to: Lang, explain: bool) {
    remember_explain(&app, explain);
    let Some(snap) = app.state::<AppState>().snapshot() else {
        return;
    };
    if snap.text.is_empty() {
        return;
    }
    let kind = kind_for(&snap.text, explain);
    start(&app, snap.text, to, kind, snap.from_selection);
}

/// The popup emptied by Clear: lookup `id` with no text and no answer,
/// keeping the chosen language and Explain. Typed text from here on, so
/// Replace has nothing to paste over.
fn cleared(snap: &Snapshot, id: u64) -> Snapshot {
    Snapshot {
        id,
        status: Status::Idle,
        text: String::new(),
        to: snap.to,
        targets: snap.targets.clone(),
        kind: Kind::Text,
        explain: snap.explain,
        result: String::new(),
        model: String::new(),
        ms: 0,
        error_kind: None,
        error: String::new(),
        from_selection: false,
        journal_error: String::new(),
    }
}

/// Clear in the popup: stops a running lookup and empties the original and
/// the answer.
#[tauri::command]
pub fn clear_popup(app: AppHandle) {
    let state = app.state::<AppState>();
    let Some(snap) = state.snapshot() else {
        return;
    };
    state.abort_task();
    let id = state.sessions.begin();
    state.reset(&app, cleared(&snap, id));
}

/// A language or Explain picked in a popup with no text yet.
#[tauri::command]
pub fn choose_target(app: AppHandle, to: Lang, explain: bool) {
    remember_explain(&app, explain);
    if let Some(snap) = app.state::<AppState>().choose(to, explain) {
        let _ = app.emit_to(popup::LABEL, "popup:snapshot", snap);
    }
}

/// The answer Copy and Replace may use: a finished, non-empty one.
fn finished_result(snap: &Snapshot) -> Option<String> {
    (snap.status == Status::Done && !snap.result.is_empty()).then(|| snap.result.clone())
}

/// Translates text typed (or edited) in the popup into `to`.
/// `from_selection`: the edited text still stands for a selection Replace can paste over.
#[tauri::command]
pub fn translate_text(
    app: AppHandle,
    text: String,
    to: Lang,
    explain: bool,
    from_selection: bool,
) -> Result<(), String> {
    let text = match prepare_text(&text) {
        Ok(text) => text,
        Err(TextError::Blank) => return Ok(()),
        Err(e) => return Err(e.to_string()),
    };
    remember_explain(&app, explain);
    let kind = kind_for(&text, explain);
    start(&app, text, to, kind, from_selection);
    Ok(())
}

#[tauri::command]
pub fn copy_result(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    // Only a finished answer: never a half-streamed one or an empty string.
    let Some(result) = app
        .state::<AppState>()
        .snapshot()
        .and_then(|s| finished_result(&s))
    else {
        return Ok(());
    };
    app.clipboard()
        .write_text(result)
        .map_err(|e| e.to_string())?;
    // A pinned popup stays open.
    if !popup::is_pinned() {
        popup::hide(&app);
    }
    Ok(())
}

/// Copies text the user selected in the popup. The app has no Edit menu, so
/// the webview's own ⌘C does nothing; the page sends the selection here.
#[tauri::command]
pub fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

/// Clipboard text for ⌘V in the popup's source field (same reason as above).
#[tauri::command]
pub fn clipboard_text(app: AppHandle) -> String {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().unwrap_or_default()
}

/// Pastes the answer over the selection, only while the app and window the
/// text came from are still in front (a pinned popup outlives a switch to
/// another document); otherwise the answer stays for Copy.
#[tauri::command]
pub fn replace_result(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let Some(snap) = state.snapshot() else {
        return Ok(());
    };
    let Some(result) = finished_result(&snap) else {
        return Ok(());
    };
    if !snap.from_selection {
        return Ok(());
    }
    let same = state
        .origin
        .lock()
        .ok()
        .and_then(|origin| Some(origin.as_ref()?.same_place(&current_origin()?)))
        .unwrap_or(false);
    if !same {
        log::info!("replace refused: the selection's app or window is no longer in front");
        return Err(
            "The text came from another app or window. Copy the translation instead.".into(),
        );
    }
    popup::hide(&app);
    // Let the source app's window take the keyboard back before ⌘V.
    thread::sleep(Duration::from_millis(80));
    paste::replace_selection(&app, &result)
}

#[tauri::command]
pub fn close_popup(app: AppHandle) {
    app.state::<AppState>().abort_task();
    popup::hide(&app);
}

#[tauri::command]
pub fn popup_dragged(app: AppHandle) {
    popup::mark_dragged(&app);
}

#[tauri::command]
pub fn resize_popup(app: AppHandle, height: f64, width: f64) {
    popup::resize(&app, height, width);
}

#[tauri::command]
pub fn open_settings(app: AppHandle, section: Option<String>) {
    if !popup::is_pinned() {
        popup::hide(&app);
    }
    crate::windows::open_settings(&app, section.as_deref());
}

/// The Pin checkbox in the popup header.
#[tauri::command]
pub fn pin_popup(app: AppHandle, pinned: bool) {
    popup::set_pinned(&app, pinned);
}

/// Whether the popup is pinned (the page asks when it loads).
#[tauri::command]
pub fn popup_pinned() -> bool {
    popup::is_pinned()
}

#[tauri::command]
pub fn open_accessibility(app: AppHandle) {
    popup::hide(&app);
    crate::selection::open_accessibility_settings();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_finished_answer_is_copied() {
        let state = AppState::new(crate::settings::defaults().clone());
        let mut snap = state.snapshot().expect("snapshot");
        snap.result = "half".into();
        snap.status = Status::Streaming;
        assert_eq!(finished_result(&snap), None);
        snap.status = Status::Error;
        assert_eq!(finished_result(&snap), None);
        snap.status = Status::Done;
        assert_eq!(finished_result(&snap), Some("half".into()));
        snap.result.clear();
        assert_eq!(finished_result(&snap), None);
    }

    #[test]
    fn snapshot_lists_targets_in_card_order() {
        let mut settings = crate::settings::defaults().clone();
        settings.add_target(Lang::De).expect("add");
        let snap = fresh(7, &settings, Lang::Ru, Status::Idle);
        assert_eq!(snap.targets, vec![Lang::En, Lang::Ru, Lang::Pl, Lang::De]);
        assert_eq!(snap.to, Lang::Ru);
        assert_eq!(snap.id, 7);
    }

    fn snapshot_for(state: &AppState, id: u64) -> Snapshot {
        let mut s = state.snapshot().expect("snapshot");
        s.id = id;
        s.status = Status::Streaming;
        s.result.clear();
        s
    }

    #[test]
    fn choice_in_an_empty_popup_is_kept() {
        let state = AppState::new(crate::settings::defaults().clone());
        let a = state.sessions.begin();
        let mut idle = snapshot_for(&state, a);
        idle.status = Status::Idle;
        state.replace(idle);
        let snap = state.choose(Lang::Pl, true).expect("snapshot");
        assert_eq!((snap.to, snap.explain), (Lang::Pl, true));
        assert_eq!(state.snapshot().expect("snapshot").to, Lang::Pl);
    }

    #[test]
    fn clearing_keeps_language_and_explain_only() {
        let state = AppState::new(crate::settings::defaults().clone());
        let mut snap = snapshot_for(&state, 3);
        snap.text = "привет".into();
        snap.result = "hello".into();
        snap.status = Status::Done;
        snap.to = Lang::De;
        snap.explain = true;
        snap.from_selection = true;
        let empty = cleared(&snap, 4);
        assert_eq!((empty.id, empty.status), (4, Status::Idle));
        assert_eq!((empty.to, empty.explain), (Lang::De, true));
        assert_eq!(empty.targets, snap.targets);
        assert!(empty.text.is_empty() && empty.result.is_empty());
        assert!(!empty.from_selection);
    }

    #[test]
    fn delta_of_current_lookup_lands() {
        let state = AppState::new(crate::settings::defaults().clone());
        let a = state.sessions.begin();
        state.replace(snapshot_for(&state, a));
        assert!(state.apply(a, |s| s.result.push_str("hi")).is_some());
        assert_eq!(state.snapshot().expect("snapshot").result, "hi");
    }

    #[test]
    fn delta_of_old_lookup_after_restart_is_dropped() {
        let state = AppState::new(crate::settings::defaults().clone());
        let a = state.sessions.begin();
        state.replace(snapshot_for(&state, a));
        let b = state.sessions.begin();
        state.replace(snapshot_for(&state, b));
        assert!(state.apply(a, |s| s.result.push_str("old")).is_none());
        assert_eq!(state.snapshot().expect("snapshot").result, "");
    }

    #[test]
    fn delta_before_its_snapshot_is_reset_is_dropped() {
        // The id is current but the popup still shows the previous lookup.
        let state = AppState::new(crate::settings::defaults().clone());
        let a = state.sessions.begin();
        assert!(state.apply(a, |s| s.result.push_str("early")).is_none());
        assert_eq!(state.snapshot().expect("snapshot").result, "");
    }
}
