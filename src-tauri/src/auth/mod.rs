use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Ok};
use keyring::Entry;
use lazy_static::lazy_static;
use tauri::{App, Url};
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver},
        RwLock,
    },
    time::{self, Instant},
};

use crate::utils::{consts::clipture_to_url, util::AtomicDropGuard};

mod types;
pub struct AuthManager {
    entry: Entry,
    cookie_map: Option<HashMap<String, String>>,
    curr_login: Arc<AtomicBool>,
    rx: UnboundedReceiver<Url>,
}

impl AuthManager {
    pub fn new(app: &mut App) -> anyhow::Result<AuthManager> {
        let (tx, rx) = mpsc::unbounded_channel();
        app.deep_link().on_open_url(move |event| {
            for url in event.urls() {
                let _ = tx.send(url.clone());
            }
        });

        let entry = Entry::new("clipture-rs", "token")?;
        let cookie_map = entry.get_password().ok().and_then(|password| {
            let r: Option<HashMap<String, String>> = serde_json::from_str(&password).ok();

            r
        });

        let a = AuthManager {
            entry,
            cookie_map,
            curr_login: Arc::new(AtomicBool::new(false)),
            rx,
        };

        Ok(a)
    }

    #[allow(dead_code)]
    pub fn get_cookies(&self) -> Option<&HashMap<String, String>> {
        self.cookie_map.as_ref()
    }

    pub fn is_logged_in(&self) -> bool {
        self.entry.get_password().is_ok()
    }

    pub async fn login(&mut self) -> anyhow::Result<()> {
        let url = clipture_to_url("/redirects/login?appLogin=true");
        if self.rx.is_closed() {
            bail!("URL callback channel closed");
        }

        // Clear URL callback here
        while !self.rx.is_empty() {
            let _ = self.rx.recv().await;
        }

        open::that_in_background(url);
        let _guard = AtomicDropGuard::new(self.curr_login.clone());

        // Timeout at 10 minutes
        let timeout_at = Duration::from_secs(60 * 10);
        let timeout_at = Instant::now() + timeout_at;

        let secret = loop {
            let url = time::timeout_at(timeout_at, self.rx.recv())
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
                .to_string();
        };

        let client = reqwest::Client::new();
        let res = client
            .get(clipture_to_url("/api/validation/report"))
            .header("Authorization", secret)
            .send()
            .await?;

        let res = res.error_for_status()?;
        let res: types::Root = res.json().await?;

        let mut mapped = HashMap::<String, String>::new();
        for entry in res.entry {
            mapped.insert(entry.key, entry.cookie);
        }

        // Wait for URL callback here
        let as_str = serde_json::to_string(&mapped).context("Serializing JSON")?;
        self.entry.set_password(&as_str).context("Setting password")?;
        self.cookie_map = Some(mapped);

        Ok(())
    }
}

lazy_static! {
    pub static ref AUTH_MANAGER: Arc<RwLock<Option<AuthManager>>> = Arc::new(RwLock::new(None));
}
