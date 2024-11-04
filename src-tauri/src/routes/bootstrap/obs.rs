use async_stream::stream;
use futures_core::Stream;

use crate::core::obs;

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


        yield BootstrapStatus::Done;
    }
}
