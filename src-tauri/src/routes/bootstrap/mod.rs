use std::{
    env::current_exe,
    sync::{atomic::AtomicBool, Arc},
};

use anyhow::Context;
use async_stream::stream;
use download::{download_obs, DownloadStatus};
use extract::extract_obs;
use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use lazy_static::lazy_static;
use obs::bootstrap_obs;
use rspc::{Router, RouterBuilder};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::{fs::remove_file, sync::broadcast};
use window::open_main_window;

use crate::utils::util::AtomicDropGuard;

mod download;
mod extract;
mod obs;
mod window;

#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub enum BootstrapStatus {
    Error(String),
    Progress(f32, String),
    Done,
}

fn verify_installation() -> anyhow::Result<bool> {
    let exe = current_exe().context("Getting current executable")?;
    let parent = exe.parent().context("Getting parent of executable")?;
    #[cfg(target_os = "windows")]
    let obs_path = parent.join("obs.dll");

    //TODO Stuff for macos / windows not tested
    #[cfg(target_os = "macos")]
    let obs_path = parent.join("obs.dylib");

    #[cfg(target_os = "linux")]
    let obs_path = parent.join("obs.so");

    Ok(obs_path.exists())
}

pub fn bootstrap_inner() -> impl Stream<Item = BootstrapStatus> {
    stream! {
        let _guard = AtomicDropGuard::new(IN_PROGRESS.clone());
        let valid_result = verify_installation();
        if let Err(err) = valid_result {
            log::error!("Error verifying installation: {:?}", err);
            yield BootstrapStatus::Error(err.to_string());
            return;
        }

        let is_valid = valid_result.unwrap();
        if !is_valid {
            yield BootstrapStatus::Progress(0.0, "Getting latest OBS release".to_string());
            let download_stream = download_obs().await;
            if let Err(e) = download_stream {
                log::error!("Error downloading OBS: {:?}", e);
                yield BootstrapStatus::Error(e.to_string());
                return;
            }

            let download_stream = download_stream.unwrap();
            pin_mut!(download_stream);
            let mut file = None;
            while let Some(status) = download_stream.next().await {
                match status {
                    DownloadStatus::Error(err) => {
                        log::error!("Error downloading OBS: {:?}", err);
                        yield BootstrapStatus::Error(err.to_string());
                        return;
                    }
                    DownloadStatus::Progress(prog, msg) => {
                        yield BootstrapStatus::Progress(prog / 3.0, msg)
                    }
                    DownloadStatus::Done(f) => {
                        yield BootstrapStatus::Progress(1.0 / 3.0, "Downloaded OBS".to_string());
                        file = Some(f);
                        break;
                    }
                }
            }

            let file = file.unwrap();
            yield BootstrapStatus::Progress(0.5, "Extracting OBS".to_string());
            let extract_stream = extract_obs(&file).await;
            if let Err(err) = extract_stream {
                log::error!("Error extracting OBS: {:?}", err);
                let _ = remove_file(&file).await;
                yield BootstrapStatus::Error(err.to_string());
                return;
            }

            let extract_stream = extract_stream.unwrap();
            pin_mut!(extract_stream);

            while let Some(status) = extract_stream.next().await {
                match status {
                    extract::ExtractStatus::Error(err) => {
                        log::error!("Error extracting OBS: {:?}", err);
                        yield BootstrapStatus::Error(err.to_string());
                        return;
                    }
                    extract::ExtractStatus::Progress(prog, msg) => {
                        yield BootstrapStatus::Progress(prog / 3.0 + 1.0 / 3.0, msg)
                    }
                }
            }
        }



        let init_stream = bootstrap_obs().await;
        pin_mut!(init_stream);
        while let Some(status) = init_stream.next().await {
            match status {
                BootstrapStatus::Error(err) => {
                    log::error!("Error initializing OBS: {:?}", err);
                    yield BootstrapStatus::Error(err);
                    return;
                }
                BootstrapStatus::Progress(prog, msg) => {
                    yield BootstrapStatus::Progress(prog / 3.0 + 2.0 / 3.0, msg)
                }
                BootstrapStatus::Done => {
                    yield BootstrapStatus::Progress(1.0, "Obs initialized".to_string());
                },
            }
        }


        let r = open_main_window().await;
        if let Err(e) = r {
            log::error!("Error opening main window: {:?}", e);
            yield BootstrapStatus::Error(e.to_string());
            return;
        }

        yield BootstrapStatus::Done;
    }
}

lazy_static! {
    pub static ref IN_PROGRESS: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    pub static ref BOOTSTRAP_BROADCAST: (
        broadcast::Sender<BootstrapStatus>,
        broadcast::Receiver<BootstrapStatus>
    ) = tokio::sync::broadcast::channel(16);
}

pub fn bootstrap() -> RouterBuilder {
    <Router>::new().subscription("initialize", |t| {
        t(|_ctx, _input: ()| {
            let handle = tauri::async_runtime::handle();

            stream! {
                let in_progress = IN_PROGRESS.load(std::sync::atomic::Ordering::SeqCst);
                    if !in_progress {
                        handle.spawn(async move {
                            let stream = bootstrap_inner();
                            pin_mut!(stream);

                            while let Some(status) = stream.next().await {
                                let _ = BOOTSTRAP_BROADCAST.0.send(status.clone());
                            }
                        });
                    }

                    let mut rx = BOOTSTRAP_BROADCAST.0.subscribe();
                    while let Ok(status) = rx.recv().await {
                        yield status;
                    }
            }
        })
    })
}
