use crate::core::game_detection::GameEventNotifier;
use async_stream::stream;
use rspc::{Router, RouterBuilder};

use crate::utils::consts::GAME_DETECTION;

pub fn game_detect() -> RouterBuilder {
    <Router>::new().subscription("game_open", |t| {
        t(|_ctx, _input: ()| {
            stream! {
                    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
                    let _guard = tokio::task::spawn(async move {
                        let game = GAME_DETECTION.read().await;

                        let mut listener = None;
                        if let Some(game) = game.as_ref() {
                            let s = game
                                .add_listener(move |event| {
                                    let tx = tx.clone();
                                    async move {
                                        let r = tx.try_send(event.clone());
                                        if let Err(e) = r {
                                            log::warn!("Error sending event: {:?}", e);
                                        }
                                    }
                                })
                                .await;

                            listener = Some(s);
                        }

                    listener.map(|l| l.drop_guard())
                    }).await;

                if let Err(e) = _guard.as_ref() {
                    log::error!("Error adding listener: {:?}", e);
                }

                if let Some(_guard) = _guard.ok().flatten() {
                    log::debug!("Listener added: {:?}", _guard);
                    while let Some(event) = rx.recv().await {
                        yield event;
                    }
                } else {
                    log::error!("Failed to add listener");
                }
            }
        })
    })
}
