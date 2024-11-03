use std::{env::temp_dir, path::PathBuf};

use anyhow::Context;
use async_stream::stream;
use futures_core::Stream;
use futures_util::StreamExt;
use semver::Version;
use sha2::{Digest, Sha256};
use tokio::{fs::File, io::AsyncWriteExt};
use uuid::Uuid;

use crate::{
    json_to_rs::github,
    utils::consts::{OBS_VERSION, RELEASES_URL},
};

pub(super) enum DownloadStatus {
    Error(anyhow::Error),
    Progress(f32, String),
    Done(PathBuf),
}

pub(super) async fn download_obs() -> anyhow::Result<impl Stream<Item = DownloadStatus>> {
    // Fetch latest OBS release
    let client = reqwest::ClientBuilder::new()
        .user_agent("clipture-rs")
        .build()?;

    let releases: github::releases::Root = client.get(RELEASES_URL).send().await?.json().await?;

    let mut possible_versions = vec![];
    for release in releases {
        let tag = release.tag_name.replace("obs-build-", "");
        let version = Version::parse(&tag).context("Parsing version")?;

        if OBS_VERSION.matches(&version) {
            possible_versions.push(release);
        }
    }

    let latest_version = possible_versions
        .iter()
        .max_by_key(|r| &r.published_at)
        .context("Finding latest version")?;

    let archive_url = latest_version
        .assets
        .iter()
        .find(|a| a.name.ends_with(".7z"))
        .context("Finding 7z asset")?
        .browser_download_url
        .clone();

    let hash_url = latest_version
        .assets
        .iter()
        .find(|a| a.name.ends_with(".sha256"))
        .context("Finding sha256 asset")?
        .browser_download_url
        .clone();

    let res = client.get(archive_url).send().await?;
    let length = res.content_length().unwrap_or(0);

    let mut bytes_stream = res.bytes_stream();

    let path = PathBuf::new()
        .join(temp_dir())
        .join(format!("{}.7z", Uuid::new_v4()));
    let mut tmp_file = File::create_new(&path)
        .await
        .context("Creating temporary file")?;
    let mut curr_len = 0;

    let mut hasher = Sha256::new();
    Ok(stream! {
        yield DownloadStatus::Progress(0.0, "Downloading OBS".to_string());
        while let Some(chunk) = bytes_stream.next().await {
            let chunk = chunk.context("Retrieving data from stream");
            if let Err(e) = chunk {
                yield DownloadStatus::Error(e);
                return;
            }

            let chunk = chunk.unwrap();
            hasher.update(&chunk);
            let r = tmp_file.write_all(&chunk).await.context("Writing to temporary file");
            if let Err(e) = r {
                yield DownloadStatus::Error(e);
                return;
            }

            curr_len = std::cmp::min(curr_len + chunk.len() as u64, length);
            yield DownloadStatus::Progress(curr_len as  f32 / length as f32, "Downloading OBS".to_string());
        }

        // Getting remote hash
        let remote_hash = client.get(hash_url).send().await.context("Fetching hash");
        if let Err(e) = remote_hash {
            yield DownloadStatus::Error(e);
            return;
        }

        let remote_hash = remote_hash.unwrap().text().await.context("Reading hash");
        if let Err(e) = remote_hash {
            yield DownloadStatus::Error(e);
            return;
        }

        let remote_hash = remote_hash.unwrap();
        let remote_hash = hex::decode(remote_hash.trim()).context("Decoding hash");
        if let Err(e) = remote_hash {
            yield DownloadStatus::Error(e);
            return;
        }

        let remote_hash = remote_hash.unwrap();

        // Calculating local hash
        let local_hash = hasher.finalize();
        if local_hash.as_slice() != remote_hash {
            yield DownloadStatus::Error(anyhow::anyhow!("Hash mismatch"));
            return;
        }

        log::info!("Hashes match");
        yield DownloadStatus::Done(path);
    })
}
