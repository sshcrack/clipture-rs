use async_stream::stream;
use futures_core::Stream;

use crate::{
    core::{game_detection::GameDetection, obs},
    utils::consts::GAME_DETECTION,
};

use super::BootstrapStatus;

pub async fn bootstrap_obs() -> impl Stream<Item = BootstrapStatus> {
    stream! {
        yield BootstrapStatus::Progress(0.0, "Initializing OBS...".to_string());

        // Its fine if we call this multiple times, there is a protection in place
        let res = obs::runtime::startup_obs().await;
        if let Err(e) = res {
            log::error!("Error initializing OBS: {:?}", e);
            yield BootstrapStatus::Error(e.to_string());
            return;
        }

        yield BootstrapStatus::Progress(0.5, "Initializing Game Detector...".to_string());

        let detector = GameDetection::initialize().await;
        if let Err(e) = detector {
            log::error!("Error initializing Game Detector: {:?}", e);
            yield BootstrapStatus::Error(e.to_string());
            return;
        }

        GAME_DETECTION.write().await.replace(detector.unwrap());
        yield BootstrapStatus::Done;
    }
}
