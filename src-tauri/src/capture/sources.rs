use super::{CaptureSource, CaptureSourceKind};
use crate::domain::settings::CaptureMethod;
use anyhow::Result;
#[cfg(windows)]
use libobs_window_helper::{WindowSearchMode, get_all_windows};

pub fn list_sources() -> Result<Vec<CaptureSource>> {
    let mut result = vec![CaptureSource {
        id: "auto".into(),
        kind: CaptureSourceKind::Auto,
        label: "Automatic".into(),
        subtitle: Some("Prefer the foreground detected game".into()),
        pid: None,
        is_game: None,
        native_id: None,
    }];

    #[cfg(windows)]
    if let Ok(windows) = get_all_windows(WindowSearchMode::IncludeMinimized) {
        for w in windows {
            let label = w
                .product_name
                .clone()
                .or(w.title.clone())
                .unwrap_or_else(|| {
                    std::path::Path::new(&w.full_exe)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Application".into())
                });
            let exe = std::path::Path::new(&w.full_exe)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            result.push(CaptureSource {
                id: format!(
                    "{}:{}:{}",
                    if w.is_game { "game" } else { "window" },
                    w.pid,
                    w.obs_id
                ),
                kind: if w.is_game {
                    CaptureSourceKind::Game
                } else {
                    CaptureSourceKind::Window
                },
                label,
                subtitle: Some(format!(
                    "{} · {}",
                    if w.is_game {
                        "Game capture"
                    } else {
                        "Window capture"
                    },
                    exe
                )),
                pid: Some(w.pid),
                is_game: Some(w.is_game),
                native_id: Some(w.obs_id),
            });
        }
    }

    if let Ok(displays) = display_info::DisplayInfo::all() {
        for (index, d) in displays.into_iter().enumerate() {
            result.push(CaptureSource {
                id: format!("monitor:{index}"),
                kind: CaptureSourceKind::Monitor,
                label: if d.name.is_empty() {
                    format!("Display {}", index + 1)
                } else {
                    d.name.clone()
                },
                subtitle: Some(format!("{} × {}", d.width, d.height)),
                pid: None,
                is_game: None,
                native_id: Some(d.name),
            });
        }
    }
    Ok(result)
}

#[cfg(windows)]
fn foreground_pid() -> Option<u32> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    let window = unsafe { GetForegroundWindow() };
    if window.0.is_null() {
        return None;
    }
    let mut pid = 0_u32;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut pid));
    }
    (pid != 0).then_some(pid)
}

pub fn resolve_auto(method: CaptureMethod) -> Option<CaptureSource> {
    let sources = list_sources().ok()?;

    #[cfg(windows)]
    let foreground = foreground_pid();
    #[cfg(not(windows))]
    let foreground: Option<u32> = None;

    let mut games = sources
        .iter()
        .filter(|source| source.kind == CaptureSourceKind::Game);
    let detected_game = foreground
        .and_then(|pid| games.clone().find(|source| source.pid == Some(pid)))
        .or_else(|| games.next())?;

    if method == CaptureMethod::Monitor {
        return sources
            .into_iter()
            .find(|source| source.kind == CaptureSourceKind::Monitor);
    }

    let mut selected = detected_game.clone();
    if method == CaptureMethod::Window {
        selected.kind = CaptureSourceKind::Window;
        selected.id = format!(
            "window:{}:{}",
            selected.pid.unwrap_or_default(),
            selected.native_id.as_deref().unwrap_or_default()
        );
        if let Some(subtitle) = &mut selected.subtitle {
            *subtitle = subtitle.replacen("Game capture", "Window capture", 1);
        }
    }
    Some(selected)
}
