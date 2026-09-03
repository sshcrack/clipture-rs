use anyhow::{Context, Result};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub replay: ReplaySettings,
    pub recording: RecordingSettings,
    pub audio: AudioSettings,
    pub hotkeys: HotkeySettings,
    pub notifications: NotificationSettings,
    pub storage: StorageSettings,
    pub behavior: BehaviorSettings,
    pub advanced: AdvancedSettings,
    pub onboarding_complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplaySettings {
    pub enabled: bool,
    pub auto_start: bool,
    pub pre_roll_seconds: u64,
    pub post_roll_seconds: u64,
    pub max_size_mb: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSettings {
    pub auto_record_games: bool,
    pub quality: QualityPreset,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub bitrate_kbps: u32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QualityPreset {
    Performance,
    Balanced,
    Quality,
    Custom,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettings {
    pub application_audio: bool,
    pub microphone: bool,
    pub microphone_device_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    pub save_clip: String,
    pub toggle_recording: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationSettings {
    pub desktop: bool,
    pub sound: bool,
    #[serde(rename = "inApp")]
    pub in_app: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StorageSettings {
    pub root: String,
    pub recording_budget_gb: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorSettings {
    pub start_with_system: bool,
    pub close_to_tray: bool,
    #[serde(default)]
    pub close_behavior_confirmed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedSettings {
    #[serde(default)]
    pub encoder: Option<String>,
    pub capture_method: CaptureMethod,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMethod {
    Auto,
    Game,
    Window,
    Monitor,
}

impl Default for AppSettings {
    fn default() -> Self {
        let root = UserDirs::new()
            .and_then(|u| u.video_dir().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Clipture");
        Self {
            replay: ReplaySettings {
                enabled: true,
                auto_start: true,
                pre_roll_seconds: 30,
                post_roll_seconds: 10,
                max_size_mb: 1500,
            },
            recording: RecordingSettings {
                auto_record_games: false,
                quality: QualityPreset::Balanced,
                fps: 60,
                width: 1920,
                height: 1080,
                bitrate_kbps: 12000,
            },
            audio: AudioSettings {
                application_audio: true,
                microphone: true,
                microphone_device_id: None,
            },
            hotkeys: HotkeySettings {
                save_clip: "F9".into(),
                toggle_recording: "Ctrl+Shift+R".into(),
            },
            notifications: NotificationSettings {
                desktop: true,
                sound: true,
                in_app: true,
            },
            storage: StorageSettings {
                root: root.to_string_lossy().to_string(),
                recording_budget_gb: Some(100),
            },
            behavior: BehaviorSettings {
                start_with_system: false,
                close_to_tray: true,
                close_behavior_confirmed: false,
            },
            advanced: AdvancedSettings {
                encoder: None,
                capture_method: CaptureMethod::Auto,
            },
            onboarding_complete: false,
        }
    }
}
impl AppSettings {
    pub fn validated(mut self) -> Self {
        self.replay.pre_roll_seconds = self.replay.pre_roll_seconds.clamp(5, 300);
        self.replay.post_roll_seconds = self.replay.post_roll_seconds.clamp(0, 120);
        self.replay.max_size_mb = self.replay.max_size_mb.clamp(128, 16_384);
        self.recording.fps = self.recording.fps.clamp(15, 240);
        self.recording.width = self.recording.width.clamp(640, 7680);
        self.recording.height = self.recording.height.clamp(360, 4320);
        self.recording.bitrate_kbps = self.recording.bitrate_kbps.clamp(1500, 120_000);
        self
    }
}

pub struct SettingsStore {
    path: PathBuf,
}
impl SettingsStore {
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("me", "sshcrack", "clipture")
            .context("could not locate Clipture config directory")?;
        fs::create_dir_all(dirs.config_dir())?;
        Ok(Self {
            path: dirs.config_dir().join("settings.json"),
        })
    }
    pub fn load(&self) -> AppSettings {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|raw| serde_json::from_str::<AppSettings>(&raw).ok())
            .unwrap_or_default()
            .validated()
    }
    pub fn save(&self, settings: &AppSettings) -> Result<()> {
        fs::write(&self.path, serde_json::to_vec_pretty(settings)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_close_confirmation_defaults_to_false() {
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        value["behavior"]
            .as_object_mut()
            .unwrap()
            .remove("closeBehaviorConfirmed");
        let settings: AppSettings = serde_json::from_value(value).unwrap();
        assert!(!settings.behavior.close_behavior_confirmed);
    }

    #[test]
    fn validation_bounds_capture_settings() {
        let mut settings = AppSettings::default();
        settings.replay.pre_roll_seconds = 0;
        settings.replay.post_roll_seconds = 999;
        settings.recording.fps = 1;
        settings.recording.width = 100;
        settings.recording.bitrate_kbps = 500_000;
        let settings = settings.validated();
        assert_eq!(settings.replay.pre_roll_seconds, 5);
        assert_eq!(settings.replay.post_roll_seconds, 120);
        assert_eq!(settings.recording.fps, 15);
        assert_eq!(settings.recording.width, 640);
        assert_eq!(settings.recording.bitrate_kbps, 120_000);
    }
}
