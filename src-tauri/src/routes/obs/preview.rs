use std::pin::Pin;

use libobs_wrapper::display::{ObsDisplayCreationData, ObsDisplayRef, WindowPositionTrait};
use rspc::{Error as RError, ErrorCode, Router, RouterBuilder};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;

use crate::{
    core::obs::{runtime::run_with_obs_rspc, ObsManager},
    utils::{consts::app_handle, rspc::to_internal_res},
};

pub type DisplayId = u32;
fn get_display(
    mgr: &mut ObsManager,
    id: DisplayId,
) -> Result<&mut Pin<Box<ObsDisplayRef>>, rspc::Error> {
    let id: usize = id.try_into().map_err(|_| {
        RError::new(
            ErrorCode::BadRequest,
            "Couldn't cast u32 display id to usize".to_string(),
        )
    })?;

    mgr.context()
        .displays_mut()
        .iter_mut()
        .find(|d| d.id() == id)
        .ok_or_else(|| RError::new(ErrorCode::NotFound, "Display not found".to_string()))
}

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

pub fn preview() -> RouterBuilder {
    <Router>::new() //
        .mutation("create", |t| {
            t(|_ctx, data: ObsPreviewCreation| async move {
                let handle = app_handle().await;
                let window = handle.webview_windows();
                let window = window.get(&data.window_label);

                if window.is_none() {
                    return Err(RError::new(
                        ErrorCode::NotFound,
                        "Window not found".to_string(),
                    ));
                }

                let window = window.unwrap().clone();
                run_with_obs_rspc(move |mgr| {
                    let hwnd = window.hwnd().map_err(|e| {
                        RError::new(
                            ErrorCode::InternalServerError,
                            format!("Error getting window handle: {:?}", e),
                        )
                    })?;

                    let mut creation =
                        ObsDisplayCreationData::new(hwnd, data.x, data.y, data.width, data.height);
                    if let Some(c) = data.background_color {
                        creation = creation.set_background_color(c);
                    }

                    let ctx = mgr.context();
                    let display = ctx.display(creation).map_err(|e| {
                        RError::new(
                            ErrorCode::InternalServerError,
                            format!("Error creating display: {:?}", e),
                        )
                    })?;

                    log::debug!("Display created with id: {}", display.id());
                    let converted = u32::try_from(display.id());
                    match converted {
                        Ok(id) => Ok(id),
                        Err(_) => {
                            ctx.remove_display_by_id(display.id());
                            Err(RError::new(
                                ErrorCode::InternalServerError,
                                "Couldn't cast display id to u32. Removing display...".to_string(),
                            ))
                        }
                    }
                })
                .await
            })
        })
        .mutation("set_pos", |t| {
            t(|_ctx, coords: ObsPositionPayload| async move {
                run_with_obs_rspc(move |mgr| {
                    let display = get_display(mgr, coords.id)?;
                    let r = display.set_pos(coords.x, coords.y);

                    to_internal_res(r)
                })
                .await
            })
        })
        .mutation("set_size", |t| {
            t(|_ctx, coords: ObsSizePayload| async move {
                run_with_obs_rspc(move |mgr| {
                    let display = get_display(mgr, coords.id)?;

                    let r = display.set_size(coords.width, coords.height);
                    to_internal_res(r)
                })
                .await
            })
        })
        .mutation("destroy", |t| {
            t(|_ctx, id: DisplayId| async move {
                run_with_obs_rspc(move |mgr| {
                    let id: usize = id.try_into().map_err(|_| {
                        RError::new(
                            ErrorCode::BadRequest,
                            "Couldn't cast u32 display id to usize".to_string(),
                        )
                    })?;

                    mgr.context().remove_display_by_id(id);
                    Ok(())
                })
                .await
            })
        })
}
