use async_stream::stream;
use futures_core::Stream;

use crate::{
    core::{
        game_detection::{GameDetection, GameEvent, GameEventNotifier, WindowType},
        obs::{self, runtime::run_with_obs, CaptureTrait},
    },
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

        log::debug!("Initializing Game Detector...");
        let detector = GameDetection::initialize().await;
        if let Err(e) = detector {
            log::error!("Error initializing Game Detector: {:?}", e);
            yield BootstrapStatus::Error(e.to_string());
            return;
        }

        let detector = detector.unwrap();

        detector.add_listener(|event| async move {
            match event {
                GameEvent::Closed(window_info) => log::trace!("Game Closed: {}", window_info.obs_id),
                GameEvent::Opened(window_type, window_info) => {
                    match window_type {
                        WindowType::Game => {
                            log::trace!("Game Opened: {}", window_info.obs_id);
                            let e = run_with_obs(|mgr| {
                                mgr.switch_window(window_info)
                            }).await;

                            if let Err(e) = e {
                                log::error!("Error running with OBS: {:?}", e);
                            }
                        },
                        _ => {}
                    }
                }
            }
        }).await;
        GAME_DETECTION.write().await.replace(detector);
        yield BootstrapStatus::Done;
    }
}
