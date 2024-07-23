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
    let ovi = ObsVideoInfoBuilder::new()
    .graphics_module(ObsGraphicsModule::DirectX11)
    .colorspace(ObsColorspace::Default)
    .scale_type(ObsScaleType::Bilinear)
    .build();

    let startup_info = StartupInfo::default().set_video_info(
        ovi,
    );
    let mut context = ObsContext::new(startup_info).unwrap();


    let mut vid_src_settings = ObsData::new();
    //vid_src_settings.set_string("monitor_id", "\\\\?\\DISPLAY#Default_Monitor#1&1f0c3c2f&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}");
    vid_src_settings.set_string("monitor_id", "\\\\?\\DISPLAY#AOC2402#7&11e44168&3&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}");

    let video_source_info = SourceInfo::new(
        "monitor_capture",
        "Screen Capture Source",
        Some(vid_src_settings),
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
    .set_string("rate_control", "CBR")
    .set_int("bitrate", 2500)
    .set_int("buffer_size", 2500)
    .set_int("crf", 23)
    .set_int("fps_num", 30)
    .set_int("fps_den", 1)
    //.set_int("width", 1366)
    .set_int("width", 1920)
    //.set_int("height", 768)
    .set_int("height", 1080)
    .set_int("keyint", 250);

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

    let mut audio_settings = ObsData::new();
    audio_settings.set_string("device_id", "default");
    let audio_info = SourceInfo::new("wasapi_output_capture", "Audio Capture Source", Some(audio_settings), None);

    // Register the source and record
    output.source(video_source_info, 0).unwrap();
    output.source(audio_info, 1).unwrap();

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
