use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tokio::{sync::RwLock, task::JoinHandle};

use crate::json_typings::clipture_api::game::detection;
use refresh::RefreshGameDetection;

mod refresh;

pub const GAME_DETECTION_FILE: &str = "game_detection.json";
lazy_static! {
    pub static ref REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60 * 24); // Refresh every 24 hours
}

pub struct GameDetection {
    game_detection: Arc<RwLock<detection::Root>>,
    #[allow(dead_code)]
    refresh_handle: JoinHandle<()>,
}

impl GameDetection {
    pub async fn new() -> anyhow::Result<Self> {
        let game_detection = Arc::new(RwLock::new(vec![]));

        let refresh_handle = Self::spawn_refresh_thread(game_detection.clone()).await;
        Ok(Self {
            game_detection,
            refresh_handle,
        })
    }
}
