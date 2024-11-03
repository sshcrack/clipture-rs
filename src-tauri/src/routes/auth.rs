use rspc::{ErrorCode, RouterBuilder};

use crate::auth::AUTH_MANAGER;

pub fn auth() -> RouterBuilder {
    <RouterBuilder>::new() //
        .mutation("login", |t| {
            t(|_ctx, _input: ()| async {
                let mut auth = AUTH_MANAGER.write().await;
                let r = auth
                    .as_mut()
                    .expect("Should have auth manager")
                    .login()
                    .await;

                if let Err(e) = r {
                    log::error!("Error logging in: {:?}", e);
                    return Err(rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("{}", e),
                    ));
                }

                Ok(())
            })
        })
        .query("is_logged_in", |t| {
            t(|_ctx, _input: ()| async {
                let auth = AUTH_MANAGER.read().await;
                let auth = auth.as_ref().expect("Should have auth manager");

                Ok(auth.is_logged_in())
            })
        })
}
