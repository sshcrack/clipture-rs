use libobs_wrapper::{
    context::ObsContext, data::ObsData, encoders::ObsContextEncoders, utils::{AudioEncoderInfo, OutputInfo, StartupInfo, VideoEncoderInfo}
};

#[cfg(not(debug_assertions))]
use libobs_wrapper::logger::FileLogger;

#[cfg(not(debug_assertions))]
use crate::utils::dir::get_log_dir;

pub fn initialize_obs(rec_file: &str) -> anyhow::Result<ObsContext> {
    println!("Initializing OBS");
    #[cfg(not(debug_assertions))]
    let logger = FileLogger::from_dir(&get_log_dir()?)?;

    // Start the OBS context
    let startup_info = StartupInfo::default();

    #[cfg(not(debug_assertions))]
    let startup_info = startup_info.set_logger(Box::new(logger));
    let mut context = ObsContext::new(startup_info)?;

    // Set up output to ./recording.mp4
    let mut output_settings = ObsData::new();
    output_settings.set_string("path", rec_file);

    let output_name = "output";
    let output_info = OutputInfo::new("ffmpeg_muxer", output_name, Some(output_settings), None);

    let output = context.output(output_info)?;

    // Register the video encoder
    let mut video_settings = ObsData::new();
    /*video_settings
            .set_int("bf", 2)
            .set_bool("psycho_aq", true)
            .set_bool("lookahead", true)
            .set_string("profile", "high")
            .set_string("preset", "hq")
            .set_string("rate_control", "cbr")
            .set_int("bitrate", 10000);
    */
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
