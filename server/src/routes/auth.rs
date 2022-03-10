use aes_gcm_siv::Aes256GcmSiv;
use deadpool_postgres::Pool;
use poem::{EndpointExt, Request, Response, Result, Route, get, handler, post, web::{Data, Json}};
use secstr::SecStr;
use serde::Deserialize;
use serde_json::json;

use crate::{
    db::{Locale, PasswordChangeReason},
    error::{AuthError, BadRequest, InternalError},
    middleware::{csrf, AuthRequired, CurrentUser},
    util::{blake2, build_json_response, decrypt, generate_token, get_session, optional, remove_cookie, set_cookie, utc_now, verify_password, verify_totp, Session, SessionError},
    CONFIG,
};

#[handler]
async fn get_csrf_token(req: &Request) -> Result<Response> {
    let (csrf_token, remove_cookie) = match get_session(req).await {
        Ok(Session { csrf_token }) => (Some(csrf_token), false),
        Err(SessionError::ExpiredSession) | Err(SessionError::InvalidSession) => (None, true),
        Err(SessionError::InternalError(err)) => return Err(err),
        Err(SessionError::NoCookie) => (None, false),
    };
    let csrf_token = match csrf_token {
        Some(csrf_token) => csrf_token,
        None => generate_token(CONFIG.csrf.token_length),
    };
    build_json_response(
        json!({ &CONFIG.csrf.response_field: csrf_token }),
        |res| {
            if remove_cookie {
                remove_cookie(res, &CONFIG.session.cookie)
            } else {
                res
            }
        }
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginData {
    username: String,
    password: String,
    #[serde(default, deserialize_with = "optional")]
    totp: Option<String>,
    remember: bool,
}

#[handler]
async fn login(
    aes: Data<&Aes256GcmSiv>,
    db: Data<&Pool>,
    req: &Request,
    Json(data): Json<LoginData>,
) -> Result<Response> {
    fn error_response(error: AuthError, clear_session_cookie: bool) -> BadRequest<AuthError> {
        let res = BadRequest::new(error);
        if clear_session_cookie {
            res.remove_cookie(&CONFIG.session.cookie)
        } else {
            res
        }
    }

    let clear_session_cookie = match get_session(req).await {
        Ok(_) => Err(BadRequest::new(AuthError::AlreadyLoggedIn))?,
        Err(SessionError::ExpiredSession) | Err(SessionError::InvalidSession) => true,
        Err(SessionError::InternalError(_)) | Err(SessionError::NoCookie) => false,
    };

    let mut db = db.get().await.map_err(InternalError::new)?;
    let select_query = r#"
        SELECT
            "id",
            "username",
            "password_hash",
            "totp_key",
            "last_used_totp",
            "password_change_reason",
            "disabled",
            "icon",
            "locale"
        FROM "users" WHERE lower("username") = lower($1)
    "#;
    let user = db
        .query_opt(select_query, &[&data.username])
        .await
        .map_err(InternalError::new)?
        .ok_or_else(|| error_response(AuthError::InvalidCredentials, clear_session_cookie))?;

    if !verify_password(&data.password, user.get("password_hash"), &aes)? {
        Err(error_response(AuthError::InvalidCredentials, clear_session_cookie))?;
    }

    let user_id = user.get::<_, &str>("id");
    let transaction = db.transaction().await.map_err(InternalError::new)?;
    let mut totp_enabled = false;

    if let Some(encrypted_totp_key) = user.get("totp_key") {
        let totp = match data.totp {
            Some(totp) => totp,
            None => Err(error_response(AuthError::MissingTotp, clear_session_cookie))?,
        };
        let totp_key = decrypt(encrypted_totp_key, &aes)?;
        if !verify_totp(&totp_key, &totp) {
            Err(error_response(AuthError::InvalidTotp, clear_session_cookie))?;
        }
        if let Some(last_used_totp) = user.get::<_, Option<String>>("last_used_totp") {
            if SecStr::from(totp.as_bytes()) == SecStr::from(last_used_totp) {
                Err(error_response(AuthError::TotpReuse, clear_session_cookie))?;
            }
        }
        transaction.execute(
            r#"UPDATE "users" SET "last_used_totp" = $1 WHERE "id" = $2"#,
            &[&totp, &user_id],
        ).await.map_err(InternalError::new)?;

        totp_enabled = true;
    }

    if user.get("disabled") {
        Err(error_response(AuthError::AccountDisabled, clear_session_cookie))?;
    }

    let session_id = generate_token(CONFIG.session.id_length);
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let now = utc_now();
    let session_expires = now + CONFIG.session.lifetime;
    let sudo_until = now + CONFIG.session.sudo_lifetime;

    let insert_session_query = r#"
        INSERT INTO "sessions"("id", "user_id", "csrf_token", "expires", "sudo_until")
        VALUES ($1, $2, $3, $4, $5)
    "#;
    let inserted = transaction.execute(
        insert_session_query,
        &[
            &blake2(&session_id),
            &user_id,
            &csrf_token,
            &session_expires,
            &sudo_until,
        ],
    )
        .await
        .map_err(InternalError::new)?;
    if inserted != 1 {
        Err(InternalError::new(format!("{} sessions inserted at login", inserted)))?;
    }

    if data.remember {
        let remember_token_id = generate_token(CONFIG.remember_token.id_length);
        let remember_token_secret = generate_token(CONFIG.remember_token.secret_length);
        let remember_token_secret_hash = blake2(&remember_token_secret);

        let insert_remember_token_query = r#"
            INSERT INTO "remember_tokens"("id", "user_id", "secret_hash") VALUES ($1, $2, $3)
        "#;
        let inserted = transaction.execute(
            insert_remember_token_query,
            &[&remember_token_id, &user_id, &remember_token_secret_hash]
        )
        .await
        .map_err(InternalError::new)?;
        if inserted != 1 {
            Err(InternalError::new(format!("{} remember tokens inserted at login", inserted)))?;
        }
    }

    transaction.commit().await.map_err(InternalError::new)?;

    build_json_response(
        json!({
            &CONFIG.csrf.response_field: csrf_token,
            "user": {
                "id": user.get::<_, &str>("id"),
                "username": user.get::<_, &str>("username"),
                "totpEnabled": totp_enabled,
                "passwordChangeReason": user.get::<_, Option<PasswordChangeReason>>(
                    "password_change_reason"
                ),
                "icon": user.get::<_, Option<&str>>("icon"),
                "locale": user.get::<_, Locale>("locale"),
                "sudoUntil": sudo_until.to_rfc3339(),
            },
        }),
        |res| {
            let res = set_cookie(res, &CONFIG.session.cookie, &session_id);
            set_cookie(res, &CONFIG.csrf.cookie, &csrf_token)
        }
    )
}

#[handler]
async fn logout(db: Data<&Pool>, user: Data<&CurrentUser>, req: &Request) -> Result<Response> {
    let mut db = db.get().await.map_err(InternalError::new)?;
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    transaction
        .execute(r#"DELETE FROM "sessions" WHERE "id" = $1"#, &[&user.session_id_hash])
        .await
        .map_err(InternalError::new)?;

    let maybe_remember_token = req.cookie().get(&CONFIG.remember_token.cookie);
    if let Some(ref remember_token) = maybe_remember_token {
        if let Some((remember_token_id, _)) = remember_token.value_str().split_once(&CONFIG.remember_token.separator) {
            transaction.execute(
                r#"DELETE FROM "remember_tokens" WHERE "id" = $1"#,
                &[&remember_token_id],
            ).await.map_err(InternalError::new)?;
        }
    }

    transaction.commit().await.map_err(InternalError::new)?;

    let csrf_token = generate_token(CONFIG.csrf.token_length);

    build_json_response(
        json!({ &CONFIG.csrf.response_field: csrf_token }),
        |res| {
            let mut res = set_cookie(res, &CONFIG.csrf.cookie, &csrf_token);
            if let Some(_) = maybe_remember_token {
                res = remove_cookie(res, &CONFIG.remember_token.cookie);
            }
            remove_cookie(res, &CONFIG.session.cookie)
        }
    )
}

#[handler]
async fn logout_all_sessions(
    user: Data<&CurrentUser>,
    db: Data<&Pool>,
    req: &Request
) -> Result<Response> {
    let mut db = db.get().await.map_err(InternalError::new)?;
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    transaction
        .execute(r#"DELETE FROM "sessions" WHERE "user_id" = $1"#, &[&user.id])
        .await
        .map_err(InternalError::new)?;

    transaction.execute(
        r#"DELETE FROM "remember_tokens" WHERE "user_id" = $1"#,
        &[&user.id],
    ).await.map_err(InternalError::new)?;

    transaction.commit().await.map_err(InternalError::new)?;

    let csrf_token = generate_token(CONFIG.csrf.token_length);

    build_json_response(
        json!({ &CONFIG.csrf.response_field: csrf_token }),
        |res| {
            let mut res = set_cookie(res, &CONFIG.csrf.cookie, &csrf_token);
            if let Some(_) = req.cookie().get(&CONFIG.remember_token.cookie) {
                res = remove_cookie(res, &CONFIG.remember_token.cookie);
            }
            remove_cookie(res, &CONFIG.session.cookie)
        }
    )
}

pub fn routes() -> Route {
    Route::new()
        .at("/csrf-token", get(get_csrf_token))
        .at("/login", post(login.around(csrf)))
        .at("/logout", post(logout.with(AuthRequired::default()).around(csrf)))
        .at("/logout/all-sessions", post(logout.with(AuthRequired::default()).around(csrf)))
}
