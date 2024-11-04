use tauri::Manager;
use crate::utils::consts::APP_HANDLE;

pub async fn open_main_window() -> anyhow::Result<()> {
    let handle = APP_HANDLE.read().await;
    let handle = handle.as_ref().expect("Should have app handle");

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
