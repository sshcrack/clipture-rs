use libobs_sources::windows::MonitorCaptureSourceBuilder;
use libobs_wrapper::{sources::ObsSourceBuilder, data::ObsObjectBuilder};

use crate::run_obs;

pub async fn prepare_example() -> anyhow::Result<()> {
    run_obs(|ctx| {
        let scene = ctx.scene("main_scene");
        let monitors = MonitorCaptureSourceBuilder::get_monitors().unwrap();
        let e = monitors.get(1).unwrap_or(monitors.get(0).unwrap());

        MonitorCaptureSourceBuilder::new("test_monitor")
            .set_monitor(e)
            .add_to_scene(scene)
            .unwrap();

        scene.add_and_set(0);
        Ok(())
    }).await
}
