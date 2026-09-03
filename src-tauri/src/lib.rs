mod capture;
mod domain;

use anyhow::Result;
use chrono::Local;
use parking_lot::RwLock;
use serde::Serialize;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartExt};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

use capture::{
    CaptureSnapshot, CaptureSource, CaptureSourceKind, ObsCapabilitiesSnapshot, PendingClip,
    engine::CaptureController,
};
use domain::{
    library::{LibraryItem, LibraryStore, MediaKind, TrimRequest},
    settings::{AppSettings, SettingsStore},
};

pub struct AppState {
    settings_store: SettingsStore,
    settings: RwLock<AppSettings>,
    library: RwLock<LibraryStore>,
    capture: CaptureController,
    auto_source: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSnapshot {
    capture: CaptureSnapshot,
    settings: AppSettings,
    recent: Vec<LibraryItem>,
    version: String,
}

#[tauri::command]
fn app_snapshot(state: tauri::State<'_, AppState>) -> Result<AppSnapshot, String> {
    let settings = state.settings.read().clone();
    let recent = state
        .library
        .read()
        .list()
        .map_err(err)?
        .into_iter()
        .take(4)
        .collect();
    Ok(AppSnapshot {
        capture: state.capture.snapshot(),
        settings,
        recent,
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> AppSettings {
    state.settings.read().clone()
}
#[tauri::command]
fn list_sources() -> Result<Vec<CaptureSource>, String> {
    capture::sources::list_sources().map_err(err)
}
#[tauri::command]
fn obs_capabilities(state: tauri::State<'_, AppState>) -> Result<ObsCapabilitiesSnapshot, String> {
    state.capture.capabilities()
}
#[tauri::command]
fn toggle_replay(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.capture.toggle_replay()?;
    emit_capture(&app, &state.capture);
    Ok(())
}
#[tauri::command]
fn select_source(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    source: CaptureSource,
) -> Result<(), String> {
    state
        .auto_source
        .store(source.kind == CaptureSourceKind::Auto, Ordering::Relaxed);
    state.capture.select(source).map_err(|e| e.to_string())?;
    emit_capture(&app, &state.capture);
    Ok(())
}
#[tauri::command]
async fn save_clip(app: AppHandle) -> Result<(), String> {
    queue_clip(app).await
}
#[tauri::command]
async fn toggle_recording(app: AppHandle) -> Result<(), String> {
    toggle_recording_inner(app).await
}
#[tauri::command]
fn library_list(state: tauri::State<'_, AppState>) -> Result<Vec<LibraryItem>, String> {
    state.library.read().list().map_err(err)
}
#[tauri::command]
fn library_delete(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.library.read().delete(&id).map_err(err)?;
    let _ = app.emit("library://changed", ());
    Ok(())
}
#[tauri::command]
fn library_rename(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    title: String,
) -> Result<(), String> {
    state.library.read().rename(&id, &title).map_err(err)?;
    let _ = app.emit("library://changed", ());
    Ok(())
}
#[tauri::command]
fn library_protect(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    protected_value: bool,
) -> Result<(), String> {
    state
        .library
        .read()
        .protect(&id, protected_value)
        .map_err(err)?;
    let _ = app.emit("library://changed", ());
    Ok(())
}
#[tauri::command]
fn library_reveal(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let p = state.library.read().path_for(&id).map_err(err)?;
    app.opener()
        .reveal_item_in_dir(p)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn trim_clip(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    request: TrimRequest,
) -> Result<LibraryItem, String> {
    let store = state.library.read().clone();
    let item = store.trim(request).await.map_err(err)?;
    let _ = app.emit("library://changed", ());
    Ok(item)
}
#[tauri::command]
fn complete_onboarding(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    mut settings: AppSettings,
) -> Result<(), String> {
    settings.onboarding_complete = true;
    apply_settings(&app, &state, settings)
}
#[tauri::command]
fn update_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let settings = settings.validated();
    apply_settings(&app, &state, settings.clone())?;
    Ok(settings)
}

fn apply_settings(app: &AppHandle, state: &AppState, settings: AppSettings) -> Result<(), String> {
    let previous = state.settings.read().clone();
    let new_library = LibraryStore::new(&settings.storage.root).map_err(err)?;
    let buffer_dir = new_library.buffer_dir().map_err(err)?;
    app.asset_protocol_scope()
        .allow_directory(&settings.storage.root, true)
        .map_err(err)?;

    register_shortcuts(app, &settings)?;
    if let Err(error) = state.capture.reconfigure(settings.clone(), buffer_dir) {
        let _ = register_shortcuts(app, &previous);
        return Err(error);
    }

    state.settings_store.save(&settings).map_err(err)?;
    if settings.behavior.start_with_system {
        let _ = app.autolaunch().enable();
    } else {
        let _ = app.autolaunch().disable();
    }
    *state.library.write() = new_library;
    *state.settings.write() = settings.clone();
    emit_capture(app, &state.capture);
    Ok(())
}

async fn queue_clip(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().clone();
    if !state.capture.snapshot().replay_active {
        return Err("Replay buffer is not ready yet".into());
    }
    let id = Uuid::new_v4().to_string();
    let now = Local::now().timestamp_millis();
    let saves_at = now + settings.replay.post_roll_seconds as i64 * 1000;
    state.capture.add_pending(PendingClip {
        id: id.clone(),
        requested_at: now,
        saves_at,
    });
    emit_capture(&app, &state.capture);
    feedback(
        &app,
        &settings,
        "Clip queued",
        &format!("Saving in {}s", settings.replay.post_roll_seconds),
    );
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(settings.replay.post_roll_seconds)).await;
        let state = app_clone.state::<AppState>();
        let result = state.capture.save_replay();
        state.capture.remove_pending(&id);
        match result {
            Ok(path) => {
                let snap = state.capture.snapshot();
                let game = snap
                    .source
                    .as_ref()
                    .map(|s| s.label.as_str())
                    .unwrap_or("Manual capture");
                let duration = Some(
                    (settings.replay.pre_roll_seconds + settings.replay.post_roll_seconds) * 1000,
                );
                match state
                    .library
                    .read()
                    .register(&path, MediaKind::Clip, game, duration)
                {
                    Ok(_) => {
                        feedback(&app_clone, &settings, "Clip saved", game);
                        let _ = app_clone.emit("library://changed", ());
                    }
                    Err(e) => feedback(
                        &app_clone,
                        &settings,
                        "Clip could not be filed",
                        &e.to_string(),
                    ),
                }
            }
            Err(e) => feedback(&app_clone, &settings, "Clip failed", &e),
        }
        emit_capture(&app_clone, &state.capture);
    });
    Ok(())
}

async fn toggle_recording_inner(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let before = state.capture.snapshot();
    let settings = state.settings.read().clone();
    let stopped_path = state
        .capture
        .toggle_recording()
        .map_err(|e| e.to_string())?;
    let after = state.capture.snapshot();
    if let Some(path) = stopped_path {
        let game = before
            .source
            .as_ref()
            .map(|s| s.label.as_str())
            .unwrap_or("Manual capture");
        let duration = before
            .recording_started_at
            .map(|start| (Local::now().timestamp_millis() - start).max(0) as u64);
        state
            .library
            .read()
            .register(&path, MediaKind::Recording, game, duration)
            .map_err(err)?;
        if let Some(budget) = settings.storage.recording_budget_gb {
            let _ = state.library.read().enforce_recording_budget(budget);
        }
        let _ = app.emit("library://changed", ());
        feedback(&app, &settings, "Recording saved", game);
    } else {
        feedback(
            &app,
            &settings,
            "Recording started",
            after
                .source
                .as_ref()
                .map(|s| s.label.as_str())
                .unwrap_or("Current source"),
        );
    }
    emit_capture(&app, &state.capture);
    Ok(())
}

fn feedback(app: &AppHandle, settings: &AppSettings, title: &str, body: &str) {
    if settings.notifications.desktop {
        let _ = app.notification().builder().title(title).body(body).show();
    }
    if settings.notifications.in_app {
        let _ = app.emit(
            "feedback://toast",
            serde_json::json!({"title":title,"body":body}),
        );
    }
    if settings.notifications.sound {
        play_feedback_sound();
    }
}
#[cfg(windows)]
fn play_feedback_sound() {
    use windows::Win32::{System::Diagnostics::Debug::MessageBeep, UI::WindowsAndMessaging::MB_OK};
    unsafe {
        let _ = MessageBeep(MB_OK);
    }
}
#[cfg(target_os = "linux")]
fn play_feedback_sound() {
    let _ = std::process::Command::new("canberra-gtk-play")
        .args(["-i", "complete"])
        .spawn();
}
#[cfg(not(any(windows, target_os = "linux")))]
fn play_feedback_sound() {}
fn emit_capture(app: &AppHandle, capture: &CaptureController) {
    let _ = app.emit("capture://state", capture.snapshot());
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn register_shortcuts(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    shortcuts.unregister_all().map_err(err)?;
    if let Err(error) = shortcuts.on_shortcut(
        settings.hotkeys.save_clip.as_str(),
        |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = queue_clip(app).await;
                });
            }
        },
    ) {
        return Err(format!(
            "Could not register save-clip hotkey '{}': {error}",
            settings.hotkeys.save_clip
        ));
    }
    if let Err(error) = shortcuts.on_shortcut(
        settings.hotkeys.toggle_recording.as_str(),
        |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_recording_inner(app).await;
                });
            }
        },
    ) {
        let _ = shortcuts.unregister_all();
        return Err(format!(
            "Could not register recording hotkey '{}': {error}",
            settings.hotkeys.toggle_recording
        ));
    }
    Ok(())
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };
    let open = MenuItem::with_id(app, "open", "Open Clipture", true, None::<&str>)?;
    let clip = MenuItem::with_id(app, "clip", "Save clip", true, None::<&str>)?;
    let record = MenuItem::with_id(app, "record", "Start / stop recording", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &clip, &record, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "clip" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = queue_clip(app).await;
                });
            }
            "record" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_recording_inner(app).await;
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn start_auto_detection(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_source_id: Option<String> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let state = app.state::<AppState>();
            if !state.auto_source.load(Ordering::Relaxed) {
                continue;
            }
            if !state.capture.snapshot().pending_clips.is_empty() {
                continue;
            }
            let method = state.settings.read().advanced.capture_method;
            let current = capture::sources::resolve_auto(method);
            let source_id = current.as_ref().map(|source| source.id.clone());
            if source_id == last_source_id {
                continue;
            }

            let auto_record = state.settings.read().recording.auto_record_games;
            if state.capture.snapshot().recording_active
                && auto_record
                && let Err(error) = toggle_recording_inner(app.clone()).await
            {
                log::warn!("Could not stop the previous automatic recording: {error}");
                continue;
            }

            let state = app.state::<AppState>();
            let switched = match current {
                Some(source) => match state.capture.select(source) {
                    Ok(()) => {
                        if auto_record
                            && !state.capture.snapshot().recording_active
                            && let Err(error) = toggle_recording_inner(app.clone()).await
                        {
                            log::warn!("Could not start automatic recording: {error}");
                        }
                        true
                    }
                    Err(error) => {
                        log::warn!("Automatic capture source switch failed: {error}");
                        false
                    }
                },
                None => match state.capture.select(CaptureSource {
                    id: "auto".into(),
                    kind: CaptureSourceKind::Auto,
                    label: "Automatic".into(),
                    subtitle: None,
                    pid: None,
                    is_game: None,
                    native_id: None,
                }) {
                    Ok(()) => true,
                    Err(error) => {
                        log::warn!("Could not clear the automatic capture source: {error}");
                        false
                    }
                },
            };
            if switched {
                last_source_id = source_id;
            }
            emit_capture(&app, &app.state::<AppState>().capture);
        }
    });
}

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            #[cfg(windows)]
            tauri::async_runtime::block_on(async {
                libobs_bootstrapper::ObsBootstrapper::bootstrap(
                    &libobs_bootstrapper::ObsBootstrapperOptions::default(),
                )
                .await
            })
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let settings_store = SettingsStore::new()?;
            let settings = settings_store.load();
            let library = LibraryStore::new(&settings.storage.root)?;
            app.asset_protocol_scope()
                .allow_directory(&settings.storage.root, true)?;
            let buffer = library.buffer_dir()?;
            let capture = CaptureController::spawn(settings.clone(), buffer);
            let state = AppState {
                settings_store,
                settings: RwLock::new(settings.clone()),
                library: RwLock::new(library),
                capture: capture.clone(),
                auto_source: AtomicBool::new(true),
            };
            app.manage(state);
            register_shortcuts(app.handle(), &settings).map_err(anyhow::Error::msg)?;
            setup_tray(app)?;
            if std::env::args().any(|argument| argument == "--minimized")
                && let Some(window) = app.get_webview_window("main")
            {
                let _ = window.hide();
            }
            start_auto_detection(app.handle().clone());
            if settings.replay.auto_start {
                let _ = capture.select(CaptureSource {
                    id: "auto".into(),
                    kind: CaptureSourceKind::Auto,
                    label: "Automatic".into(),
                    subtitle: None,
                    pid: None,
                    is_game: None,
                    native_id: None,
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let behavior = window.state::<AppState>().settings.read().behavior.clone();
                if !behavior.close_behavior_confirmed {
                    api.prevent_close();
                    let window = window.clone();
                    let app = window.app_handle().clone();
                    app.dialog()
                        .message("Keep Clipture running in the tray when you close the window? Capture and global hotkeys can stay active in the background.")
                        .title("Keep Clipture ready?")
                        .buttons(MessageDialogButtons::OkCancelCustom(
                            "Keep running".into(),
                            "Quit Clipture".into(),
                        ))
                        .show(move |keep_running| {
                            let state = app.state::<AppState>();
                            let mut settings = state.settings.read().clone();
                            settings.behavior.close_behavior_confirmed = true;
                            settings.behavior.close_to_tray = keep_running;
                            let _ = state.settings_store.save(&settings);
                            *state.settings.write() = settings;
                            if keep_running {
                                let _ = window.hide();
                            } else {
                                app.exit(0);
                            }
                        });
                } else if behavior.close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_snapshot,
            get_settings,
            list_sources,
            obs_capabilities,
            toggle_replay,
            select_source,
            save_clip,
            toggle_recording,
            library_list,
            library_delete,
            library_rename,
            library_protect,
            library_reveal,
            trim_clip,
            complete_onboarding,
            update_settings
        ]);
    builder
        .run(tauri::generate_context!())
        .expect("error while running Clipture");
}
