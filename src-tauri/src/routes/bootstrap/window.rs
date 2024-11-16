use crate::utils::consts::app_handle;
use tauri::{Manager, WebviewWindowBuilder};
use tauri_plugin_window_state::{StateFlags, WindowExt};

pub async fn open_main_window() -> anyhow::Result<()> {
    let handle = app_handle().await;

    log::trace!("Closing bootstrap window");
    handle
        .get_webview_window("bootstrap")
        .expect("Should have bootstrap window")
        .close()?;

    let main = handle
        .get_webview_window("main")
        .expect("Should have main window");

    log::trace!("Showing main window");
    main.show()?;

    Ok(())
}

pub async fn create_main_window() -> anyhow::Result<()> {
    let handle = app_handle().await;

    log::trace!("Closing bootstrap window");
    handle
        .get_webview_window("bootstrap")
        .expect("Should have bootstrap window")
        .close()?;

    let cfg = handle.config();
    let windows = &cfg.app.windows;
    let main_window = windows
        .iter()
        .find(|w| w.label == "main")
        .expect("Should have main window");

    log::trace!("Building main from config");
    let webview_window = WebviewWindowBuilder::from_config(&handle, main_window)
        .unwrap()
        .build()?;

    log::trace!("Showing, setting focus and restoring state");
    webview_window.show()?;
    webview_window.set_focus()?;
    webview_window.restore_state(StateFlags::SIZE | StateFlags::POSITION)?;

    Ok(())
}
