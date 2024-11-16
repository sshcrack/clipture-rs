use std::{path::PathBuf, sync::Arc};

use rspc::{Config, Router};

mod auth;
mod bootstrap;
mod game_detect;
mod obs;

use auth::auth;
use bootstrap::bootstrap;
use game_detect::game_detect;

pub fn router() -> Arc<Router<()>> {
    <Router>::new()
        .config(Config::new().export_ts_bindings(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/misc/bindings.ts"),
        ))
        .merge("auth.", auth())
        .merge("bootstrap.", bootstrap())
        .merge("game_detect.", game_detect())
        .merge("obs.", obs::obs())
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
