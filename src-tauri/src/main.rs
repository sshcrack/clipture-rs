// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env::{current_exe, set_current_dir}, sync::{Arc, Mutex}};

use anyhow::Context;
use libobs_sources::windows::WindowCaptureSourceBuilder;
use libobs_window_helper::{WindowInfo, WindowSearchMode};
use libobs_wrapper::{context::ObsContext, display::ObsDisplayCreationData, sources::ObsSourceBuilder};
use obs::initialize_obs;
use tauri::Manager;
use lazy_static::lazy_static;

mod crash_handler;
mod obs;
mod utils;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    main_wrapper().unwrap();
}

lazy_static! {
    static ref OBS_CTX: Arc<Mutex<Option<ObsContext>>> = Arc::new(Mutex::new(None));
}


fn find_notepad() -> Option<WindowInfo> {
    let windows =
        WindowCaptureSourceBuilder::get_windows(WindowSearchMode::ExcludeMinimized).unwrap();
    windows.into_iter().find(|w| {
        w.class
            .as_ref()
            .is_some_and(|e| e.to_lowercase().contains("notepad"))
    })
}

fn main_wrapper() -> anyhow::Result<()> {
    let curr_dir = current_exe().context("Couldn't get current exe")?;
    let curr_dir = curr_dir.parent().context("Unwrapping parent from exe")?;
    set_current_dir(curr_dir)?;
    let _ = crash_handler::attach_crash_handler();

    // Initialize OBS
    let ctx = initialize_obs("./recording.mp4")?;
    OBS_CTX.lock().unwrap().replace(ctx);

    let tmp = OBS_CTX.clone();
    tauri::Builder::default()
    .setup(move |app| {
        let mut opt = tmp.lock().unwrap();
        let ctx = opt.as_mut().unwrap();
        let scene = ctx.scene("main_scene");
        let w = find_notepad().unwrap();
        WindowCaptureSourceBuilder::new("notepad")
            .set_window(&w)
            .add_to_scene(scene)
            .unwrap();

        scene.add_and_set(0);

        let v = app.windows();
        let main_window = v.values().next().ok_or(anyhow::anyhow!("Couldn't get main window"))?;
        let hwnd = main_window.hwnd().unwrap();

        println!("Creating display {:?}", hwnd);
        let c = ObsDisplayCreationData::new(hwnd, 0, 0, 800, 600);
        ctx.display(c).unwrap();

        Ok(())

    })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
