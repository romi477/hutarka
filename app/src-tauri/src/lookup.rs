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
use crate::selection::{SelectionError, read_selection};
use crate::session::Sessions;
use crate::settings::{self, Settings, build_request};
use crate::translator::{Direction, Kind, Lang, auto_direction, kind_for, prepare_text};
use crate::{paste, popup};

/// Why a lookup produced no answer; the popup picks its message and button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    NoKey,
    NotTrusted,
    Nothing,
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
    pub direction: Direction,
    pub kind: Kind,
    pub explain: bool,
    pub result: String,
    pub model: String,
    pub ms: u64,
    pub error_kind: Option<ErrorKind>,
    pub error: String,
}

pub struct AppState {
    pub sessions: Sessions,
    pub settings: Mutex<Settings>,
    snapshot: Mutex<Snapshot>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        let snapshot = Snapshot {
            id: 0,
            status: Status::Idle,
            text: String::new(),
            direction: Direction::EnRu,
            kind: Kind::Text,
            explain: settings.explain,
            result: String::new(),
            model: String::new(),
            ms: 0,
            error_kind: None,
            error: String::new(),
        };
        AppState {
            sessions: Sessions::default(),
            settings: Mutex::new(settings),
            snapshot: Mutex::new(snapshot),
            task: Mutex::new(None),
        }
    }

    pub fn snapshot(&self) -> Option<Snapshot> {
        self.snapshot.lock().ok().map(|s| s.clone())
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Changes the snapshot of lookup `id` (ignored when stale) and sends it.
    fn update(&self, app: &AppHandle, id: u64, change: impl FnOnce(&mut Snapshot)) {
        if !self.sessions.is_current(id) {
            return;
        }
        let Ok(mut snap) = self.snapshot.lock() else {
            return;
        };
        change(&mut snap);
        let _ = app.emit_to(popup::LABEL, "popup:snapshot", snap.clone());
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
    thread::spawn(move || {
        let selection = read_selection(&app);
        let state = app.state::<AppState>();
        let settings = state.settings();
        match selection
            .map_err(|e| match e {
                SelectionError::NotTrusted => (
                    ErrorKind::NotTrusted,
                    "Hutarka needs Accessibility permission to read the selection.".to_string(),
                ),
                SelectionError::Nothing => (ErrorKind::Nothing, "Nothing selected".to_string()),
            })
            .and_then(|raw| {
                prepare_text(&raw).map_err(|e| {
                    let kind = match e {
                        crate::translator::TextError::Blank => ErrorKind::Nothing,
                        crate::translator::TextError::TooLong(_) => ErrorKind::TooLong,
                    };
                    (kind, e.to_string())
                })
            }) {
            Ok(text) => {
                let dir = auto_direction(&text, settings.english_target);
                let kind = kind_for(&text, settings.explain);
                popup::show_near_cursor(&app);
                start(&app, text, dir, kind);
            }
            Err((kind, message)) => {
                popup::show_near_cursor(&app);
                fail(&app, kind, message);
            }
        }
    });
}

/// A lookup that ends before any request (no selection, no permission).
fn fail(app: &AppHandle, kind: ErrorKind, message: String) {
    let state = app.state::<AppState>();
    state.abort_task();
    let id = state.sessions.begin();
    let explain = state.settings().explain;
    state.update(app, id, |s| {
        *s = Snapshot {
            id,
            status: Status::Error,
            text: String::new(),
            direction: s.direction,
            kind: Kind::Text,
            explain,
            result: String::new(),
            model: String::new(),
            ms: 0,
            error_kind: Some(kind),
            error: message,
        };
    });
}

/// Starts (or restarts) a lookup of `text`; any running one is cancelled.
pub fn start(app: &AppHandle, text: String, dir: Direction, kind: Kind) {
    let state = app.state::<AppState>();
    state.abort_task();
    let id = state.sessions.begin();
    let settings = state.settings();
    let req = build_request(&settings, &text, dir, kind);
    state.update(app, id, |s| {
        *s = Snapshot {
            id,
            status: Status::Streaming,
            text: text.clone(),
            direction: dir,
            kind,
            explain: settings.explain,
            result: String::new(),
            model: req.model.clone(),
            ms: 0,
            error_kind: None,
            error: String::new(),
        };
    });
    let key = settings.api_key.trim().to_string();
    if key.is_empty() {
        state.update(app, id, |s| {
            s.status = Status::Error;
            s.error_kind = Some(ErrorKind::NoKey);
            s.error = "No Anthropic API key".into();
        });
        return;
    }
    let handle = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        let client = Client::new(key, DEFAULT_BASE_URL.to_string());
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
                state.update(&handle, id, |s| {
                    s.status = Status::Done;
                    s.result = result.clone();
                    s.ms = ms;
                });
                let record = LookupRecord {
                    ts: chrono::Local::now().to_rfc3339(),
                    kind: "lookup",
                    mode: kind,
                    direction: dir,
                    text,
                    result,
                    model: req.model.clone(),
                    ms,
                };
                if let Err(e) =
                    append_lookup(&journal_root(), &record, chrono::Local::now().date_naive())
                {
                    log::error!("cannot write the lookup journal: {e}");
                }
            }
            Err(e) => state.update(&handle, id, |s| {
                s.status = Status::Error;
                s.error_kind = Some(ErrorKind::Api);
                s.error = e.to_string();
            }),
        }
    });
    if let Ok(mut slot) = state.task.lock() {
        *slot = Some(task);
    }
}

fn save_settings(app: &AppHandle, change: impl FnOnce(&mut Settings)) {
    let state = app.state::<AppState>();
    let Ok(mut settings) = state.settings.lock() else {
        return;
    };
    change(&mut settings);
    if let Err(e) = settings::save(app, &settings) {
        log::error!("cannot save settings: {e}");
    }
}

#[tauri::command]
pub fn popup_snapshot(state: tauri::State<'_, AppState>) -> Option<Snapshot> {
    state.snapshot()
}

/// Same selection again with another direction or the Explain switch.
#[tauri::command]
pub fn rerun(app: AppHandle, direction: Direction, explain: bool) {
    save_settings(&app, |s| {
        s.explain = explain;
        if direction.from() == Lang::En {
            s.english_target = direction.to();
        }
    });
    let text = app
        .state::<AppState>()
        .snapshot()
        .map(|s| s.text)
        .unwrap_or_default();
    if text.is_empty() {
        return;
    }
    let kind = kind_for(&text, explain);
    start(&app, text, direction, kind);
}

#[tauri::command]
pub fn copy_result(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let result = app
        .state::<AppState>()
        .snapshot()
        .map(|s| s.result)
        .unwrap_or_default();
    app.clipboard()
        .write_text(result)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn replace_result(app: AppHandle) -> Result<(), String> {
    let result = app
        .state::<AppState>()
        .snapshot()
        .map(|s| s.result)
        .unwrap_or_default();
    if result.is_empty() {
        return Ok(());
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
pub fn resize_popup(app: AppHandle, height: f64) {
    popup::resize(&app, height);
}

#[tauri::command]
pub fn open_settings(app: AppHandle, section: Option<String>) {
    popup::hide(&app);
    crate::windows::open_settings(&app, section.as_deref());
}

#[tauri::command]
pub fn open_accessibility(app: AppHandle) {
    popup::hide(&app);
    crate::selection::open_accessibility_settings();
}
