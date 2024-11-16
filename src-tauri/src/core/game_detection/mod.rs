use std::{
    collections::HashMap,
    fmt::{self, Debug, Formatter},
    fs,
    future::Future,
    sync::Arc,
    time::Duration,
};

use lazy_static::lazy_static;
use libobs_window_helper::WindowInfo;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::RwLock;
use tokio_util::sync::{CancellationToken, DropGuard};
use uuid::Uuid;

use crate::json_typings::clipture_api::game::detection;
use refresh::RefreshGameDetection;

mod event;
pub use event::GameEventNotifier;

mod refresh;

pub const GAME_DETECTION_FILE: &str = "game_detection.json";
lazy_static! {
    pub static ref REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60 * 24); // Refresh every 24 hours
}

pub type ListenerPtr<T> =
    dyn Fn(T) -> Box<dyn Future<Output = ()> + Unpin + Send> + Send + Sync + 'static;
pub type ListenerPtrBoxed<T> = Box<ListenerPtr<T>>;
pub type ListenersTypeRw = Arc<RwLock<HashMap<Uuid, ListenerPtrBoxed<GameEvent>>>>;
pub type GameDetectionTypeRw = Arc<RwLock<detection::Root>>;

#[derive(Clone)]
pub struct ListenerRef {
    key: Uuid,
    map: ListenersTypeRw,
}

impl Debug for ListenerRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListenerRef")
            .field("key", &self.key)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ListenerDropGuard(ListenerRef);

impl ListenerRef {
    pub async fn remove(self) {
        self.map.write().await.remove(&self.key);
    }

    pub fn drop_guard(self) -> ListenerDropGuard {
        ListenerDropGuard(self)
    }
}

impl Drop for ListenerDropGuard {
    fn drop(&mut self) {
        let r = self.0.clone();
        tokio::spawn(async move {
            log::debug!("Dropping listener: {:?}", r.key);
            r.remove().await;
        });
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum WindowType {
    Game,
    Window,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    Closed(WindowInfo),
    Opened(WindowType, WindowInfo),
}

pub struct GameDetection {
    //TODO Maybe use later? Used in other threads for sure
    #[allow(dead_code)]
    game_detection: GameDetectionTypeRw,
    listeners: ListenersTypeRw,
    _token: DropGuard,
}

impl GameDetection {
    pub async fn initialize() -> anyhow::Result<Self> {
        let detection_file = Self::get_detection_file().await?;
        let detection_str = fs::read_to_string(detection_file).ok();
        let detection = detection_str
            .map(|s| serde_json::from_str::<detection::Root>(&s))
            .transpose()?
            .unwrap_or_default();

        let game_detection = Arc::new(RwLock::new(detection));
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
}
