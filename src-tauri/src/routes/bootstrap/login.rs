use async_stream::stream;
use futures_core::Stream;

use crate::auth::AUTH_MANAGER;

pub(super) enum LoginStatus {
    Error(anyhow::Error),
    Progress(f32, String),
    Done,
}

pub fn try_login() -> impl Stream<Item = LoginStatus> {
    stream! {
        let mut msg = AUTH_MANAGER.write().await;

        let msg = msg.as_mut();
        if msg.is_none() {
            yield LoginStatus::Error(anyhow::anyhow!("AuthManager not initialized"));
            return;
        }

        let msg = msg.unwrap();
        yield LoginStatus::Progress(0.0, "Initializing login...".to_string());

        if msg.is_logged_in() {
            yield LoginStatus::Done;
            return;
        }

        let r = msg.login().await;
        if let Err(e) = r {
            log::error!("Error logging in: {:?}", e);
            yield LoginStatus::Error(e);
            return;
        }

        yield LoginStatus::Done;
    }
}
