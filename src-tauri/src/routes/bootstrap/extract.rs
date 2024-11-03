use std::{
    env::current_exe,
    path::{Path, PathBuf},
};

use async_stream::stream;
use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use sevenz_rust::{default_entry_extract_fn, Password, SevenZReader};
pub(super) enum ExtractStatus {
    Error(anyhow::Error),
    Progress(f32, String),
    Done(PathBuf),
}

pub async fn extract_obs(file: &Path) -> anyhow::Result<impl Stream<Item = ExtractStatus>> {
    log::info!("Extracting OBS at {}", file.display());

    let handle = tauri::async_runtime::handle();
    let path = PathBuf::from(file);

    let destination = current_exe().expect("Should be able to get current exe");
    let destination = destination
        .parent()
        .expect("Should be able to get parent of exe");
    let destination = PathBuf::from(destination).join("new-obs");

    let dest = destination.clone();
    let stream = stream! {
        yield Ok((0.0, "Reading file...".to_string()));
        let mut sz = SevenZReader::open(&path, Password::empty())?;
        let (tx, mut rx) = tokio::sync::mpsc::channel(5);

        let total = sz.archive().files.len() as f32;
        if !dest.exists() {
            std::fs::create_dir_all(&dest)?;
        }

        let mut curr = 0;
        let mut r = handle.spawn_blocking(move || {
            sz.for_each_entries(|entry, reader| {
                curr += 1;
                tx.blocking_send((curr as f32 / total, format!("Extracting {}", entry.name()))).unwrap();

                let dest_path = dest.join(entry.name());

                default_entry_extract_fn(entry, reader, &dest_path)
            })?;

            Result::<_, anyhow::Error>::Ok((1.0, "Extraction done".to_string()))
        });

        loop {
            tokio::select! {
                m = rx.recv() => {
                    match m {
                        Some(e) => yield Ok(e),
                        None => break
                    }
                },
                res = &mut r => {
                    match res {
                        Ok(e) => yield e,
                        Err(e) => {
                            yield Err(e.into());
                            break;
                        }
                    }
                }
            };
        }

        yield Ok((1.0, "Extraction done".to_string()));
    };

    Ok(stream! {
        pin_mut!(stream);
        while let Some(status) = stream.next().await {
            match status {
                Ok(e) => yield ExtractStatus::Progress(e.0, e.1),
                Err(err) => {
                    log::error!("Error extracting OBS: {:?}", err);
                    yield ExtractStatus::Error(err);
                    return;
                }
            }
        }

        yield ExtractStatus::Done(destination);
    })
}
