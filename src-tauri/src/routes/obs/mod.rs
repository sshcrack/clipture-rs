use preview::DisplayId;
use rspc::{Router, RouterBuilder};
use serde::{Deserialize, Serialize};
use specta::Type;

mod preview;

#[derive(Serialize, Deserialize, Type)]
struct ObsPreviewCreation {
    window_label: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    background_color: Option<u32>,
}

#[derive(Serialize, Deserialize, Type)]
struct ObsPositionPayload {
    id: DisplayId,
    x: i32,
    y: i32,
}

#[derive(Serialize, Deserialize, Type)]
struct ObsSizePayload {
    id: DisplayId,
    width: u32,
    height: u32,
}

pub fn obs() -> RouterBuilder {
    <Router>::new().merge("preview.", preview::preview())
}
