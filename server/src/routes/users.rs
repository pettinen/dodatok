use deadpool_postgres::Pool;
use poem::{
    handler,
    web::{Data, Path},
    EndpointExt, Response, Result, Route,
};
use serde_json::json;

use crate::{
    db::{Locale, PasswordChangeReason, Permission},
    error::{Forbidden, InternalError, NotFound},
    middleware::{AuthRequired, AuthRequiredOptions, CurrentUser},
    util::{get, json_response},
};

fn current_user_response(current_user: &CurrentUser) -> Result<Response> {
    json_response(json!({
        "id": current_user.id,
        "username": current_user.username.as_ref().unwrap(),
        "totpEnabled": current_user.totp_enabled,
        "passwordChangeReason": current_user.password_change_reason,
        "icon": current_user.icon.as_ref().unwrap(),
        "locale": current_user.locale.as_ref().unwrap(),
        "sudoUntil": current_user.sudo_until.unwrap().map(|datetime| datetime.to_rfc3339()),
    }))
}

#[handler]
async fn get_me(current_user: Data<&CurrentUser>) -> Result<Response> {
    current_user_response(&current_user)
}

#[handler]
async fn get_user(
    Path(user_id): Path<String>,
    db: Data<&Pool>,
    current_user: Data<&CurrentUser>,
) -> Result<Response> {
    if user_id == current_user.id {
        return current_user_response(&current_user);
    } else if !current_user
        .permissions
        .as_ref()
        .unwrap()
        .contains(&Permission::ViewUser)
    {
        return Err(Forbidden)?;
    }

    let db = db.get().await.map_err(InternalError::new)?;
    let query = r#"
        SELECT "id", "username", "password_change_reason", "icon", "locale"
        FROM "users" WHERE "id" = $1
    "#;
    let row = db
        .query_opt(query, &[&user_id])
        .await
        .map_err(InternalError::new)?
        .ok_or_else(|| NotFound)?;
    json_response(json!({
        "id": row.get::<_, &str>("id"),
        "username": row.get::<_, &str>("username"),
        "passwordChangeReason": row.get::<_, Option<PasswordChangeReason>>(
            "password_change_reason"
        ),
        "icon": row.get::<_, Option<&str>>("icon"),
        "locale": row.get::<_, Locale>("locale"),
    }))
}

pub fn routes() -> Route {
    Route::new()
        .at(
            "/:user_id",
            get!(get_user).with(AuthRequired::new(
                AuthRequiredOptions::WITH_USERNAME
                    | AuthRequiredOptions::WITH_TOTP_STATUS
                    | AuthRequiredOptions::WITH_ICON
                    | AuthRequiredOptions::WITH_LOCALE
                    | AuthRequiredOptions::WITH_PERMISSIONS
                    | AuthRequiredOptions::WITH_SUDO_UNTIL,
            ))
        )
        .at(
            "/me",
            get!(get_me).with(AuthRequired::new(
                AuthRequiredOptions::WITH_USERNAME
                    | AuthRequiredOptions::WITH_TOTP_STATUS
                    | AuthRequiredOptions::WITH_ICON
                    | AuthRequiredOptions::WITH_LOCALE
                    | AuthRequiredOptions::WITH_PERMISSIONS
                    | AuthRequiredOptions::WITH_SUDO_UNTIL
                    | AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON,
            ))
        )
}
