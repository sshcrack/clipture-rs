use std::{
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
use libobs_wrapper::context::ObsContext;
use login::LoginStatus;
use obs::bootstrap_obs;
use rspc::{ErrorCode, Router, RouterBuilder};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{process::current_binary, Manager};
use tokio::{
    fs::{self, remove_file},
    sync::broadcast,
};
use window::open_main_window;

use crate::utils::{
    consts::{APP_HANDLE, INVALID_OBS_SIZE, OBS_VERSION},
    util::AtomicDropGuard,
};

mod download;
mod extract;
mod login;
mod obs;
mod window;

#[derive(Debug, Clone, Type, Serialize, Deserialize)]
pub enum BootstrapStatus {
    Error(String),
    Progress(f32, String),
    Done,
}

async fn verify_installation() -> anyhow::Result<bool> {
    let handle = APP_HANDLE.read().await;
    let handle = handle.as_ref().expect("Should have app handle");

    let binary_path = current_binary(&handle.env())?;
    let obs_path = binary_path.parent().unwrap().join("obs.dll");
    if !obs_path.exists() {
        log::debug!("obs.dll at path {:?} does not exist", obs_path.display());
        return Ok(false);
    }

    let metadata = fs::metadata(&obs_path).await?;
    if metadata.len() < INVALID_OBS_SIZE as u64 {
        log::debug!("obs.dll is invalid size: {:?}", metadata.len());
        return Ok(false);
    }

    let local_version = ObsContext::get_version();
    log::debug!("Verify version OBS: {:?}", local_version);
    let local_version = local_version.parse::<semver::Version>();

    if local_version.is_err() {
        return Ok(false);
    }

    let matches = OBS_VERSION.matches(&local_version.unwrap());
    log::debug!("Version matches: {:?}", matches);

    return Ok(matches);
}

async fn restart_with_extracted(extract_path: std::path::PathBuf) -> ! {
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

            restart_with_extracted(extract_path.expect("Should have a extract_path")).await;
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
                }
            }
        }


        let login_stream = login::try_login();
        pin_mut!(login_stream);
        while let Some(status) = login_stream.next().await {
            match status {
                LoginStatus::Error(err) => {
                    log::error!("Error initializing OBS: {:?}", err);
                    yield BootstrapStatus::Error(format!("{}", err));
                    return;
                }
                LoginStatus::Progress(prog, msg) => {
                    yield BootstrapStatus::Progress(prog / 3.0 + 2.0 / 3.0, msg)
                },
                LoginStatus::Done => {
                    yield BootstrapStatus::Progress(1.0, "Obs initialized".to_string());
                },
            }
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
        .query("show_main", |t| {
            t(|_ctx, _input: ()| async {
                let r = open_main_window().await;
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
