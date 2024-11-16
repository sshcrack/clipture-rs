use std::sync::Arc;

use anyhow::{anyhow, bail};
use lazy_static::lazy_static;
use log::debug;
use tauri::async_runtime::JoinHandle;
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot, Mutex, RwLock,
};

use super::ObsManager;

pub struct RunObsFunc(pub Box<dyn FnOnce(&mut ObsManager) + Send>);
unsafe impl Sync for RunObsFunc {}

pub struct ObsRuntime {
    handle: Option<JoinHandle<()>>,
}

impl ObsRuntime {
    fn new() -> Self {
        Self { handle: None }
    }

    /// ONLY RUN ON OBS THREAD
    async fn startup(&mut self) -> anyhow::Result<UnboundedSender<RunObsFunc>> {
        if self.handle.is_some() {
            bail!("OBS is already running");
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<RunObsFunc>();

        let (init_tx, init_rx) = oneshot::channel();
        let h = tauri::async_runtime::spawn_blocking(move || {
            let ctx = ObsManager::initialize_obs();
            if let Err(e) = ctx {
                let _ = init_tx.send(Err(e));
                return;
            }

            let mut ctx = ctx.unwrap();
            init_tx.send(Ok(())).unwrap();
            loop {
                let f: Option<RunObsFunc> = rx.blocking_recv();

                if f.is_none() {
                    return;
                }

                f.unwrap().0(&mut ctx);
            }
        });

        self.handle = Some(h);
        init_rx.await??;

        Ok(tx)
    }
}

lazy_static! {
    static ref __OBS_RUNTIME: Arc<Mutex<ObsRuntime>> = Arc::new(Mutex::new(ObsRuntime::new()));
    static ref __OBS_RUNTIME_SENDER: Arc<RwLock<Option<UnboundedSender<RunObsFunc>>>> =
        Arc::new(RwLock::new(None));
}

pub async fn startup_obs() -> anyhow::Result<()> {
    debug!("Starting OBS runtime");
    let sender = __OBS_RUNTIME.lock().await.startup().await?;

    debug!("Writing sender...");
    __OBS_RUNTIME_SENDER.write().await.replace(sender);

    debug!("Done.");
    Ok(())
}

pub async fn run_with_obs<
    T: Send + 'static,
    F: FnOnce(&mut ObsManager) -> anyhow::Result<T> + Send + 'static,
>(
    f: F,
) -> anyhow::Result<T> {
    let (tx, rx) = oneshot::channel();

    let f = move |ctx: &mut ObsManager| {
        let _ = tx.send(f(ctx));
    };

    __OBS_RUNTIME_SENDER
        .read()
        .await
        .as_ref()
        .expect("OBS must be initialized to run on obs thread")
        .send(RunObsFunc(Box::new(f)))
        .map_err(|e| anyhow!("{}", e.to_string()))?;

    Ok(rx.await??)
}
