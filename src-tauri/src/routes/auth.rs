use rspc::{ErrorCode, RouterBuilder};

use crate::auth::AUTH_MANAGER;

pub fn auth() -> RouterBuilder {
    <RouterBuilder>::new() //
        .mutation("sign_in", |t| {
            t(|_ctx, _input: ()| async {
                let auth = AUTH_MANAGER.read().await;
                let r = auth
                    .as_ref()
                    .expect("Should have auth manager")
                    .sign_in()
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
        .mutation("sign_out", |t| {
            t(|_ctx, _input: ()| async {
                let auth = AUTH_MANAGER.read().await;
                let r = auth
                    .as_ref()
                    .expect("Should have auth manager")
                    .sign_out()
                    .await;

                if let Err(e) = r {
                    log::error!("Error signing out: {:?}", e);
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

                Ok(auth.is_logged_in().await)
            })
        })
        .query("open_auth_window", |t| {
            t(|_ctx, _input: ()| async {
                let auth = AUTH_MANAGER.read().await;
                let auth = auth.as_ref().expect("Should have auth manager");

                auth.open_sign_in_window();
            })
        })
}
