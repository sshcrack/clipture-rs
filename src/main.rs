use std::env::current_dir;
use std::{thread, time::Duration};

use libobs::wrapper::{
    ObsVideoInfoBuilder, AudioEncoderInfo, ObsColorspace, ObsContext, ObsData, ObsGraphicsModule, ObsPath, ObsScaleType, ObsVideoEncoderType, OutputInfo, SourceInfo, StartupInfo, VideoEncoderInfo
};
pub fn main() {
    wrapper_test();
}

fn wrapper_test() {
    // Start the OBS context
    let startup_info = StartupInfo::default().set_video_info(
        ObsVideoInfoBuilder::new()
            .graphics_module(ObsGraphicsModule::OpenGL)
            .colorspace(ObsColorspace::Default)
            .scale_type(ObsScaleType::Bilinear)
            .build(),
    );
    let mut context = ObsContext::new(startup_info).unwrap();



    let video_source_info = SourceInfo::new(
        "monitor_capture",
        "Screen Capture Source",
        None,
        None,
    );


    // Set up output to ./recording.mp4
    let mut output_settings = ObsData::new();

    let path = ObsPath::new(&(current_dir().unwrap().display().to_string() + "/recording.mp4")).build();

    println!("Outputting to {:?}", path);
    output_settings.set_string("path", path);

    let output_info = OutputInfo::new("ffmpeg_muxer", "simple_ffmpeg_output", Some(output_settings), None);

    let output = context.output(output_info).unwrap();

    // Register the video encoder
    let mut video_enc_settings = ObsData::new();
    video_enc_settings
        .set_bool("use_bufsize", true)
        .set_string("profile", "high")
        .set_string("preset", "veryfast")
        .set_string("rate_control", "CRF")
        .set_int("crf", 20);

    let video_enc = VideoEncoderInfo::new(
        ObsVideoEncoderType::OBS_X264,
        "simple_h264_recording",
        Some(video_enc_settings),
        None,
    );

    let video_handler = ObsContext::get_video_ptr().unwrap();
    output.video_encoder(video_enc, video_handler).unwrap();

    // Register the audio encoder
    let audio_info =
        AudioEncoderInfo::new("ffmpeg_aac", "simple_aac_recording", None, None);

    let audio_handler = ObsContext::get_audio_ptr().unwrap();
    output.audio_encoder(audio_info, 0, audio_handler).unwrap();


    // Register the source and record
    output.source(video_source_info, 0).unwrap();
    output.start().unwrap();

    println!("recording for 10 seconds...");
    thread::sleep(Duration::new(10, 0));

    // Open any fullscreen application and
    // Success!
    println!("Stopping...");
    let is_active = output.stop();
    if !is_active {
        println!("Success!");
    }

    drop(context);
    thread::sleep(Duration::new(5, 0));
}
