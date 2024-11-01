use std::{env::current_exe, path::PathBuf};

use async_stream::stream;
use async_tempfile::TempFile;
use async_zip::tokio::read::seek::ZipFileReader;
use futures_core::Stream;
use futures_util::io::copy;
use tokio::{
    fs::{create_dir_all, OpenOptions},
    io::BufReader,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub(super) enum ExtractStatus {
    Error(anyhow::Error),
    Progress(f32, String),
}

/// Returns a relative path without reserved names, redundant separators, ".", or "..".
fn sanitize_file_path(path: &str) -> PathBuf {
    // Replaces backwards slashes
    path.replace('\\', "/")
        // Sanitizes each component
        .split('/')
        .map(sanitize_filename::sanitize)
        .collect()
}

pub async fn extract_obs(file: TempFile) -> anyhow::Result<impl Stream<Item = ExtractStatus>> {
    let file = BufReader::new(file);

    Ok(stream! {
        yield ExtractStatus::Progress(0.0, "Parsing archive file...".to_string());
        let reader = ZipFileReader::new(file.compat()).await;
        if let Err(e) = reader {
            log::error!("Error reading zip file: {:?}", e);
            yield ExtractStatus::Error(e.into());
            return;
        }

        let mut reader = reader.unwrap();
        let total = reader.file().entries().len();

        let out_dir = current_exe()
            .expect("Should be able to get current executable path");

        let out_dir = out_dir.parent()
            .expect("Should have a parent directory");

        for index in 0..total {
            let entries = reader.file().entries();
            let entry = entries.get(index).unwrap();
            let file_name = entry.filename().as_str();

            if let Err(e) = file_name {
                log::error!("Error extracting file: {:?}", e);
                yield ExtractStatus::Error(e.into());
                return;
            }

            let path = out_dir.join(sanitize_file_path(file_name.unwrap()));

            let is_dir = entry.dir();
            if let Err(e) = is_dir {
                log::error!("Error extracting file: {:?}", e);
                yield ExtractStatus::Error(e.into());
                return;
            }

            let is_dir = is_dir.unwrap();
            if is_dir {
                if !path.exists() {
                    let r = create_dir_all(&path).await;
                    if let Err(e) = r {
                        log::error!("Error creating directory: {:?}", e);
                        yield ExtractStatus::Error(e.into());
                        return;
                    }
                }

                continue;
            }

            let entry_reader = reader.reader_without_entry(index).await;
            if let Err(e) = entry_reader {
                log::error!("Error extracting file: {:?}", e);
                yield ExtractStatus::Error(e.into());
                return;
            }

            let mut entry_reader = entry_reader.unwrap();

            let parent = path.parent();
            if parent.is_none() {
                log::error!("Error extracting file: Should have a parent directory");
                yield ExtractStatus::Error(anyhow::anyhow!("Parent directory is None"));
                return;
            }

            let parent = parent.unwrap();
            if !parent.is_dir() {
                let r = create_dir_all(&parent).await;
                if let Err(e) = r {
                    log::error!("Error creating directory: {:?}", e);
                    yield ExtractStatus::Error(e.into());
                    return;
                }
            }

            let writer = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await;

            if let Err(e) = writer {
                log::error!("Error extracting file: {:?}", e);
                yield ExtractStatus::Error(e.into());
                return;
            }

            let writer = writer.unwrap();
            let r= copy(&mut entry_reader, &mut writer.compat_write()).await;
            if let Err(e) = r {
                log::error!("Error extracting file: {:?}", e);
                yield ExtractStatus::Error(e.into());
                return;
            }

            yield ExtractStatus::Progress(index as f32 / total as f32, "Extracting OBS".to_string());
        }

        yield ExtractStatus::Progress(1.0, "Extracted OBS".to_string());
    })
}
