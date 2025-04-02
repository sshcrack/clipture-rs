use crate::{core::obs::runtime::run_with_obs, utils::consts::app_handle};
use libobs_wrapper::display::ObsDisplayCreationData;
use tauri::{Manager, WebviewWindowBuilder};
use tauri_plugin_window_state::{StateFlags, WindowExt};
use tokio::sync::oneshot;

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

    let (tx, rx) = oneshot::channel();
    let h = handle.clone();
    handle.run_on_main_thread(move || {
        let r = || {
            h.get_webview_window("bootstrap")
                .expect("Should have bootstrap window")
                .close()?;

            let cfg = h.config();
            let windows = &cfg.app.windows;
            let main_window = windows
                .iter()
                .find(|w| w.label == "main")
                .expect("Should have main window");

            let webview_window = WebviewWindowBuilder::from_config(&h, main_window)
                .unwrap()
                .build()?;

            webview_window.show()?;
            webview_window.set_focus()?;
            webview_window.restore_state(StateFlags::SIZE | StateFlags::POSITION)?;

            anyhow::Ok(())
        };

        let _ = tx.send(r());
    })?;

    rx.await??;

    println!("Done!");
    Ok(())
}
