use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, Context, Ok};
use keyring::Entry;
use lazy_static::lazy_static;
use tauri::{App, Url};
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver},
        Mutex, RwLock,
    },
    time::{self, Instant},
};

use crate::{json_typings::clipture_api::validation, utils::consts::clipture_to_url};
pub struct AuthManager {
    entry: Entry,
    cookie_map: Arc<RwLock<Option<HashMap<String, String>>>>,
    rx: Mutex<UnboundedReceiver<Url>>,
}

impl AuthManager {
    pub fn new(app: &mut App) -> anyhow::Result<AuthManager> {
        let (tx, rx) = mpsc::unbounded_channel();
        app.deep_link().on_open_url(move |event| {
            for url in event.urls() {
                let _ = tx.send(url.clone());
            }
        });

        let entry = Entry::new("clipture-rs", &whoami::username())?;
        let pass = entry.get_password();
        if let Err(e) = &pass {
            log::warn!("Failed to get password: {}", e);
        }

        let cookie_map = pass.ok().and_then(|password| {
            let r = serde_json::from_str::<HashMap<String, String>>(&password);
            if let Err(e) = r {
                log::warn!("Failed to deserialize password: {}", e);
                return None;
            }

            r.ok()
        });

        log::debug!("Auth Manager initializing: {}", cookie_map.is_some());
        let a = AuthManager {
            entry,
            cookie_map: Arc::new(RwLock::new(cookie_map)),
            rx: Mutex::new(rx),
        };

        Ok(a)
    }

    #[allow(dead_code)]
    pub async fn get_cookies(&self) -> Option<HashMap<String, String>> {
        self.cookie_map.read().await.clone()
    }

    pub async fn is_logged_in(&self) -> bool {
        self.cookie_map.read().await.is_some()
    }

    pub async fn sign_out(&self) -> anyhow::Result<()> {
        *self.cookie_map.write().await = None;
        self.entry
            .delete_credential()
            .context("Deleting password")?;
        Ok(())
    }

    pub fn open_sign_in_window(&self) -> () {
        let url: String = clipture_to_url("/redirects/login?appLogin=true");
        open::that_in_background(url);
    }

    pub async fn sign_in(&self) -> anyhow::Result<()> {
        let mut rx = self
            .rx
            .try_lock()
            .map_err(|_| anyhow!("Already logging in [ALREADY_LOG]"))?;

        self.open_sign_in_window();

        // Clear URL callback here
        while !rx.is_empty() {
            let _ = rx.recv().await;
        }

        // Timeout at 10 minutes
        let timeout_at = Duration::from_secs(60 * 10);
        let timeout_at = Instant::now() + timeout_at;

        let secret = loop {
            let url = time::timeout_at(timeout_at, rx.recv())
                .await
                .context("Waiting for URL callback timed out")?
                .ok_or_else(|| anyhow!("URL callback channel closed"))?;

            if url.scheme() != "clipture" {
                log::debug!("Ignoring URL: {}", url);
                continue;
            }

            if url.path().starts_with("/login") || url.path().starts_with("login") {
                log::debug!("Ignoring URL: {}", url);
                continue;
            }

            break url
                .query_pairs()
                .find(|entry| entry.0 == "secret")
                .ok_or_else(|| anyhow!("No secret in URL"))?
                .1
                .trim()
                .to_string();
        };

        let client = reqwest::Client::new();
        let res = client
            .get(clipture_to_url("/api/validation/report"))
            .header("Authorization", secret)
            .send()
            .await?;

        let res = res.error_for_status()?;
        let res: validation::report::Root = res.json().await?;

        let mut mapped = HashMap::<String, String>::new();
        for entry in &res.entry {
            mapped.insert(entry.key.clone(), entry.cookie.clone());
        }

        // Wait for URL callback here
        let as_str = serde_json::to_string(&mapped).context("Serializing JSON")?;
        self.entry
            .set_password(&as_str)
            .context("Setting password")?;

        if mapped.len() == 0 {
            log::warn!("{:?}", res);
            bail!("No cookies received");
        }
        log::debug!("Saved {} total of cookies", mapped.len());
        *self.cookie_map.write().await = Some(mapped);

        Ok(())
    }
}

lazy_static! {
    pub static ref AUTH_MANAGER: Arc<RwLock<Option<AuthManager>>> = Arc::new(RwLock::new(None));
}
