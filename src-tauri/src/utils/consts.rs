use std::sync::Arc;

use lazy_static::lazy_static;
use semver::VersionReq;
use tauri::AppHandle;
use tokio::sync::RwLock;

lazy_static! {
    pub static ref APP_HANDLE: Arc<RwLock<Option<AppHandle>>> = Arc::new(RwLock::new(None));
    pub static ref OBS_VERSION: VersionReq =
        VersionReq::parse("^30.2.0").expect("Invalid OBS version requirement");
}

/// obs.dll has to be a minimum size of 100k bytes to be considered valid
/// (also please fix later)
pub const INVALID_OBS_SIZE: usize = 1024 * 100;
pub const CLIPTURE_BASE_URL: &'static str = "http://localhost:3000";

pub fn clipture_to_url<T: Into<String>>(url: T) -> String {
    format!("{}{}", CLIPTURE_BASE_URL, url.into())
}

#[allow(dead_code)]
pub async fn app_handle() -> AppHandle {
    APP_HANDLE.read().await.clone().expect("AppHandle not set")
}

pub const RELEASES_URL: &'static str =
    "https://api.github.com/repos/sshcrack/obs-builds-clipture/releases";
