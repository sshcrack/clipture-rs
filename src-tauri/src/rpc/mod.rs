use std::{path::PathBuf, sync::Arc};

use bootstrap::bootstrap;
use libobs_wrapper::display::ShowHideTrait;
use rspc::{Config, ErrorCode, Router};

use crate::run_obs;

mod bootstrap;
pub fn router() -> Arc<Router<()>> {
    <Router>::new()
        .config(Config::new().export_ts_bindings(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/misc/bindings.ts"),
        ))
        .query("display_toggle", |t| {
            t(|_ctx, _a: ()| async {
                run_obs(|ctx| {
                    let display = ctx.displays_mut().get_mut(0).unwrap();
                    if display.is_visible() {
                        display.hide();
                    } else {
                        display.show();
                    }

                    Ok(())
                })
                .await
                .map_err(|err| {
                    rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("Couldn't toggle display - {}", err),
                    )
                })
            })
        })
        .merge("", bootstrap())
        .build()
        .arced()
}

#[cfg(test)]
mod tests {
    // It is highly recommended to unit test your rspc router by creating it
    // This will ensure it doesn't have any issues and also export updated Typescript types.

    #[test]
    fn test_rspc_router() {
        super::router();
    }
}
