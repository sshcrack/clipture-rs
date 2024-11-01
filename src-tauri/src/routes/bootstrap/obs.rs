use async_stream::stream;
use futures_core::Stream;
use tokio::sync::oneshot;

use crate::{obs::initialize_obs, utils::consts::APP_HANDLE, __OBS_CTX};

use super::BootstrapStatus;

pub async fn bootstrap_obs() -> impl Stream<Item = BootstrapStatus> {
    stream! {
        yield BootstrapStatus::Progress(0.0, "Initializing OBS...".to_string());
        if __OBS_CTX.lock().unwrap().is_some() {
            yield BootstrapStatus::Done;
            return;
        }

        let handle = APP_HANDLE.read().await;
        let handle = handle.as_ref().expect("Should have app handle");

        let (tx, rx) = oneshot::channel();
        handle
            .run_on_main_thread(|| {
                let r = initialize_obs("recording.mp4");
                if let Err(e) = r {
                    let r = tx.send(Err(e));
                    if let Err(e) = r {
                        log::error!("Error sending OBS init: {:?}", e);
                    }
                    return;
                }

                __OBS_CTX.lock().unwrap().replace(r.unwrap());
                let r = tx.send(Ok(()));
                if let Err(e) = r {
                    log::error!("Error sending OBS init: {:?}", e);
                }
            })
            .unwrap();

        let e = rx.await;
        if let Err(e) = e {
            log::error!("Error receiving message from initialize thread: {:?}", e);
            yield BootstrapStatus::Error(e.to_string());
            return;
        }

        if let Err(e) = e {
            log::error!("Error initializing OBS: {:?}", e);
            yield BootstrapStatus::Error(e.to_string());
            return;
        }

        yield BootstrapStatus::Done;
    }
}
