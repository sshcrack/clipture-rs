use std::sync::Arc;

use lazy_static::lazy_static;
use tauri::AppHandle;
use tokio::sync::RwLock;

lazy_static! {
    pub static ref __APP_HANDLE: Arc<RwLock<Option<AppHandle>>> = Arc::new(RwLock::new(None));
}

pub async fn app_handle() -> AppHandle {
    __APP_HANDLE.read().await.clone().expect("AppHandle not set")
}
