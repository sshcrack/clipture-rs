use anyhow::{Context, Result};
use chrono::Local;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use libobs_simple::{
    output::{
        replay::ObsContextReplayExt,
        simple::{HardwareCodec, HardwarePreset, ObsContextSimpleExt, OutputFormat},
    },
    sources::ObsSourceBuilder,
};
use libobs_wrapper::{
    capabilities::{EncoderKind, ListValue, PropertyKind, PropertyMetadata},
    context::ObsContext,
    data::{
        ObsDataSetters,
        output::{ObsOutputRef, ObsOutputTrait, ObsReplayBufferOutputRef},
        video::ObsVideoInfoBuilder,
    },
    encoders::ObsVideoEncoderType,
    scenes::{ObsSceneRef, SceneItemTrait},
    utils::{ObsPath, StartupInfo, VideoEncoderInfo},
};
use parking_lot::RwLock;
use std::{path::PathBuf, sync::Arc, thread};

use crate::{
    capture::{
        CapturePhase, CaptureSnapshot, CaptureSource, CaptureSourceKind, ObsCapabilitiesSnapshot,
        ObsEncoderCapability, ObsOption, ObsPropertySchema, ObsSourceCapability, PendingClip,
    },
    domain::settings::AppSettings,
};

#[derive(Clone)]
pub struct CaptureController {
    tx: Sender<Command>,
    state: Arc<RwLock<CaptureSnapshot>>,
}
enum Command {
    Select(CaptureSource, Sender<std::result::Result<(), String>>),
    ToggleRecording(Sender<std::result::Result<Option<PathBuf>, String>>),
    ToggleReplay(Sender<std::result::Result<(), String>>),
    SaveReplay(Sender<std::result::Result<PathBuf, String>>),
    Capabilities(Sender<std::result::Result<ObsCapabilitiesSnapshot, String>>),
    Reconfigure(
        AppSettings,
        PathBuf,
        Sender<std::result::Result<(), String>>,
    ),
}

impl CaptureController {
    pub fn spawn(settings: AppSettings, buffer_dir: PathBuf) -> Self {
        let state = Arc::new(RwLock::new(CaptureSnapshot::booting(&settings)));
        let (tx, rx) = unbounded();
        let state_worker = state.clone();
        thread::Builder::new()
            .name("clipture-obs".into())
            .spawn(
                move || match Engine::new(settings, buffer_dir, state_worker.clone()) {
                    Ok(mut engine) => engine.run(rx),
                    Err(error) => {
                        let mut s = state_worker.write();
                        s.phase = CapturePhase::Error;
                        s.message = Some(format!("{error:#}"));
                    }
                },
            )
            .expect("failed to start OBS thread");
        Self { tx, state }
    }
    pub fn snapshot(&self) -> CaptureSnapshot {
        self.state.read().clone()
    }
    pub fn add_pending(&self, p: PendingClip) {
        self.state.write().pending_clips.push(p)
    }
    pub fn remove_pending(&self, id: &str) {
        self.state.write().pending_clips.retain(|p| p.id != id)
    }
    fn request<T>(
        &self,
        make: impl FnOnce(Sender<std::result::Result<T, String>>) -> Command,
    ) -> std::result::Result<T, String> {
        let (tx, rx) = bounded(1);
        self.tx
            .send(make(tx))
            .map_err(|_| "capture engine stopped".to_string())?;
        rx.recv()
            .map_err(|_| "capture engine stopped".to_string())?
    }
    pub fn select(&self, s: CaptureSource) -> std::result::Result<(), String> {
        self.request(|r| Command::Select(s, r))
    }
    pub fn toggle_recording(&self) -> std::result::Result<Option<PathBuf>, String> {
        self.request(Command::ToggleRecording)
    }
    pub fn toggle_replay(&self) -> std::result::Result<(), String> {
        self.request(Command::ToggleReplay)
    }
    pub fn save_replay(&self) -> std::result::Result<PathBuf, String> {
        self.request(Command::SaveReplay)
    }
    pub fn capabilities(&self) -> std::result::Result<ObsCapabilitiesSnapshot, String> {
        self.request(Command::Capabilities)
    }
    pub fn reconfigure(
        &self,
        settings: AppSettings,
        buffer_dir: PathBuf,
    ) -> std::result::Result<(), String> {
        self.request(|reply| Command::Reconfigure(settings, buffer_dir, reply))
    }
}

struct Engine {
    context: ObsContext,
    scene: ObsSceneRef,
    replay: Option<ObsReplayBufferOutputRef>,
    recording: Option<ObsOutputRef>,
    recording_path: Option<PathBuf>,
    settings: AppSettings,
    buffer_dir: PathBuf,
    state: Arc<RwLock<CaptureSnapshot>>,
}
impl Engine {
    fn video_info(settings: &AppSettings) -> libobs_wrapper::data::video::ObsVideoInfo {
        ObsVideoInfoBuilder::new()
            .fps_num(settings.recording.fps)
            .fps_den(1)
            .base_width(settings.recording.width)
            .base_height(settings.recording.height)
            .output_width(settings.recording.width)
            .output_height(settings.recording.height)
            .build()
    }
    fn new(
        settings: AppSettings,
        buffer_dir: PathBuf,
        state: Arc<RwLock<CaptureSnapshot>>,
    ) -> Result<Self> {
        let mut context = StartupInfo::new()
            .set_video_info(Self::video_info(&settings))
            .start()
            .context("libobs startup failed")?;
        let scene = context
            .scene("Clipture", Some(0))
            .context("could not create Clipture scene")?;
        let encoder = selected_encoder_label(&context, &settings);
        {
            let mut s = state.write();
            s.phase = CapturePhase::Idle;
            s.message = None;
            s.encoder = encoder;
            s.fps = settings.recording.fps;
            s.output_resolution = format!(
                "{} × {}",
                settings.recording.width, settings.recording.height
            );
        }
        let mut engine = Self {
            context,
            scene,
            replay: None,
            recording: None,
            recording_path: None,
            settings,
            buffer_dir,
            state,
        };
        if engine.settings.replay.enabled && engine.settings.replay.auto_start {
            engine.start_replay()?;
        }
        Ok(engine)
    }
    fn run(&mut self, rx: Receiver<Command>) {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                Command::Select(source, reply) => {
                    let result = self.select_source(source);
                    if let Err(error) = &result {
                        let mut snapshot = self.state.write();
                        snapshot.source = None;
                        snapshot.phase = CapturePhase::Error;
                        snapshot.message = Some(format!("{error:#}"));
                    }
                    let _ = reply.send(result.map_err(|e| format!("{e:#}")));
                }
                Command::ToggleRecording(reply) => {
                    let _ = reply.send(self.toggle_recording().map_err(|e| format!("{e:#}")));
                }
                Command::ToggleReplay(reply) => {
                    let _ = reply.send(self.toggle_replay().map_err(|e| format!("{e:#}")));
                }
                Command::SaveReplay(reply) => {
                    let _ = reply.send(self.save_replay().map_err(|e| format!("{e:#}")));
                }
                Command::Capabilities(reply) => {
                    let _ = reply.send(self.capabilities_snapshot().map_err(|e| format!("{e:#}")));
                }
                Command::Reconfigure(settings, buffer_dir, reply) => {
                    let _ = reply.send(
                        self.reconfigure(settings, buffer_dir)
                            .map_err(|e| format!("{e:#}")),
                    );
                }
            }
        }
    }
    fn start_replay(&mut self) -> Result<()> {
        if self.replay.is_some() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.buffer_dir)?;
        let seconds = (self.settings.replay.pre_roll_seconds
            + self.settings.replay.post_roll_seconds)
            .max(1) as i64;
        let replay = self
            .context
            .replay_buffer_builder(
                "clipture-replay",
                ObsPath::new(&self.buffer_dir.to_string_lossy()),
            )
            .max_time_sec(seconds)
            .max_size_mb(self.settings.replay.max_size_mb as i64)
            .video_bitrate(self.settings.recording.bitrate_kbps)
            .audio_bitrate(192)
            .hardware_encoder(HardwareCodec::H264, HardwarePreset::Balanced)
            .extension("mp4")
            .format("%CCYY-%MM-%DD_%hh-%mm-%ss")
            .build()?;
        self.apply_encoder_override(&replay, "clipture-replay-video")?;
        replay.start()?;
        self.replay = Some(replay);
        self.update_phase();
        Ok(())
    }
    fn stop_replay(&mut self) {
        if let Some(r) = self.replay.take() {
            let _ = r.stop();
        }
        self.update_phase();
    }
    fn toggle_replay(&mut self) -> Result<()> {
        if self.replay.is_some() {
            self.stop_replay();
        } else {
            self.start_replay()?;
        }
        Ok(())
    }

    fn save_replay(&mut self) -> Result<PathBuf> {
        let replay = self
            .replay
            .as_ref()
            .context("replay buffer is not running")?;
        Ok(replay.save_buffer()?.into_path_buf())
    }
    fn toggle_recording(&mut self) -> Result<Option<PathBuf>> {
        if let Some(output) = self.recording.take() {
            output.stop()?;
            self.state.write().recording_active = false;
            self.state.write().recording_started_at = None;
            let path = self.recording_path.take();
            self.update_phase();
            return Ok(path);
        };
        let game = self
            .state
            .read()
            .source
            .as_ref()
            .map(|s| s.label.clone())
            .unwrap_or_else(|| "Manual capture".into());
        let root = PathBuf::from(&self.settings.storage.root);
        let date = Local::now().format("%Y-%m-%d").to_string();
        let safe_game: String = game
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, ' ' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let dir = root.join(safe_game).join(date);
        std::fs::create_dir_all(&dir)?;
        let unique = uuid::Uuid::new_v4().simple().to_string();
        let path = dir.join(format!(
            "{}-{}-recording.mkv",
            Local::now().format("%H-%M-%S-%3f"),
            &unique[..8]
        ));
        let output = self
            .context
            .simple_output_builder("clipture-recording", ObsPath::new(&path.to_string_lossy()))
            .format(OutputFormat::MatroskaVideo)
            .auto_video_encoder(HardwareCodec::H264)
            .video_bitrate(self.settings.recording.bitrate_kbps)
            .audio_bitrate(192)
            .build()?;
        self.apply_encoder_override(&output, "clipture-recording-video")?;
        output.start()?;
        self.recording = Some(output);
        self.recording_path = Some(path);
        {
            let mut s = self.state.write();
            s.recording_active = true;
            s.recording_started_at = Some(Local::now().timestamp_millis());
        }
        self.update_phase();
        Ok(None)
    }
    fn reconfigure(&mut self, settings: AppSettings, buffer_dir: PathBuf) -> Result<()> {
        let video_changed = self.settings.recording.fps != settings.recording.fps
            || self.settings.recording.width != settings.recording.width
            || self.settings.recording.height != settings.recording.height;
        let encoder_changed = self.settings.advanced.encoder != settings.advanced.encoder;
        if (video_changed || encoder_changed) && self.recording.is_some() {
            anyhow::bail!("Stop the full recording before changing resolution, FPS, or encoder")
        }

        let replay_was_running = self.replay.is_some();
        let replay_changed = self.settings.replay != settings.replay
            || self.settings.recording.bitrate_kbps != settings.recording.bitrate_kbps
            || self.buffer_dir != buffer_dir
            || video_changed
            || encoder_changed;
        let audio_changed = self.settings.audio != settings.audio;
        let current_source = self.state.read().source.clone();
        if (replay_changed || audio_changed) && !self.state.read().pending_clips.is_empty() {
            anyhow::bail!("Wait for queued clips to finish saving before changing capture settings")
        }

        if replay_changed {
            self.stop_replay();
        }
        if video_changed {
            self.context.reset_video(Self::video_info(&settings))?;
        }

        self.settings = settings;
        self.buffer_dir = buffer_dir;
        {
            let mut snapshot = self.state.write();
            snapshot.fps = self.settings.recording.fps;
            snapshot.encoder = selected_encoder_label(&self.context, &self.settings);
            snapshot.output_resolution = format!(
                "{} × {}",
                self.settings.recording.width, self.settings.recording.height
            );
        }

        if self.settings.replay.enabled
            && (replay_was_running || self.settings.replay.auto_start)
            && self.replay.is_none()
        {
            self.start_replay()?;
        }

        if audio_changed && let Some(source) = current_source {
            self.scene.clear()?;
            self.add_source(&source)?;
        }
        self.update_phase();
        Ok(())
    }

    fn update_phase(&self) {
        let mut s = self.state.write();
        s.replay_active = self.replay.is_some();
        s.recording_active = self.recording.is_some();
        s.phase = if s.recording_active {
            CapturePhase::Recording
        } else if s.replay_active {
            CapturePhase::Buffering
        } else {
            CapturePhase::Idle
        };
    }
    fn apply_encoder_override<T: ObsOutputTrait>(&self, output: &T, name: &str) -> Result<()> {
        let Some(id) = self.settings.advanced.encoder.as_deref() else {
            return Ok(());
        };
        let capabilities = self.context.capabilities()?;
        let Some(encoder) = capabilities.encoders().iter().find(|encoder| {
            encoder.kind() == EncoderKind::Video
                && !encoder.is_internal()
                && !encoder.is_deprecated()
                && encoder.id() == id
        }) else {
            log::warn!(
                "Configured video encoder '{id}' is unavailable; falling back to automatic selection"
            );
            return Ok(());
        };
        if encoder.codec() != Some("h264") {
            anyhow::bail!(
                "Selected encoder '{}' is not an H.264 encoder",
                encoder.id()
            );
        }
        let mut settings = self.context.data()?;
        settings.set_string("rate_control", "CBR")?;
        settings.set_int("bitrate", self.settings.recording.bitrate_kbps as i64)?;
        let info = VideoEncoderInfo::new(
            ObsVideoEncoderType::from(encoder.id()),
            name,
            Some(settings),
            None,
        );
        output.create_and_set_video_encoder(info)?;
        Ok(())
    }

    fn capabilities_snapshot(&self) -> Result<ObsCapabilitiesSnapshot> {
        let capabilities = self.context.capabilities()?;
        let video_encoders = capabilities
            .encoders()
            .iter()
            .filter(|encoder| encoder.kind() == EncoderKind::Video)
            .filter(|encoder| !encoder.is_internal() && !encoder.is_deprecated())
            .map(|encoder| ObsEncoderCapability {
                id: encoder.id().to_string(),
                label: encoder.display_name().unwrap_or(encoder.id()).to_string(),
                codec: encoder.codec().map(str::to_string),
                hardware: encoder.is_likely_hardware_accelerated(),
                properties: encoder
                    .properties()
                    .unwrap_or_default()
                    .iter()
                    .map(property_schema)
                    .collect(),
            })
            .collect();

        let interesting_source = |id: &str| {
            matches!(
                id,
                "game_capture"
                    | "window_capture"
                    | "monitor_capture"
                    | "wasapi_input_capture"
                    | "pulse_input_capture"
                    | "linux_general_window_capture"
                    | "linux_general_screen_capture"
            )
        };
        let capture_sources: Vec<_> = capabilities
            .source_types()
            .iter()
            .filter(|source| interesting_source(source.id()))
            .map(|source| ObsSourceCapability {
                id: source.id().to_string(),
                label: source.display_name().unwrap_or(source.id()).to_string(),
                properties: source
                    .properties()
                    .unwrap_or_default()
                    .iter()
                    .map(property_schema)
                    .collect(),
            })
            .collect();

        let microphones = capture_sources
            .iter()
            .find(|source| {
                source.id == "wasapi_input_capture" || source.id == "pulse_input_capture"
            })
            .and_then(|source| {
                source
                    .properties
                    .iter()
                    .find(|property| property.name == "device_id" || property.name == "device")
            })
            .map(|property| property.options.clone())
            .unwrap_or_default();

        Ok(ObsCapabilitiesSnapshot {
            video_encoders,
            capture_sources,
            microphones,
        })
    }

    fn stop_all(&mut self) {
        if let Some(r) = self.recording.take() {
            let _ = r.stop();
        }
        if let Some(r) = self.replay.take() {
            let _ = r.stop();
        }
    }
    fn select_source(&mut self, source: CaptureSource) -> Result<()> {
        if source.kind == CaptureSourceKind::Auto {
            if let Some(auto) = super::sources::resolve_auto(self.settings.advanced.capture_method)
            {
                return self.select_source(auto);
            };
            self.scene.clear()?;
            {
                let mut snapshot = self.state.write();
                snapshot.source = None;
                snapshot.message = None;
            }
            self.update_phase();
            return Ok(());
        }
        self.scene.clear()?;
        self.add_source(&source)?;
        {
            let mut snapshot = self.state.write();
            snapshot.source = Some(source);
            snapshot.message = None;
        }
        self.update_phase();
        Ok(())
    }

    #[cfg(windows)]
    fn add_source(&mut self, source: &CaptureSource) -> Result<()> {
        use libobs_simple::sources::windows::{
            GameCaptureSourceBuilder, MonitorCaptureSourceBuilder, WindowCaptureSourceBuilder,
            WindowSearchMode,
        };
        let per_app_audio = self.settings.audio.application_audio
            && self
                .context
                .capabilities()?
                .source_types()
                .iter()
                .any(|source| source.id() == "wasapi_process_output_capture");
        match source.kind {
            CaptureSourceKind::Game => {
                let windows =
                    GameCaptureSourceBuilder::get_windows(WindowSearchMode::IncludeMinimized)?;
                let w = windows
                    .into_iter()
                    .find(|window| {
                        source.native_id.as_deref() == Some(window.obs_id.as_str())
                            || (source.native_id.is_none() && Some(window.pid) == source.pid)
                    })
                    .context("game window disappeared")?;
                let capture = self
                    .context
                    .source_builder::<GameCaptureSourceBuilder, _>("Clipture game")?
                    .set_window(&w)
                    .set_capture_audio(per_app_audio)?
                    .build()?;
                let item = self.scene.add(capture)?;
                item.fit_source_to_screen()?;
            }
            CaptureSourceKind::Window => {
                let windows =
                    WindowCaptureSourceBuilder::get_windows(WindowSearchMode::IncludeMinimized)?;
                let w = windows
                    .into_iter()
                    .find(|window| {
                        source.native_id.as_deref() == Some(window.obs_id.as_str())
                            || (source.native_id.is_none() && Some(window.pid) == source.pid)
                    })
                    .context("window disappeared")?;
                let capture = self
                    .context
                    .source_builder::<WindowCaptureSourceBuilder, _>("Clipture window")?
                    .set_window(&w)
                    .set_capture_audio(per_app_audio)?
                    .build()?;
                let item = self.scene.add(capture)?;
                item.fit_source_to_screen()?;
            }
            CaptureSourceKind::Monitor => {
                let displays = MonitorCaptureSourceBuilder::get_monitors()?;
                let index = source
                    .id
                    .split(':')
                    .nth(1)
                    .and_then(|value| value.parse::<usize>().ok());
                let name = source.native_id.as_deref().unwrap_or_default();
                let display = index
                    .and_then(|index| displays.get(index))
                    .or_else(|| displays.iter().find(|display| display.name == name))
                    .or(displays.first())
                    .context("display disappeared")?;
                let item = self
                    .context
                    .source_builder::<MonitorCaptureSourceBuilder, _>("Clipture monitor")?
                    .set_monitor(display)
                    .add_to_scene(&mut self.scene)?;
                item.fit_source_to_screen()?;
            }
            CaptureSourceKind::Auto => unreachable!(),
        }
        if self.settings.audio.application_audio
            && (source.kind == CaptureSourceKind::Monitor || !per_app_audio)
        {
            self.add_system_audio();
        }
        self.add_microphone();
        Ok(())
    }
    #[cfg(target_os = "linux")]
    fn add_source(&mut self, source: &CaptureSource) -> Result<()> {
        use libobs_simple::sources::linux::{
            LinuxGeneralScreenCaptureBuilder, LinuxGeneralWindowCaptureBuilder,
        };
        match source.kind {
            CaptureSourceKind::Game | CaptureSourceKind::Window => {
                let native = source
                    .native_id
                    .as_deref()
                    .context("window has no capture id")?;
                let capture = self
                    .context
                    .source_builder::<LinuxGeneralWindowCaptureBuilder, _>("Clipture window")?
                    .set_capture_window(native)
                    .build()?;
                let item = self.scene.add(capture)?;
                item.fit_source_to_screen()?;
            }
            CaptureSourceKind::Monitor => {
                let index = source
                    .id
                    .split(':')
                    .nth(1)
                    .and_then(|v| v.parse::<i64>().ok())
                    .unwrap_or(0);
                let capture = self
                    .context
                    .source_builder::<LinuxGeneralScreenCaptureBuilder, _>("Clipture monitor")?
                    .set_screen(index)
                    .build()?;
                let item = self.scene.add(capture)?;
                item.fit_source_to_screen()?;
            }
            CaptureSourceKind::Auto => unreachable!(),
        }
        self.add_system_audio();
        self.add_microphone();
        Ok(())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    fn add_source(&mut self, _: &CaptureSource) -> Result<()> {
        anyhow::bail!("capture is not supported on this platform yet")
    }
    fn add_system_audio(&mut self) {
        if !self.settings.audio.application_audio {
            return;
        }
        let Ok(mut data) = self.context.data() else {
            return;
        };
        let _ = data.set_string("device_id", "default");
        #[cfg(windows)]
        let id = "wasapi_output_capture";
        #[cfg(target_os = "linux")]
        let id = "pulse_output_capture";
        #[cfg(not(any(windows, target_os = "linux")))]
        let id = "";
        if id.is_empty() {
            return;
        }
        let info =
            libobs_wrapper::utils::SourceInfo::new(id, "Clipture system audio", Some(data), None);
        if let Err(error) = self.scene.add_new_source(info) {
            log::warn!("Could not add system audio capture: {error}");
        }
    }

    fn add_microphone(&mut self) {
        if !self.settings.audio.microphone {
            return;
        }
        let Ok(mut data) = self.context.data() else {
            return;
        };
        let device = self
            .settings
            .audio
            .microphone_device_id
            .as_deref()
            .unwrap_or("default");
        let _ = data.set_string("device_id", device);
        #[cfg(windows)]
        let id = "wasapi_input_capture";
        #[cfg(target_os = "linux")]
        let id = "pulse_input_capture";
        #[cfg(not(any(windows, target_os = "linux")))]
        let id = "";
        if id.is_empty() {
            return;
        }
        let info =
            libobs_wrapper::utils::SourceInfo::new(id, "Clipture microphone", Some(data), None);
        if let Err(error) = self.scene.add_new_source(info) {
            log::warn!("Could not add microphone capture: {error}");
        }
    }
}

fn selected_encoder_label(context: &ObsContext, settings: &AppSettings) -> Option<String> {
    let capabilities = context.capabilities().ok()?;
    if let Some(id) = settings.advanced.encoder.as_deref()
        && let Some(encoder) = capabilities.encoders().iter().find(|encoder| {
            encoder.kind() == EncoderKind::Video
                && !encoder.is_internal()
                && !encoder.is_deprecated()
                && encoder.id() == id
        })
    {
        return Some(encoder.display_name().unwrap_or(encoder.id()).to_string());
    }
    capabilities
        .select_video_encoder()
        .codec("h264")
        .prefer_hardware()
        .best_available()
        .map(|encoder| encoder.display_name().unwrap_or(encoder.id()).to_string())
}

fn list_value(value: &ListValue) -> String {
    match value {
        ListValue::String(value) => value.clone(),
        ListValue::Int(value) => value.to_string(),
        ListValue::Float(value) => value.to_string(),
        ListValue::Bool(value) => value.to_string(),
        ListValue::UnknownFormat(value) => value.to_string(),
    }
}

fn property_schema(property: &PropertyMetadata) -> ObsPropertySchema {
    let (kind, options) = match &property.kind {
        PropertyKind::Bool => ("Boolean", vec![]),
        PropertyKind::Integer { .. } => ("Integer", vec![]),
        PropertyKind::Float { .. } => ("Number", vec![]),
        PropertyKind::Text { .. } => ("Text", vec![]),
        PropertyKind::Path { .. } => ("Path", vec![]),
        PropertyKind::List { items, .. } => (
            "List",
            items
                .iter()
                .map(|item| ObsOption {
                    label: item.name.clone(),
                    value: list_value(&item.value),
                    disabled: item.disabled,
                })
                .collect(),
        ),
        PropertyKind::Color => ("Color", vec![]),
        PropertyKind::Button { .. } => ("Button", vec![]),
        PropertyKind::Font => ("Font", vec![]),
        PropertyKind::EditableList { .. } => ("Editable list", vec![]),
        PropertyKind::FrameRate { .. } => ("Frame rate", vec![]),
        PropertyKind::Group { .. } => ("Group", vec![]),
        PropertyKind::ColorAlpha => ("Color + alpha", vec![]),
        PropertyKind::Unknown(_) => ("Unknown", vec![]),
        PropertyKind::Invalid => ("Invalid", vec![]),
    };
    ObsPropertySchema {
        name: property.name.clone(),
        label: property
            .description
            .clone()
            .unwrap_or_else(|| property.name.clone()),
        description: property.long_description.clone(),
        kind: kind.into(),
        enabled: property.enabled,
        visible: property.visible,
        options,
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop_all();
    }
}
