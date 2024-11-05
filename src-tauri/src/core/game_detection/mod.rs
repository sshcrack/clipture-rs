use std::{collections::HashMap, sync::Arc, time::Duration, vec};

use lazy_static::lazy_static;
use libobs_window_helper::WindowInfo;
use tokio::sync::RwLock;
use tokio_util::sync::{CancellationToken, DropGuard};
use uuid::Uuid;

use crate::json_typings::clipture_api::game::detection;
use refresh::RefreshGameDetection;
use event::GameEventNotifier;

mod refresh;
mod event;

pub const GAME_DETECTION_FILE: &str = "game_detection.json";
lazy_static! {
    pub static ref REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60 * 24); // Refresh every 24 hours
}

pub type ListenerPtr<T> = Box<dyn Fn(&T) + Send + Sync + 'static>;
pub type ListenersTypeRw = Arc<RwLock<HashMap<Uuid, ListenerPtr<GameEvent>>>>;
pub type GameDetectionTypeRw = Arc<RwLock<detection::Root>>;

#[derive(Debug, Clone)]
pub enum WindowType {
    Game,
    Window
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    Closed(WindowInfo),
    Opened(WindowType, WindowInfo),
}

pub struct GameDetection {
    game_detection: GameDetectionTypeRw,
    listeners: ListenersTypeRw,
    _token: DropGuard,
}

impl GameDetection {
    pub async fn initialize() -> anyhow::Result<Self> {
        let game_detection = Arc::new(RwLock::new(vec![]));
        let listeners = Arc::new(RwLock::new(HashMap::new()));

        let token = CancellationToken::new();
        Self::spawn_refresh_file_thread(token.clone(), game_detection.clone()).await;
        Self::spawn_event_thread(token.clone(), listeners.clone(), game_detection.clone()).await;

        let s = Self {
            game_detection,
            listeners,
            _token: token.drop_guard(),
        };

        Ok(s)
    }

    #[allow(dead_code)]
    pub async fn add_listener(&mut self, listener: ListenerPtr<GameEvent>) -> Uuid {
        let token = Uuid::new_v4();
        self.listeners.write().await.insert(token.clone(), listener);

        token
    }

    #[allow(dead_code)]
    pub async fn remove_listener(&mut self, token: Uuid) {
        self.listeners.write().await.remove(&token);
    }
}
