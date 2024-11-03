use libobs_wrapper::{
    context::ObsContext,
    data::ObsData,
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
            ObsLogLevel::Error => log::error!("{}", msg),
            ObsLogLevel::Warning => log::warn!("{}", msg),
            ObsLogLevel::Info => log::info!("{}", msg),
            ObsLogLevel::Debug => log::debug!("{}", msg),
        }
    }
}

pub fn initialize_obs(rec_file: &str) -> anyhow::Result<ObsContext> {
    println!("Initializing OBS");

    // Start the OBS context
    let startup_info = StartupInfo::default().set_logger(Box::new(LogLogger {}));

    let mut context = ObsContext::new(startup_info)?;

    // Set up output to ./recording.mp4
    let mut output_settings = ObsData::new();
    output_settings.set_string("path", rec_file);

    let output_name = "output";
    let output_info = OutputInfo::new("ffmpeg_muxer", output_name, Some(output_settings), None);

    let output = context.output(output_info)?;

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

    Ok(context)
}
