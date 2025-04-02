mod capture;
pub mod runtime;

pub use capture::*;
use libobs_wrapper::{context::ObsContext, sources::ObsSourceRef};

pub struct ObsManager {
    #[allow(dead_code)]
    ctx: ObsContext,
    capture_source: ObsSourceRef,
}

use libobs_sources::windows::{MonitorCaptureSourceBuilder, WindowCaptureSourceBuilder};
use libobs_wrapper::{
    data::{ObsData, ObsObjectBuilder},
    encoders::ObsContextEncoders,
    enums::ObsLogLevel,
    logger::ObsLogger,
    utils::{AudioEncoderInfo, OutputInfo, StartupInfo, VideoEncoderInfo},
};

#[derive(Debug)]
pub struct LogLogger {}
impl ObsLogger for LogLogger {
    fn log(&mut self, level: ObsLogLevel, msg: String) {
        match level {
            ObsLogLevel::Error => log::error!(target: "obs", "{}", msg),
            ObsLogLevel::Warning => log::warn!(target: "obs", "{}", msg),
            ObsLogLevel::Info => log::info!(target: "obs", "{}", msg),
            ObsLogLevel::Debug => log::debug!(target: "obs", "{}", msg),
        }
    }
}

impl ObsManager {
    fn initialize_obs() -> anyhow::Result<ObsManager> {
        // Start the OBS context
        let startup_info = StartupInfo::default().set_logger(Box::new(LogLogger {}));

        let mut context = ObsContext::new(startup_info)?;

        let output_name = "output";
        let output_info = OutputInfo::new("ffmpeg_muxer", output_name, None, None);

        let mut output = context.output(output_info)?;

        // Register the video encoder
        let video_settings = ObsData::new();
        let video_info = VideoEncoderInfo::new(
            ObsContext::get_best_video_encoder(),
            "video_encoder",
            Some(video_settings),
            None,
        );

        let video_handler = ObsContext::get_video_ptr()?;
        output.video_encoder(video_info, video_handler)?;

        // Register the audio encoder
        let audio_settings = ObsData::new();

        let audio_info =
            AudioEncoderInfo::new("ffmpeg_aac", "audio_encoder", Some(audio_settings), None);

        let audio_handler = ObsContext::get_audio_ptr()?;
        output.audio_encoder(audio_info, 0, audio_handler)?;

        let mut scene = context.scene("Main Scene");
        let mut window_capture = MonitorCaptureSourceBuilder::new("window_capture")
            .set_monitor(&MonitorCaptureSourceBuilder::get_monitors().unwrap()[1]);

        let t = WindowCaptureSourceBuilder::get_windows(
            libobs_window_helper::WindowSearchMode::IncludeMinimized,
        )
        .unwrap();

        let window = t.iter().find(|w| w.obs_id.to_lowercase().contains("code.exe")).unwrap();
        println!("Found window: {:?}", window);
        //window_capture = window_capture.set_window(window);

        let window_capture = scene.add_source(window_capture.build())?;

        Ok(ObsManager {
            ctx: context,
            capture_source: window_capture,
        })
    }

    pub fn context(&mut self) -> &mut ObsContext {
        &mut self.ctx
    }
}
