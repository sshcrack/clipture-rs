use std::env::current_exe;

use anyhow::Context;
use async_stream::stream;
use download::{download_obs, DownloadStatus};
use extract::extract_obs;
use futures_util::{pin_mut, StreamExt};
use rspc::{Router, RouterBuilder};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::json_to_rs::*;

mod download;
mod extract;

#[derive(Type, Serialize, Deserialize)]
pub enum BootstrapStatus {
    Error(String),
    Progress(f32, String),
    Done,
}

fn verify_installation() -> anyhow::Result<bool> {
    let exe = current_exe().context("Getting current executable")?;
    let parent = exe.parent().context("Getting parent of executable")?;
    #[cfg(target_os = "windows")]
    let obs_path = parent.join("obs64.dll");

    //TODO Stuff for macos / windows not tested
    #[cfg(target_os = "macos")]
    let obs_path = parent.join("obs64.dylib");

    #[cfg(target_os = "linux")]
    let obs_path = parent.join("obs64.so");

    if !obs_path.exists() {
        Ok(false)
    } else {
        Ok(true)
    }
}

pub fn bootstrap() -> RouterBuilder {
    <Router>::new().subscription("initialize", |t| {
        t(|_ctx, _input: ()| {
            stream! {
                let valid_result = verify_installation();
                if let Err(err) = valid_result {
                    log::error!("Error verifying installation: {:?}", err);
                    yield BootstrapStatus::Error(err.to_string());
                    return;
                }

                let valid_result = valid_result.unwrap();
                if !valid_result {
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
                            },
                            DownloadStatus::Progress(prog, msg) => yield BootstrapStatus::Progress(prog / 2.0, msg),
                            DownloadStatus::Done(f) => {
                                file = Some(f);
                                break;
                            },
                        }
                    }

                    yield BootstrapStatus::Progress(0.5, "Extracting OBS".to_string());
                    let extract_stream = extract_obs(file.unwrap()).await;
                    if let Err(err) = extract_stream {
                        log::error!("Error extracting OBS: {:?}", err);
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
                            },
                            extract::ExtractStatus::Progress(prog, msg) => yield BootstrapStatus::Progress(prog / 2.0 + 0.5, msg),
                        }
                    }

                    // Extract OBS
                    yield BootstrapStatus::Progress(1.0, "Done".to_string());
                }

                yield BootstrapStatus::Done;
            }
        })
    })
}
