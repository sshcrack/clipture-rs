use anyhow::{Context, Result, bail};
use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::UNIX_EPOCH,
};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Clip,
    Recording,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub id: String,
    pub kind: MediaKind,
    pub title: String,
    pub game: String,
    pub created_at: i64,
    pub duration_ms: Option<u64>,
    pub size_bytes: u64,
    pub path: String,
    pub protected: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimRequest {
    pub id: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub title: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sidecar {
    id: String,
    kind: MediaKind,
    title: String,
    game: String,
    created_at: i64,
    duration_ms: Option<u64>,
    protected: bool,
}

#[derive(Clone)]
pub struct LibraryStore {
    root: PathBuf,
}
impl LibraryStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }
    pub fn buffer_dir(&self) -> Result<PathBuf> {
        let p = self.root.join(".buffer");
        fs::create_dir_all(&p)?;
        Ok(p)
    }
    fn sidecar(path: &Path) -> PathBuf {
        PathBuf::from(format!("{}.clipture.json", path.to_string_lossy()))
    }
    fn safe_game(game: &str) -> String {
        let s: String = game
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, ' ' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if s.trim().is_empty() {
            "Unknown game".into()
        } else {
            s.trim().to_string()
        }
    }
    pub fn destination(&self, kind: MediaKind, game: &str, extension: &str) -> Result<PathBuf> {
        let now = Local::now();
        let dir = self
            .root
            .join(Self::safe_game(game))
            .join(now.format("%Y-%m-%d").to_string());
        fs::create_dir_all(&dir)?;
        let suffix = if kind == MediaKind::Clip {
            "clip"
        } else {
            "recording"
        };
        let unique = Uuid::new_v4().simple().to_string();
        Ok(dir.join(format!(
            "{}-{}-{suffix}.{extension}",
            now.format("%H-%M-%S-%3f"),
            &unique[..8]
        )))
    }
    pub fn register(
        &self,
        source: &Path,
        kind: MediaKind,
        game: &str,
        duration_ms: Option<u64>,
    ) -> Result<LibraryItem> {
        let extension =
            source
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or(if kind == MediaKind::Clip {
                    "mp4"
                } else {
                    "mkv"
                });
        let target = self.destination(kind, game, extension)?;
        let final_path = if source != target {
            fs::rename(source, &target).or_else(|_| {
                fs::copy(source, &target)?;
                fs::remove_file(source)?;
                Ok::<_, std::io::Error>(())
            })?;
            target
        } else {
            target
        };
        let created_at = Local::now().timestamp_millis();
        let title = format!("{} · {}", game, Local::now().format("%H:%M"));
        let side = Sidecar {
            id: Uuid::new_v4().to_string(),
            kind,
            title,
            game: game.into(),
            created_at,
            duration_ms,
            protected: false,
        };
        fs::write(
            Self::sidecar(&final_path),
            serde_json::to_vec_pretty(&side)?,
        )?;
        self.item_from(&final_path, Some(side))
    }
    fn item_from(&self, path: &Path, side: Option<Sidecar>) -> Result<LibraryItem> {
        let meta = fs::metadata(path)?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let side = side.or_else(|| {
            fs::read_to_string(Self::sidecar(path))
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        });
        let fallback_game = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".into());
        let kind = if path.extension().and_then(|e| e.to_str()) == Some("mkv") {
            MediaKind::Recording
        } else {
            MediaKind::Clip
        };
        Ok(match side {
            Some(s) => LibraryItem {
                id: s.id,
                kind: s.kind,
                title: s.title,
                game: s.game,
                created_at: s.created_at,
                duration_ms: s.duration_ms,
                size_bytes: meta.len(),
                path: path.to_string_lossy().to_string(),
                protected: s.protected,
            },
            None => LibraryItem {
                id: path.to_string_lossy().to_string(),
                kind,
                title: format!(
                    "{} · {}",
                    fallback_game,
                    Local
                        .timestamp_millis_opt(modified)
                        .single()
                        .unwrap_or_else(Local::now)
                        .format("%H:%M")
                ),
                game: fallback_game,
                created_at: modified,
                duration_ms: None,
                size_bytes: meta.len(),
                path: path.to_string_lossy().to_string(),
                protected: false,
            },
        })
    }
    pub fn list(&self) -> Result<Vec<LibraryItem>> {
        let mut out = vec![];
        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let p = entry.path();
            if p.components().any(|c| c.as_os_str() == ".buffer") {
                continue;
            }
            if matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("mp4" | "mkv" | "mov")
            ) && let Ok(i) = self.item_from(p, None)
            {
                out.push(i)
            }
        }
        out.sort_by_key(|i| std::cmp::Reverse(i.created_at));
        Ok(out)
    }
    fn find(&self, id: &str) -> Result<LibraryItem> {
        self.list()?
            .into_iter()
            .find(|i| i.id == id)
            .context("capture no longer exists")
    }
    pub fn delete(&self, id: &str) -> Result<()> {
        let i = self.find(id)?;
        let p = PathBuf::from(&i.path);
        fs::remove_file(&p)?;
        let _ = fs::remove_file(Self::sidecar(&p));
        Ok(())
    }
    pub fn rename(&self, id: &str, title: &str) -> Result<()> {
        let i = self.find(id)?;
        let p = PathBuf::from(&i.path);
        let mut side = self.read_side(&p, &i);
        side.title = title.trim().chars().take(120).collect();
        fs::write(Self::sidecar(&p), serde_json::to_vec_pretty(&side)?)?;
        Ok(())
    }
    pub fn protect(&self, id: &str, value: bool) -> Result<()> {
        let i = self.find(id)?;
        let p = PathBuf::from(&i.path);
        let mut side = self.read_side(&p, &i);
        side.protected = value;
        fs::write(Self::sidecar(&p), serde_json::to_vec_pretty(&side)?)?;
        Ok(())
    }
    fn read_side(&self, p: &Path, i: &LibraryItem) -> Sidecar {
        fs::read_to_string(Self::sidecar(p))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Sidecar {
                id: i.id.clone(),
                kind: i.kind,
                title: i.title.clone(),
                game: i.game.clone(),
                created_at: i.created_at,
                duration_ms: i.duration_ms,
                protected: i.protected,
            })
    }
    pub fn path_for(&self, id: &str) -> Result<PathBuf> {
        Ok(PathBuf::from(self.find(id)?.path))
    }
    pub async fn trim(&self, request: TrimRequest) -> Result<LibraryItem> {
        if request.end_seconds <= request.start_seconds {
            bail!("trim end must be after start")
        };
        let item = self.find(&request.id)?;
        let input = PathBuf::from(&item.path);
        let output = self.destination(MediaKind::Clip, &item.game, "mp4")?;
        let ffmpeg = ffmpeg_sidecar::paths::ffmpeg_path();
        if !ffmpeg.exists() {
            ffmpeg_sidecar::download::auto_download()
                .context("could not install the clip editor runtime")?;
        }
        let status = Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-ss",
                &format!("{:.3}", request.start_seconds),
                "-to",
                &format!("{:.3}", request.end_seconds),
                "-i",
            ])
            .arg(&input)
            .args(["-map", "0", "-c", "copy"])
            .arg(&output)
            .status()?;
        if !status.success() {
            bail!("ffmpeg could not trim this capture")
        };
        let mut result = self.register(
            &output,
            MediaKind::Clip,
            &item.game,
            Some(((request.end_seconds - request.start_seconds) * 1000.0) as u64),
        )?;
        if let Some(title) = request.title {
            self.rename(&result.id, &title)?;
            result.title = title;
        }
        Ok(result)
    }
    pub fn enforce_recording_budget(&self, budget_gb: u64) -> Result<()> {
        let mut recs: Vec<_> = self
            .list()?
            .into_iter()
            .filter(|i| i.kind == MediaKind::Recording && !i.protected)
            .collect();
        let mut total: u64 = self
            .list()?
            .into_iter()
            .filter(|i| i.kind == MediaKind::Recording)
            .map(|i| i.size_bytes)
            .sum();
        let budget = budget_gb * 1_000_000_000;
        recs.sort_by_key(|i| i.created_at);
        for i in recs {
            if total <= budget {
                break;
            }
            let size = i.size_bytes;
            self.delete(&i.id)?;
            total = total.saturating_sub(size)
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_file(root: &Path, name: &str, bytes: usize) -> PathBuf {
        let path = root.join(name);
        fs::write(&path, vec![0_u8; bytes]).unwrap();
        path
    }

    #[test]
    fn recording_budget_never_deletes_clips_or_protected_recordings() {
        let temp = tempfile::tempdir().unwrap();
        let incoming = temp.path().join("incoming");
        fs::create_dir_all(&incoming).unwrap();
        let store = LibraryStore::new(temp.path().join("library")).unwrap();

        let clip = store
            .register(
                &fake_file(&incoming, "clip.mp4", 64),
                MediaKind::Clip,
                "Test Game",
                Some(40_000),
            )
            .unwrap();
        let protected = store
            .register(
                &fake_file(&incoming, "protected.mkv", 64),
                MediaKind::Recording,
                "Test Game",
                Some(60_000),
            )
            .unwrap();
        store.protect(&protected.id, true).unwrap();
        let disposable = store
            .register(
                &fake_file(&incoming, "disposable.mkv", 64),
                MediaKind::Recording,
                "Test Game",
                Some(60_000),
            )
            .unwrap();

        store.enforce_recording_budget(0).unwrap();
        let items = store.list().unwrap();
        assert!(items.iter().any(|item| item.id == clip.id));
        assert!(items.iter().any(|item| item.id == protected.id));
        assert!(!items.iter().any(|item| item.id == disposable.id));
    }
}
