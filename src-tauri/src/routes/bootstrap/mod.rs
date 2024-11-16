use std::{
    path::Path,
    process::{exit, Command},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use async_stream::stream;
use download::{download_obs, DownloadStatus};
use extract::extract_obs;
use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use lazy_static::lazy_static;
use obs::bootstrap_obs;
use rspc::{ErrorCode, Router, RouterBuilder};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{process::current_binary, Manager};
use tokio::{fs::remove_file, sync::broadcast};
use verify::verify_installation;
use window::{create_main_window, open_main_window};

use crate::utils::{
    consts::{app_handle, APP_HANDLE},
    util::AtomicDropGuard,
};

mod download;
mod extract;
mod obs;
mod verify;
mod window;

#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub enum BootstrapStatus {
    Error(String),
    Progress(f32, String),
    Done,
}

lazy_static! {
    pub static ref BOOTSTRAP_DONE: AtomicBool = AtomicBool::new(false);
}

async fn restart_with_extracted(extract_path: &Path) -> ! {
    let handle = APP_HANDLE.read().await;
    let handle = handle.as_ref().expect("Should have app handle");

    handle.cleanup_before_exit();

    let env = handle.env();
    if let Ok(path) = current_binary(&env) {
        let installation_updater = path.parent().unwrap().join("installation-updater.exe");
        if let Err(e) = Command::new(installation_updater)
            .arg(extract_path)
            .arg(path)
            .arg(std::process::id().to_string())
            .args(env.args_os.iter().skip(1).collect::<Vec<_>>())
            .spawn()
        {
            log::error!("failed to restart app: {e}");
        }
    }

    exit(0);
}

pub fn prepare_obs() -> impl Stream<Item = BootstrapStatus> {
    stream! {
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

            let mut extract_path = None;
            while let Some(status) = extract_stream.next().await {
                match status {
                    extract::ExtractStatus::Error(err) => {
                        log::error!("Error extracting OBS: {:?}", err);
                        yield BootstrapStatus::Error(err.to_string());
                        return;
                    }
                    extract::ExtractStatus::Progress(prog, msg) => {
                        yield BootstrapStatus::Progress(prog / 3.0 + 1.0 / 3.0, msg)
                    },
                    extract::ExtractStatus::Done(path) => {
                        extract_path = Some(path);
                        break;
                    }
                }
            }

            restart_with_extracted(&extract_path.expect("Should have a extract_path")).await;
    }
}

pub fn bootstrap_inner() -> impl Stream<Item = BootstrapStatus> {
    stream! {
        log::info!("Starting bootstrap");

        let _guard = AtomicDropGuard::new(IN_PROGRESS.clone());
        let valid_result = verify_installation().await;
        if let Err(err) = valid_result {
            log::error!("Error verifying installation: {:?}", err);
            yield BootstrapStatus::Error(err.to_string());
            return;
        }

        let verify_result = valid_result.unwrap();
        match verify_result {
            verify::VerifyResult::Invalid => {
                let prepare_stream = prepare_obs();
                pin_mut!(prepare_stream);

                while let Some(status) = prepare_stream.next().await {
                    yield status;
                }
            },
            verify::VerifyResult::Restored(extract_path) => {
                restart_with_extracted(&extract_path).await;
            },
            verify::VerifyResult::Ok => {}
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
                    yield BootstrapStatus::Progress(1.0, "Obs Initialized".to_string());
                }
            }
        }

        log::debug!("Bootstrap done");

        BOOTSTRAP_DONE.store(true, Ordering::Release);
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
    <Router>::new()
        .subscription("initialize", |t| {
            t(|_ctx, _input: ()| {
                let handle = tauri::async_runtime::handle();

                stream! {
                    let in_progress = IN_PROGRESS.load(Ordering::Acquire);
                    if !in_progress {
                        IN_PROGRESS.store(true, Ordering::Release);
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
        .query("show_or_create_main", |t| {
            t(|_ctx, _input: ()| async {
                let handle = app_handle().await;
                let exists = handle.webview_windows().get("main").is_some();

                let r = if exists {
                    open_main_window().await
                } else {
                    create_main_window().await
                };
                if let Err(e) = r {
                    log::error!("Error opening main window: {:?}", e);
                    return Err(rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("{}", e),
                    ));
                }

                Ok(())
            })
        })
}
