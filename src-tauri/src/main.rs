// Prevents additional console window on Windows in release, DO NOT REMOVE!!
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env::{current_exe, set_current_dir},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use lazy_static::lazy_static;
use libobs_sources::windows::MonitorCaptureSourceBuilder;
use libobs_wrapper::{
    context::ObsContext, data::ObsObjectBuilder, display::{ObsDisplayCreationData, WindowPositionTrait},
    sources::ObsSourceBuilder,
};
use obs::initialize_obs;
use tauri::{Manager, Position};
use tauri_plugin_log as t_log;

mod rpc;
mod crash_handler;
mod obs;
mod utils;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

lazy_static! {
    static ref OBS_CTX: Arc<Mutex<Option<ObsContext>>> = Arc::new(Mutex::new(None));
}

fn main() -> anyhow::Result<()> {
    let curr_dir = current_exe().context("Couldn't get current exe")?;
    let curr_dir = curr_dir.parent().context("Unwrapping parent from exe")?;
    set_current_dir(curr_dir)?;
    let _ = crash_handler::attach_crash_handler();


    let router = rpc::router();


    // Initialize OBS
    let ctx = initialize_obs("./recording.mp4")?;
    OBS_CTX.lock().unwrap().replace(ctx);

    let tmp = OBS_CTX.clone();
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
            let mut opt = tmp.lock().unwrap();
            let ctx = opt.as_mut().unwrap();
            let scene = ctx.scene("main_scene");
            let monitors = MonitorCaptureSourceBuilder::get_monitors().unwrap();
            let e = monitors.get(1).unwrap_or(monitors.get(0).unwrap());

            MonitorCaptureSourceBuilder::new("test_monitor")
                .set_monitor(e)
                .add_to_scene(scene)
                .unwrap();

            scene.add_and_set(0);

            let v = app.webview_windows();
            let main_window = v
                .values()
                .next()
                .ok_or(anyhow::anyhow!("Couldn't get main window"))?;
            let hwnd = main_window.hwnd().unwrap();

            let m = app.available_monitors();
            let m = m.unwrap();
            let m = m.get(1).unwrap();

            let p = m.position();
            main_window
                .set_position(Position::Physical(p.to_owned()))
                .unwrap();
            println!("Creating display {:?}", hwnd);

            let size = main_window.inner_size()?;

            let ratio = 16.0 / 9.0;
            let width = size.height as f32 / 2.0 * ratio;

            let c = ObsDisplayCreationData::new(hwnd, 0, 0, width as u32, size.height / 2);
            let d = ctx.display(c).unwrap();
            d.create();

            Ok(())
        })
        .on_window_event(|_w, event| match event {
            tauri::WindowEvent::Resized(size) => {
                let mut opt = OBS_CTX.lock().unwrap();
                let ctx = opt.as_mut().unwrap();
                let d = ctx.displays_mut().get_mut(0).unwrap();

                let ratio = 16.0 / 9.0;
                let width = size.height as f32 / 2.0 * ratio;

                d.set_size(width as u32, size.height / 2).unwrap();
            },
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
