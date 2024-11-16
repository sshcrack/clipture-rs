// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    not(any(debug_assertions, feature = "show-console")),
    windows_subsystem = "windows"
)]

use std::{
    env::{current_exe, set_current_dir},
    process,
};

use anyhow::Context;
use tauri::Manager;
use core::auth::{AuthManager, AUTH_MANAGER};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_log as t_log;
use utils::{consts::APP_HANDLE, crash_handler};

mod core;
mod json_typings;
mod routes;
mod utils;

fn main() -> anyhow::Result<()> {
    let curr_dir = current_exe().context("Couldn't get current exe")?;
    let curr_dir = curr_dir.parent().context("Unwrapping parent from exe")?;
    set_current_dir(curr_dir)?;

    let _ = crash_handler::attach_crash_handler();
    let router = routes::router();

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("Single instance check");
            let window = app
                .get_webview_window("bootstrap")
                .or_else(|| app.get_webview_window("main"));

            if let Some(window) = window {
                log::debug!("Focusing window {}", window.label());
                let e = window.set_focus();
                if let Err(e) = e {
                    log::warn!("Error setting focus to main window: {:?}", e);
                }
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin({
            let b = tauri_plugin_window_state::Builder::new().skip_initial_state("main");

            #[cfg(debug_assertions)]
            let b = b.with_denylist(&["bootstrap"]);

            b.build()
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(rspc_tauri2::plugin(router, |_| ()))
        .plugin(
            t_log::Builder::new()
                .target(t_log::Target::new(t_log::TargetKind::LogDir {
                    file_name: Some("logs".to_string()),
                }))
                .build(),
        )
        .setup(move |app| {
            APP_HANDLE.blocking_write().replace(app.handle().clone());

            let auth_manager = AuthManager::new(app);
            if let Err(err) = auth_manager {
                app.dialog()
                    .message(format!("Error initializing auth manager: {}", err))
                    .kind(MessageDialogKind::Error)
                    .blocking_show();

                process::exit(1);
            } else {
                let mut guard = AUTH_MANAGER.blocking_write();
                *guard = Some(auth_manager.unwrap());
            }

            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
