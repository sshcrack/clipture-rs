
use rspc::{Router, RouterBuilder};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Type, Serialize, Deserialize)]
pub struct ObsBootstrapProgress {
    pub progress: f32,
    pub message: String
}

pub fn bootstrap() -> RouterBuilder
{
    <Router>::new()
        .subscription("bootstrap", |t| {
            t(|ctx, input: ()| {
                async_stream::stream! {
                    yield ObsBootstrapProgress {
                        progress: 0.0,
                        message: "Verifying installation...".to_string()
                    }
                }
            })
        })
}
