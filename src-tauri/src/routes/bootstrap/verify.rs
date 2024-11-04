use std::path::Path;

use anyhow::bail;
use libobs_wrapper::context::ObsContext;
use tauri::{process::current_binary, Manager};
use tokio::fs;

use crate::utils::consts::{APP_HANDLE, INVALID_OBS_SIZE, OBS_VERSION};

pub const OBS_ALTERNATIVE_DLL: &'static str = "obs_backup.dll";

pub async fn restore_dll(obs_dir: &Path) -> anyhow::Result<Box<Path>> {
    let backup_path = obs_dir.join(OBS_ALTERNATIVE_DLL);
    if !backup_path.exists() {
        log::debug!("Back up dll does not exist");
        bail!("Backup dll does not exist");
    }

    let out_dir = obs_dir.join("obs-backup");
    if !out_dir.exists() {
        fs::create_dir_all(&out_dir).await?;
    }

    let out_dll = out_dir.join("obs.dll");
    log::debug!("Restoring from {:?} to obs.dll", backup_path.display());

    fs::copy(&backup_path, out_dll).await?;

    Ok(out_dir.into_boxed_path())
}

pub async fn back_up_dll(path: &Path) -> anyhow::Result<()> {
    let backup_path = path.parent().unwrap().join(OBS_ALTERNATIVE_DLL);
    if backup_path.exists() {
        let backup_size = fs::metadata(&backup_path).await?.len();
        let original_size = fs::metadata(path).await?.len();

        //TODO Maybe hashing? But also this is a bit overkill
        if backup_size == original_size {
            log::debug!("obs.dll already backed up");
            return Ok(());
        }
    }

    log::debug!("Backing up obs.dll to {:?}", backup_path.display());
    fs::copy(path, &backup_path).await?;

    Ok(())
}

pub enum VerifyResult {
    Ok,
    // Can either be a version mismatch or a size mismatch (or doesn't exist at all but that should never happen)
    Invalid,
    Restored(Box<Path>),
}

pub async fn verify_installation() -> anyhow::Result<VerifyResult> {
    let handle = APP_HANDLE.read().await;
    let handle = handle.as_ref().expect("Should have app handle");

    let binary_path = current_binary(&handle.env())?;
    let obs_path = binary_path.parent().unwrap().join("obs.dll");
    if !obs_path.exists() {
        log::debug!("obs.dll at path {:?} does not exist", obs_path.display());
        return Ok(VerifyResult::Invalid);
    }

    let metadata = fs::metadata(&obs_path).await?;
    if metadata.len() < INVALID_OBS_SIZE as u64 {
        let res = restore_dll(&obs_path.parent().unwrap()).await;
        if let Ok(path) = res {
            // We restored the dll, so we need to restart the app
            return Ok(VerifyResult::Restored(path));
        }

        log::debug!(
            "obs.dll is invalid size: {:?} and restore failed with {:?}",
            metadata.len(),
            res.unwrap_err()
        );
        return Ok(VerifyResult::Invalid);
    }

    let local_version = ObsContext::get_version();
    log::debug!("Verify version OBS: {:?}", local_version);
    let local_version = local_version.parse::<semver::Version>();

    if local_version.is_err() {
        return Ok(VerifyResult::Invalid);
    }

    let matches = OBS_VERSION.matches(&local_version.unwrap());
    back_up_dll(&obs_path).await?;
    log::debug!("Version matches: {:?}", matches);

    if matches {
        Ok(VerifyResult::Ok)
    } else {
        Ok(VerifyResult::Invalid)
    }
}
