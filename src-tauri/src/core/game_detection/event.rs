use async_trait::async_trait;
use libobs_window_helper::{get_all_windows, WindowInfo, WindowSearchMode};
use tokio::{select, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{
    core::game_detection::{GameEvent, WindowType},
    json_typings::clipture_api::game::detection,
};

use super::{GameDetection, GameDetectionTypeRw, ListenersTypeRw};

#[async_trait]
pub trait GameEventNotifier {
    fn get_window_type(info: &WindowInfo, data: &detection::Root) -> WindowType;

    async fn spawn_event_thread(
        token: CancellationToken,
        listeners: ListenersTypeRw,
        detection: GameDetectionTypeRw,
    ) -> JoinHandle<()>;
}

#[async_trait]
impl GameEventNotifier for GameDetection {
    async fn spawn_event_thread(
        token: CancellationToken,
        listeners: ListenersTypeRw,
        detection: GameDetectionTypeRw,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut prev = vec![];
            loop {
                let windows = match get_all_windows(WindowSearchMode::IncludeMinimized) {
                    Ok(games) => games,
                    Err(e) => {
                        eprintln!("Error getting windows: {:?}", e);
                        continue;
                    }
                };

                let detection = detection.read().await;
                let listeners = listeners.read().await;

                for window in windows.iter() {
                    if !prev.contains(window) {
                        let window_type = Self::get_window_type(&window, &detection);

                        for listener in listeners.values() {
                            listener(&GameEvent::Opened(window_type.clone(), window.clone()));
                        }
                    }

                    if !windows.contains(window) {
                        for listener in listeners.values() {
                            listener(&GameEvent::Closed(window.clone()));
                        }
                    }
                }

                prev = windows;
                select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {}
                }
            }
        })
    }

    fn get_window_type(window: &WindowInfo, data: &detection::Root) -> WindowType {
        let mut window_type = WindowType::Window;
        for game in data.iter() {
            for exe in game.executables.iter() {
                if !window.full_exe.ends_with(&exe.name) {
                    continue;
                }

                if let Some(args) = exe.arguments.as_ref() {
                    // Maybe if cmdline is not present, we should not check for it?
                    if !window.cmd_line.as_ref().is_some_and(|e| e.contains(args)) {
                        continue;
                    }
                }

                window_type = WindowType::Game;
            }
        }

        window_type
    }
}
