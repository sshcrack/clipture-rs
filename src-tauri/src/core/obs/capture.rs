use anyhow::bail;
use libobs_sources::windows::WindowCaptureSourceUpdater;
use libobs_window_helper::WindowInfo;
use libobs_wrapper::{data::ObsObjectUpdater, utils::traits::ObsUpdatable};

use super::ObsManager;

pub trait CaptureTrait {
    fn switch_window(&mut self, window: WindowInfo) -> anyhow::Result<()>;
}

impl CaptureTrait for ObsManager {
    fn switch_window(&mut self, window: WindowInfo) -> anyhow::Result<()> {
        if self.capture_source.id() != "window_capture" {
            bail!("Capture source is not window_capture");
        }

        let updater = self
            .capture_source
            .create_updater::<WindowCaptureSourceUpdater>();

        updater.set_window(&window).update();
        Ok(())
    }
}
