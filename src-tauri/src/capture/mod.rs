pub mod engine;
pub mod sources;

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureSourceKind {
    Auto,
    Game,
    Window,
    Monitor,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    pub id: String,
    pub kind: CaptureSourceKind,
    pub label: String,
    pub subtitle: Option<String>,
    pub pid: Option<u32>,
    pub is_game: Option<bool>,
    pub native_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingClip {
    pub id: String,
    pub requested_at: i64,
    pub saves_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapturePhase {
    Booting,
    Idle,
    Buffering,
    Recording,
    Error,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshot {
    pub phase: CapturePhase,
    pub replay_active: bool,
    pub recording_active: bool,
    pub recording_started_at: Option<i64>,
    pub source: Option<CaptureSource>,
    pub pending_clips: Vec<PendingClip>,
    pub encoder: Option<String>,
    pub fps: u32,
    pub output_resolution: String,
    pub message: Option<String>,
}
impl CaptureSnapshot {
    pub fn booting(settings: &crate::domain::settings::AppSettings) -> Self {
        Self {
            phase: CapturePhase::Booting,
            replay_active: false,
            recording_active: false,
            recording_started_at: None,
            source: None,
            pending_clips: vec![],
            encoder: None,
            fps: settings.recording.fps,
            output_resolution: format!(
                "{} × {}",
                settings.recording.width, settings.recording.height
            ),
            message: Some("Preparing OBS runtime…".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsOption {
    pub label: String,
    pub value: String,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsPropertySchema {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub kind: String,
    pub enabled: bool,
    pub visible: bool,
    pub options: Vec<ObsOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsEncoderCapability {
    pub id: String,
    pub label: String,
    pub codec: Option<String>,
    pub hardware: bool,
    pub properties: Vec<ObsPropertySchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsSourceCapability {
    pub id: String,
    pub label: String,
    pub properties: Vec<ObsPropertySchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsCapabilitiesSnapshot {
    pub video_encoders: Vec<ObsEncoderCapability>,
    pub capture_sources: Vec<ObsSourceCapability>,
    pub microphones: Vec<ObsOption>,
}
