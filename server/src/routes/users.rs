use deadpool_postgres::Pool;
use poem::{
    handler,
    web::{Data, Path},
    EndpointExt, Response, Result, Route,
};
use serde_json::json;

use crate::{
    config::Config,
    db::{Language, PasswordChangeReason, Permission},
    error::{AuthError, GeneralError, InternalError},
    middleware::{AuthRequired, AuthRequiredOptions, CurrentUser},
    util::{get, json_response},
};

fn current_user_response(current_user: &CurrentUser) -> Result<Response> {
    json_response(json!({
        "success": true,
        "data": {
            "id": current_user.id,
            "username": current_user.username.as_ref().unwrap(),
            "totp_enabled": current_user.totp_enabled,
            "password_change_reason": current_user.password_change_reason,
            "icon": current_user.icon.as_ref().unwrap(),
            "language": current_user.language.as_ref().unwrap(),
        },
        "sudo_until": current_user.sudo_until.unwrap().map(|datetime| datetime.to_rfc3339()),
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
        return Err(AuthError::Forbidden(None).into());
    }

    let db = db.get().await.map_err(InternalError::new)?;
    let query = r#"
        SELECT "id", "username", "password_change_reason", "icon", "language"
        FROM "users" WHERE "id" = $1
    "#;
    let row = db
        .query_opt(query, &[&user_id])
        .await
        .map_err(InternalError::new)?
        .ok_or(GeneralError::NotFound(None))?;
    json_response(json!({
        "success": true,
        "data": {
            "id": row.get::<_, &str>("id"),
            "username": row.get::<_, &str>("username"),
            "password_change_reason": row.get::<_, Option<PasswordChangeReason>>(
                "password_change_reason"
            ),
            "icon": row.get::<_, Option<&str>>("icon"),
            "language": row.get::<_, Language>("language"),
        },
    }))
}

pub fn routes(config: &Config) -> Route {
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
                config.clone(),
            )),
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
                config.clone(),
            )),
        )
}
