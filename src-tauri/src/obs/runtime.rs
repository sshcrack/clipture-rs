use std::sync::Arc;

use anyhow::anyhow;
use lazy_static::lazy_static;
use libobs_wrapper::context::ObsContext;
use tauri::async_runtime::JoinHandle;
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot, Mutex,
};

use super::initialize_obs;

pub struct ObsRuntime {
    _handle: JoinHandle<()>,
    ctx: Option<ObsContext>,
}

impl ObsRuntime {
    fn new() -> (Self, UnboundedSender<Box<dyn FnOnce() + Send>>) {
        let (tx, mut rx) = mpsc::unbounded_channel();

        let handle = tauri::async_runtime::spawn_blocking(move || loop {
            let f: Option<Box<dyn FnOnce() + Send>> = rx.blocking_recv();

            if f.is_none() {
                return;
            }

            f.unwrap()();
        });

        (
            Self {
                _handle: handle,
                ctx: None,
            },
            tx,
        )
    }

    fn startup(&mut self) -> anyhow::Result<()> {
        if self.ctx.is_some() {
            return Ok(());
        }

        let ctx = initialize_obs("recording.mp4")?;
        self.ctx = Some(ctx);

        Ok(())
    }
}

lazy_static! {
    static ref __OBS_RUNTIME: Arc<Mutex<(ObsRuntime, UnboundedSender<Box<dyn FnOnce() + Send>>)>> =
        Arc::new(Mutex::new(ObsRuntime::new()));
}

pub async fn startup_obs() -> anyhow::Result<()> {
    let (tx, rx) = oneshot::channel();
    run_on_obs_thread(|| {
        let mut obs = __OBS_RUNTIME.blocking_lock();

        let r = obs.0.startup();
        let _ = tx.send(r);
    })
    .await?;

    rx.await??;
    Ok(())
}

pub async fn run_on_obs_thread<F: FnOnce() + Send + 'static>(f: F) -> anyhow::Result<()> {
    __OBS_RUNTIME
        .lock()
        .await
        .1
        .send(Box::new(f))
        .map_err(|e| anyhow!("{}", e.to_string()))?;

    Ok(())
}

#[allow(dead_code)]
pub async fn run_with_obs<
    T: Send + 'static,
    F: FnOnce(&mut ObsContext) -> anyhow::Result<T> + Send + 'static,
>(
    f: F,
) -> anyhow::Result<T> {
    let (tx, rx) = oneshot::channel();
    run_on_obs_thread(|| {
        let mut obs = __OBS_RUNTIME.blocking_lock();

        let ctx = obs
            .0
            .ctx
            .as_mut()
            .expect("Should not call run_with_obs before startup");

        let _ = tx.send(f(ctx));
    })
    .await?;

    rx.await.unwrap()
}
