use std::{fs::create_dir_all, path::PathBuf};

use directories::ProjectDirs;

pub fn get_project_dirs() -> anyhow::Result<ProjectDirs> {
    ProjectDirs::from("me", "sshcrack", "clipture")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))
}

#[allow(dead_code)]
pub fn get_log_dir() -> anyhow::Result<PathBuf> {
    let project_dirs = get_project_dirs()?;

    let logs = project_dirs.data_dir().parent().unwrap().join("logs");
    println!("logs {:?}", logs);
    create_dir_all(&logs)?;

    Ok(logs)
}
