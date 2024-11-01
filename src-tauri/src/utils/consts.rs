use std::sync::Arc;

use lazy_static::lazy_static;
use semver::VersionReq;
use tauri::AppHandle;
use tokio::sync::RwLock;

lazy_static! {
    pub static ref __APP_HANDLE: Arc<RwLock<Option<AppHandle>>> = Arc::new(RwLock::new(None));
    pub static ref OBS_VERSION: VersionReq =
        VersionReq::parse("^30.2.0").expect("Invalid OBS version requirement");
}

pub async fn app_handle() -> AppHandle {
    __APP_HANDLE
        .read()
        .await
        .clone()
        .expect("AppHandle not set")
}

pub const RELEASES_URL: &'static str =
    "https://api.github.com/repos/sshcrack/obs-builds-clipture/releases";
