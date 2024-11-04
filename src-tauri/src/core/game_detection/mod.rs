use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use tauri::Manager;
use tokio::{fs, sync::RwLock, task::JoinHandle, time::Instant};

use crate::{
    json_typings::clipture_api::game::detection,
    utils::consts::{app_handle, clipture_to_url},
};

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

        let tmp = game_detection.clone();
        let refresh_handle = tokio::spawn(async move {
            loop {
                let r = Self::refresh().await;
                if let Err(e) = r {
                    log::error!("Error refreshing game detection: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }

                let (game_detection, refresh_time) = r.unwrap();
                if let Some(game_detection) = game_detection {
                    *tmp.write().await = game_detection;
                }

                tokio::time::sleep_until(refresh_time).await;
            }
        });

        Ok(Self {
            game_detection,
            refresh_handle,
        })
    }

    async fn fetch_game_detection() -> anyhow::Result<detection::Root> {
        let detection_data = reqwest::get(clipture_to_url("/api/game/detection")).await?;
        let detection_data = detection_data.error_for_status()?;
        let detection_data: detection::Root = detection_data.json().await?;

        Ok(detection_data)
    }

    // Returns as an option the detection data and when the data should be refreshed again
    async fn refresh() -> anyhow::Result<(Option<detection::Root>, Instant)> {
        // Either file can be missing or it's time to refresh
        let app = app_handle().await;
        let data = app.path().app_data_dir()?;
        let detection_file = data.join(GAME_DETECTION_FILE);

        let meta = fs::metadata(&detection_file).await;
        if let Ok(meta) = meta {
            let last_modified = meta.modified()?;
            let duration = last_modified.elapsed()?;

            if duration <= *REFRESH_INTERVAL {
                return Ok((None, Instant::now() + (*REFRESH_INTERVAL - duration)));
            }
        }

        let game_detection = Self::fetch_game_detection().await?;
        fs::write(&detection_file, serde_json::to_string(&game_detection)?).await?;

        let refresh_time = Instant::now() + *REFRESH_INTERVAL;
        return Ok((Some(game_detection), refresh_time));
    }
}
