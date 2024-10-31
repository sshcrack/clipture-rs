// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env::{current_exe, set_current_dir},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use lazy_static::lazy_static;
use libobs_wrapper::context::ObsContext;
use tauri::async_runtime::block_on;
use tauri_plugin_log as t_log;
use tokio::sync::oneshot;
use utils::consts::{app_handle, __APP_HANDLE};

mod crash_handler;
mod obs;
mod utils;
mod routes;

lazy_static! {
    /// DO NOT EVER RUN THIS FUNCTION ON ANY OTHER THREAD THAN THE MAIN THREAD
    /// It will cause issues, trust me
    static ref __OBS_CTX: Arc<Mutex<Option<ObsContext>>> = Arc::new(Mutex::new(None));
}

pub async fn run_obs<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut ObsContext) -> anyhow::Result<()> + Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    app_handle().await.run_on_main_thread(move || {
        let mut ctx = __OBS_CTX.lock().unwrap();
        let ctx = ctx.as_mut().unwrap();
        let r = f(ctx);

        // Receiver will always
        let _ = tx.send(r);
    })?;

    rx.await?
}

fn main() -> anyhow::Result<()> {
    let curr_dir = current_exe().context("Couldn't get current exe")?;
    let curr_dir = curr_dir.parent().context("Unwrapping parent from exe")?;
    set_current_dir(curr_dir)?;
    let _ = crash_handler::attach_crash_handler();

    let router = routes::router();

    // Initialize OBS
    let ctx = obs::initialize_obs("./recording.mp4")?;
    __OBS_CTX.lock().unwrap().replace(ctx);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(rspc_tauri2::plugin(router, |_| ()))
        .plugin(
            t_log::Builder::new()
                .target(t_log::Target::new(t_log::TargetKind::LogDir {
                    file_name: Some("logs".to_string()),
                }))
                .build(),
        )
        .setup(move |app| {
            block_on(__APP_HANDLE.write()).replace(app.handle().clone());
            block_on(obs::prepare_example()).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
