use std::future::Future;

use async_trait::async_trait;
use libobs_window_helper::{get_all_windows, WindowInfo, WindowSearchMode};
use tokio::{pin, select, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    core::game_detection::{GameEvent, WindowType},
    json_typings::clipture_api::game::detection,
};

use super::{GameDetection, GameDetectionTypeRw, ListenerRef, ListenersTypeRw};

#[async_trait]
pub trait GameEventNotifier {
    async fn add_listener<
        F: Future<Output = ()> + Send + 'static,
        T: (Fn(GameEvent) -> F) + Send + Sync + 'static,
    >(
        &self,
        listener: T,
    ) -> ListenerRef;
    fn get_window_type(info: &WindowInfo, data: &detection::Root) -> WindowType;

    async fn spawn_event_thread(
        token: CancellationToken,
        listeners: ListenersTypeRw,
        detection: GameDetectionTypeRw,
    ) -> JoinHandle<()>;
}

#[async_trait]
impl GameEventNotifier for GameDetection {
    async fn add_listener<
        F: Future<Output = ()> + Send + 'static,
        T: (Fn(GameEvent) -> F) + Send + Sync + 'static,
    >(
        &self,
        listener: T,
    ) -> ListenerRef {
        let token = Uuid::new_v4();
        self.listeners.write().await.insert(token.clone(), Box::new(move |e| Box::new(Box::pin(listener(e)))));

        ListenerRef {
            key: token.clone(),
            map: self.listeners.clone(),
        }
    }

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
                    if !prev.iter().any(|x: &WindowInfo| x.pid == window.pid) {
                        let window_type = Self::get_window_type(&window, &detection);

                        for listener in listeners.values() {
                            let r =
                                listener(GameEvent::Opened(window_type.clone(), window.clone()));
                            pin!(r);

                            r.await
                        }
                    }

                    if !windows.iter().any(|x: &WindowInfo| x.pid == window.pid) {
                        for listener in listeners.values() {
                            let r = listener(GameEvent::Closed(window.clone()));
                            pin!(r);

                            r.await
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
        for game in data.iter() {
            for exe in game.executables.iter() {
                let full_exe = window.full_exe.to_lowercase().replace("\\", "/");
                if !full_exe.ends_with(&exe.name.to_lowercase()) {
                    continue;
                }
                if let Some(args) = exe.arguments.as_ref() {
                    // Maybe if cmdline is not present, we should not check for it?
                    if !window
                        .cmd_line
                        .as_ref()
                        .is_some_and(|e| e.to_lowercase().contains(&args.to_lowercase()))
                    {
                        continue;
                    }
                }

                return WindowType::Game;
            }
        }

        return WindowType::Window;
    }
}
