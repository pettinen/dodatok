use aes_gcm_siv::Aes256GcmSiv;
use deadpool_postgres::Pool;
use poem::{
    handler, post, EndpointExt, Request, Response, Result, Route,
    web::{Data, Json, cookie::CookieJar},
};
use secstr::SecStr;
use serde::Deserialize;
use serde_json::json;

use crate::{
    db::{Locale, PasswordChangeReason},
    error::{AuthError, AuthWarning, BadRequest, InternalError},
    middleware::{csrf, AuthRequired, AuthRequiredOptions, CurrentUser},
    util::{blake2, build_json_response, decrypt, generate_token, get, get_session, optional, remove_cookie, set_cookie, utc_now, verify_password, verify_totp, Session, SessionError},
    CONFIG,
};

#[handler]
async fn get_csrf_token(req: &Request) -> Result<Response> {
    let (csrf_token, remove_session_cookie) = match get_session(req).await {
        Ok(Session { csrf_token }) => (Some(csrf_token), false),
        Err(SessionError::ExpiredSession) | Err(SessionError::InvalidSession) => (None, true),
        Err(SessionError::InternalError(err)) => Err(err)?,
        Err(SessionError::NoCookie) => (None, false),
    };
    let csrf_token = match csrf_token {
        Some(csrf_token) => csrf_token,
        None => generate_token(CONFIG.csrf.token_length),
    };
    build_json_response(
        json!({ &CONFIG.csrf.response_field: csrf_token }),
        |res| {
            let res = set_cookie(
                res, &CONFIG.csrf.cookie, &csrf_token, Some(CONFIG.csrf.cookie_lifetime)
            );
            if remove_session_cookie {
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
        Ok(_) => Err(error_response(AuthError::AlreadyLoggedIn, false))?,
        Err(SessionError::ExpiredSession) | Err(SessionError::InvalidSession) => true,
        Err(SessionError::NoCookie) => false,
        Err(SessionError::InternalError(err)) => Err(err)?,
    };

    let mut warnings = Vec::<AuthWarning>::new();

    let mut db = db.get().await.map_err(InternalError::new)?;
    let select_query = r#"
        SELECT
            "id",
            "active",
            "username",
            "password",
            "totp_key",
            "last_used_totp",
            "password_change_reason",
            "icon",
            "locale"
        FROM "users" WHERE lower("username") = lower($1)
    "#;
    let user = db
        .query_opt(select_query, &[&data.username])
        .await
        .map_err(InternalError::new)?
        .ok_or_else(|| error_response(AuthError::InvalidCredentials, clear_session_cookie))?;

    if !verify_password(&data.password, user.get("password"), &aes)? {
        Err(error_response(AuthError::InvalidCredentials, clear_session_cookie))?;
    }

    let user_id = user.get::<_, &str>("id");
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    let totp_enabled = if let Some(encrypted_totp_key) = user.get("totp_key") {
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
        let updated = transaction.execute(
            r#"UPDATE "users" SET "last_used_totp" = $1 WHERE "id" = $2"#,
            &[&totp, &user_id],
        ).await.map_err(InternalError::new)?;
        if updated != 1 {
            Err(InternalError::new(format!("last_used_totp updated for {} users in login", updated)))?;
        }

        true
    } else if data.totp.is_some() {
        warnings.push(AuthWarning::UnusedTotp);
        false
    } else {
        false
    };

    if !user.get::<_, bool>("active") {
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
    let inserted = transaction
        .execute(
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
        Err(InternalError::new(format!("{} sessions inserted in login", inserted)))?;
    }

    let remember_token = if data.remember {
            let remember_token_id = generate_token(CONFIG.remember_token.id_length);
            let remember_token_secret = generate_token(CONFIG.remember_token.secret_length);
            let remember_token_secret_hash = blake2(&remember_token_secret);

            let insert_remember_token_query = r#"
                INSERT INTO "remember_tokens"("id", "user_id", "secret") VALUES ($1, $2, $3)
            "#;
            let inserted = transaction.execute(
                insert_remember_token_query,
                &[&blake2(&remember_token_id), &user_id, &remember_token_secret_hash]
            )
            .await
            .map_err(InternalError::new)?;
            if inserted != 1 {
                Err(InternalError::new(format!("{} remember tokens inserted in login", inserted)))?;
            }
            Some([remember_token_id, remember_token_secret].join(&CONFIG.remember_token.separator))
    } else {
        None
    };

    transaction.commit().await.map_err(InternalError::new)?;

    let mut response_json = json!({
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
    });
    if warnings.len() > 0 {
        if let Some(map) = response_json.as_object_mut() {
            map.insert("warnings".to_owned(), json!(warnings));
        }
    }

    build_json_response(
        response_json,
        |res| {
            let res = set_cookie(res, &CONFIG.session.cookie, &session_id, None);
            let res = set_cookie(
                res, &CONFIG.csrf.cookie, &csrf_token, Some(CONFIG.csrf.cookie_lifetime)
            );
            if let Some(remember_token) = remember_token {
                set_cookie(
                    res,
                    &CONFIG.remember_token.cookie,
                    &remember_token,
                    Some(CONFIG.remember_token.cookie_lifetime),
                )
            } else {
                res
            }
        }
    )
}

#[handler]
async fn logout(cookies: &CookieJar, db: Data<&Pool>, user: Data<&CurrentUser>) -> Result<Response> {
    let mut db = db.get().await.map_err(InternalError::new)?;
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    let deleted = transaction
        .execute(r#"DELETE FROM "sessions" WHERE "id" = $1"#, &[&user.session_id_hash])
        .await
        .map_err(InternalError::new)?;
    if deleted != 1 {
        Err(InternalError::new(format!("{} sessions deleted in logout", deleted)))?;
    }

    let maybe_remember_token = cookies.get(&CONFIG.remember_token.cookie);
    if let Some(ref remember_token) = maybe_remember_token {
        if let Some((remember_token_id, _)) = remember_token.value_str().split_once(&CONFIG.remember_token.separator) {
            let deleted = transaction.execute(
                r#"DELETE FROM "remember_tokens" WHERE "id" = $1"#,
                &[&blake2(remember_token_id)],
            ).await.map_err(InternalError::new)?;
            if deleted > 1 {
                Err(InternalError::new(format!("{} remember tokens deleted in logout", deleted)))?;
            }
        }
    }

    transaction.commit().await.map_err(InternalError::new)?;

    let csrf_token = generate_token(CONFIG.csrf.token_length);

    build_json_response(
        json!({ &CONFIG.csrf.response_field: csrf_token }),
        |res| {
            let res = set_cookie(
                res, &CONFIG.csrf.cookie, &csrf_token, Some(CONFIG.csrf.cookie_lifetime)
            );
            let res = remove_cookie(res, &CONFIG.session.cookie);
            if maybe_remember_token.is_some() {
                remove_cookie(res, &CONFIG.remember_token.cookie)
            } else {
                res
            }
        }
    )
}

#[handler]
async fn logout_all_sessions(
    cookies: &CookieJar,
    db: Data<&Pool>,
    user: Data<&CurrentUser>,
) -> Result<Response> {
    let mut db = db.get().await.map_err(InternalError::new)?;
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    let deleted = transaction
        .execute(r#"DELETE FROM "sessions" WHERE "user_id" = $1"#, &[&user.id])
        .await
        .map_err(InternalError::new)?;
    if deleted == 0 {
        Err(InternalError::new(format!("{} sessions deleted in logout_all_sessions", deleted)))?;
    }

    transaction.execute(
        r#"DELETE FROM "remember_tokens" WHERE "user_id" = $1"#,
        &[&user.id],
    ).await.map_err(InternalError::new)?;

    transaction.commit().await.map_err(InternalError::new)?;

    let csrf_token = generate_token(CONFIG.csrf.token_length);

    build_json_response(
        json!({ &CONFIG.csrf.response_field: csrf_token }),
        |res| {
            let mut res = set_cookie(
                res, &CONFIG.csrf.cookie, &csrf_token, Some(CONFIG.csrf.cookie_lifetime)
            );
            if let Some(_) = cookies.get(&CONFIG.remember_token.cookie) {
                res = remove_cookie(res, &CONFIG.remember_token.cookie);
            }
            remove_cookie(res, &CONFIG.session.cookie)
        }
    )
}

#[handler]
async fn restore_session(cookies: &CookieJar, db: Data<&Pool>) -> Result<Response> {
    fn error_response(error: AuthError, delete_remember_cookie: bool, delete_session_cookie: bool) -> poem::Error {
        let mut res = BadRequest::new(error);
        if delete_remember_cookie {
            res = res.remove_cookie(&CONFIG.remember_token.cookie);
        }
        if delete_session_cookie {
            res = res.remove_cookie(&CONFIG.session.cookie);
        }
        res.into()
    }

    if cookies.get(&CONFIG.session.cookie).is_some() {
        return Err(error_response(AuthError::AlreadyLoggedIn, false, false));
    }

    let remember_cookie = match cookies.get(&CONFIG.remember_token.cookie) {
        Some(remember_cookie) => remember_cookie,
        None => return Err(error_response(AuthError::MissingRememberToken, false, false)),
    };
    let (remember_token_id, remember_token_secret) = match remember_cookie.value_str().split_once(&CONFIG.remember_token.separator) {
        Some(parts) => parts,
        None => return Err(error_response(AuthError::InvalidRememberToken, true, false)),
    };
    let remember_token_id_hash = blake2(remember_token_id);

    let mut db = db.get().await.map_err(InternalError::new)?;
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    let query = r#"
        SELECT
            "remember_tokens"."id",
            "remember_tokens"."user_id",
            "remember_tokens"."secret",
            "users"."active"
        FROM "remember_tokens" JOIN "users" ON "remember_tokens"."user_id" = "users"."id"
        WHERE "remember_tokens"."id" = $1
    "#;
    let data = transaction
        .query_opt(query, &[&remember_token_id_hash])
        .await
        .map_err(InternalError::new)?
        .ok_or_else(|| error_response(AuthError::InvalidRememberToken, true, false))?;

    let user_id = data.get::<_, String>("user_id");

    if SecStr::from(blake2(remember_token_secret)) != SecStr::from(data.get::<_, &[u8]>("secret")) {
        // Possible session hijack attempt, invalidate sessions
        if let Err(err) = transaction
            .execute(r#"DELETE FROM "sessions" WHERE "user_id" = $1"#, &[&user_id])
            .await
        {
            InternalError::new(err);
        }
        if let Err(err) = transaction
            .execute(r#"DELETE FROM "remember_tokens" WHERE "user_id" = $1"#, &[&user_id])
            .await
        {
            InternalError::new(err);
        }
        match transaction.execute(
            r#"
                UPDATE "users" SET "password_change_reason" = 'session_compromise'
                WHERE "id" = $1
            "#,
            &[&user_id],
        ).await
        {
            Ok(updated) => if updated != 1 {
                Err(InternalError::new(format!("password_change_reason updated for {} users in restore_session", updated)))?;
            },
            Err(err) => Err(InternalError::new(err))?,
        }

        if let Err(err) = transaction.commit().await {
            InternalError::new(err);
        }
        let delete_session_cookie = cookies.get(&CONFIG.session.cookie).is_some();
        return Err(
            error_response(AuthError::RememberTokenSecretMismatch, true, delete_session_cookie)
        );
    }

    if !data.get::<_, bool>("active") {
        let delete_session_cookie = cookies.get(&CONFIG.session.cookie).is_some();
        return Err(error_response(AuthError::AccountDisabled, true, delete_session_cookie));
    }

    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let session_id = generate_token(CONFIG.session.id_length);
    let session_expires = utc_now() + CONFIG.session.lifetime;
    transaction
        .execute(
            r#"
                INSERT INTO "sessions"("id", "user_id", "csrf_token", "expires", "sudo_until")
                    VALUES ($1, $2, $3, $4, NULL)
            "#,
            &[&blake2(&session_id), &user_id, &csrf_token, &session_expires],
        )
        .await
        .map_err(InternalError::new)?;

    let new_secret = generate_token(CONFIG.remember_token.secret_length);
    transaction
        .execute(
            r#"UPDATE "remember_tokens" SET "secret" = $1 WHERE "id" = $2"#,
            &[&blake2(&new_secret), &remember_token_id_hash],
        )
        .await
        .map_err(InternalError::new)?;

    transaction.commit().await.map_err(InternalError::new)?;
    build_json_response(
        json!({ "csrfToken": csrf_token }),
        |res| {
            let res = set_cookie(
                res, &CONFIG.csrf.cookie, &csrf_token, Some(CONFIG.csrf.cookie_lifetime)
            );
            let res = set_cookie(
                res,
                &CONFIG.remember_token.cookie,
                &[remember_token_id, &new_secret].join(&CONFIG.remember_token.separator),
                Some(CONFIG.remember_token.cookie_lifetime),
            );
            set_cookie(res, &CONFIG.session.cookie, &session_id, None)
        },
    )
}

pub fn routes() -> Route {
    Route::new()
        .at("/csrf-token", get!(get_csrf_token))
        .at("/login", post(login.around(csrf)))
        .at("/logout", post(logout.with(AuthRequired::new(AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON)).around(csrf)))
        .at("/logout/all-sessions", post(logout.with(AuthRequired::new(AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON)).around(csrf)))
        .at("/restore-session", post(restore_session))
}
