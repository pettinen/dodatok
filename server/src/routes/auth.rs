use deadpool_postgres::Pool;
use poem::{
    handler, post,
    web::{cookie::CookieJar, Data, Json},
    EndpointExt, Request, Response, Result, Route,
};
use secstr::SecStr;
use serde::Deserialize;
use serde_json::json;

use crate::{
    config::Config,
    db::{Language, PasswordChangeReason},
    error::{AuthError, AuthWarning, ErrorData, InternalError},
    middleware::{AuthRequired, AuthRequiredOptions, Csrf, CurrentUser},
    util::{
        build_json_response, clear_cookie, decrypt, generate_token, get, get_session, hash,
        optional, remove_cookie, set_cookie, utc_now, verify_password, verify_totp, Session,
        SessionError, VerifyTotpError,
    },
};

#[handler]
async fn get_csrf_token(config: Data<&Config>, req: &Request) -> Result<Response> {
    let (csrf_token, remove_session_cookie) = match get_session(req, &config).await {
        Ok(Session { csrf_token }) => (Some(csrf_token), false),
        Err(SessionError::ExpiredSession) | Err(SessionError::InvalidSession) => (None, true),
        Err(SessionError::InternalError(err)) => Err(err)?,
        Err(SessionError::NoCookie) => (None, false),
    };
    let csrf_token = match csrf_token {
        Some(csrf_token) => csrf_token,
        None => generate_token(config.csrf.token_length),
    };
    build_json_response(json!({
        "success": true,
        "data": null,
        &config.csrf.response_field: csrf_token,
    }), |res| {
        let res = set_cookie(
            res,
            &config.csrf.cookie,
            &csrf_token,
            Some(config.csrf.cookie_lifetime),
            &config,
        );
        if remove_session_cookie {
            remove_cookie(res, &config.session.cookie, &config)
        } else {
            res
        }
    })
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
    config: Data<&Config>,
    db: Data<&Pool>,
    req: &Request,
    Json(data): Json<LoginData>,
) -> Result<Response> {
    let error = |error: AuthError, delete_session_cookie: bool| {
        let data = if delete_session_cookie {
            Some(ErrorData {
                cookies: vec![clear_cookie(&config.session.cookie, &config)],
                ..Default::default()
            })
        } else {
            Some(ErrorData::default())
        };
        Err(match error {
            AuthError::AccountDisabled(_) => AuthError::AccountDisabled(data),
            AuthError::AlreadyLoggedIn(_) => AuthError::AlreadyLoggedIn(data),
            AuthError::Forbidden(_) => AuthError::Forbidden(data),
            AuthError::InvalidCredentials(_) => AuthError::InvalidCredentials(data),
            AuthError::InvalidRememberToken(_) => AuthError::InvalidRememberToken(data),
            AuthError::InvalidTotp(_) => AuthError::InvalidTotp(data),
            AuthError::MissingRememberToken(_) => AuthError::MissingRememberToken(data),
            AuthError::MissingTotp(_) => AuthError::MissingTotp(data),
            AuthError::NotLoggedIn(_) => AuthError::NotLoggedIn(data),
            AuthError::PasswordChangeRequired(_) => AuthError::PasswordChangeRequired(data),
            AuthError::RememberTokenSecretMismatch(_) => {
                AuthError::RememberTokenSecretMismatch(data)
            }
            AuthError::SessionExpired(_) => AuthError::SessionExpired(data),
            AuthError::TotpReuse(_) => AuthError::TotpReuse(data),
        }
        .into())
    };

    let clear_session_cookie = match get_session(req, &config).await {
        Ok(_) => return Err(AuthError::AlreadyLoggedIn(None).into()),
        Err(SessionError::ExpiredSession) | Err(SessionError::InvalidSession) => true,
        Err(SessionError::NoCookie) => false,
        Err(SessionError::InternalError(err)) => return Err(err.into()),
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
            "last_totp_time_step",
            "password_change_reason",
            "icon",
            "language"
        FROM "users" WHERE lower("username") = lower($1)
    "#;
    let user = db
        .query_opt(select_query, &[&data.username])
        .await
        .map_err(InternalError::new)?;

    let Some(user) = user else {
        return error(AuthError::InvalidCredentials(None), clear_session_cookie);
    };

    if !verify_password(&data.password, user.get("password"), &config)? {
        tracing::warn!("invalid password, returning error");
        return error(AuthError::InvalidCredentials(None), clear_session_cookie);
    }

    let user_id = user.get::<_, &str>("id");
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    let totp_enabled = if let Some(encrypted_totp_key) = user.get("totp_key") {
        let totp = match data.totp {
            Some(totp) => totp,
            None => return error(AuthError::MissingTotp(None), clear_session_cookie),
        };
        let totp_key = decrypt(encrypted_totp_key, &config)?;
        let totp_time_step = verify_totp(&totp_key, &totp, &config);
        let totp_time_step = match totp_time_step {
            Ok(totp_time_step) => totp_time_step,
            Err(VerifyTotpError::InvalidTotp) => {
                return error(AuthError::InvalidTotp(None), clear_session_cookie)
            }
            Err(VerifyTotpError::InternalError(err)) => return Err(err.into()),
        };

        if let Some(last_totp_time_step) = user.get::<_, Option<i64>>("last_totp_time_step")
        {
            if totp_time_step <= last_totp_time_step {
                return error(AuthError::TotpReuse(None), clear_session_cookie);
            }
        }

        let updated = transaction
            .execute(
                r#"UPDATE "users" SET "last_totp_time_step" = $1 WHERE "id" = $2"#,
                &[&totp_time_step, &user_id],
            )
            .await
            .map_err(InternalError::new)?;
        if updated != 1 {
            Err(InternalError::new(format!(
                "last_totp_time_step updated for {} users in login",
                updated
            )))?;
        }

        true
    } else if data.totp.is_some() {
        warnings.push(AuthWarning::UnusedTotp(None));
        false
    } else {
        false
    };

    if !user.get::<_, bool>("active") {
        return error(AuthError::AccountDisabled(None), clear_session_cookie);
    }

    let session_id = generate_token(config.session.id_length);
    let csrf_token = generate_token(config.csrf.token_length);
    let now = utc_now();
    let session_expires = now + config.session.lifetime;
    let sudo_until = now + config.session.sudo_lifetime;

    let insert_session_query = r#"
        INSERT INTO "sessions"("id", "user_id", "csrf_token", "expires", "sudo_until")
        VALUES ($1, $2, $3, $4, $5)
    "#;
    let inserted = transaction
        .execute(
            insert_session_query,
            &[
                &hash(&session_id),
                &user_id,
                &csrf_token,
                &session_expires,
                &sudo_until,
            ],
        )
        .await
        .map_err(InternalError::new)?;
    if inserted != 1 {
        Err(InternalError::new(format!(
            "{} sessions inserted in login",
            inserted
        )))?;
    }

    let remember_token = if data.remember {
        let remember_token_id = generate_token(config.remember_token.id_length);
        let remember_token_secret = generate_token(config.remember_token.secret_length);
        let remember_token_secret_hash = hash(&remember_token_secret);

        let insert_remember_token_query = r#"
                INSERT INTO "remember_tokens"("id", "user_id", "secret") VALUES ($1, $2, $3)
            "#;
        let inserted = transaction
            .execute(
                insert_remember_token_query,
                &[
                    &hash(&remember_token_id),
                    &user_id,
                    &remember_token_secret_hash,
                ],
            )
            .await
            .map_err(InternalError::new)?;
        if inserted != 1 {
            Err(InternalError::new(format!(
                "{} remember tokens inserted in login",
                inserted
            )))?;
        }
        Some([remember_token_id, remember_token_secret].join(&config.remember_token.separator))
    } else {
        None
    };

    transaction.commit().await.map_err(InternalError::new)?;

    let mut response_json = json!({
        "success": true,
        &config.csrf.response_field: csrf_token,
        "data": {
            "id": user.get::<_, &str>("id"),
            "username": user.get::<_, &str>("username"),
            "totp_enabled": totp_enabled,
            "password_change_reason": user.get::<_, Option<PasswordChangeReason>>(
                "password_change_reason"
            ),
            "icon": user.get::<_, Option<&str>>("icon"),
            "language": user.get::<_, Language>("language"),
        },
        "sudo_until": sudo_until.to_rfc3339(),
    });
    if warnings.len() > 0 {
        if let Some(map) = response_json.as_object_mut() {
            map.insert("warnings".to_owned(), json!(warnings));
        }
    }

    build_json_response(response_json, |res| {
        let res = set_cookie(res, &config.session.cookie, &session_id, None, &config);
        let res = set_cookie(
            res,
            &config.csrf.cookie,
            &csrf_token,
            Some(config.csrf.cookie_lifetime),
            &config,
        );
        if let Some(remember_token) = remember_token {
            set_cookie(
                res,
                &config.remember_token.cookie,
                &remember_token,
                Some(config.remember_token.cookie_lifetime),
                &config,
            )
        } else {
            res
        }
    })
}

#[handler]
async fn logout(
    config: Data<&Config>,
    cookies: &CookieJar,
    db: Data<&Pool>,
    user: Data<&CurrentUser>,
) -> Result<Response> {
    let mut db = db.get().await.map_err(InternalError::new)?;
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    let deleted = transaction
        .execute(
            r#"DELETE FROM "sessions" WHERE "id" = $1"#,
            &[&user.session_id_hash],
        )
        .await
        .map_err(InternalError::new)?;
    if deleted != 1 {
        Err(InternalError::new(format!(
            "{} sessions deleted in logout",
            deleted
        )))?;
    }

    let maybe_remember_token = cookies.get(&config.remember_token.cookie);
    if let Some(ref remember_token) = maybe_remember_token {
        if let Some((remember_token_id, _)) = remember_token
            .value_str()
            .split_once(&config.remember_token.separator)
        {
            let deleted = transaction
                .execute(
                    r#"DELETE FROM "remember_tokens" WHERE "id" = $1"#,
                    &[&hash(remember_token_id)],
                )
                .await
                .map_err(InternalError::new)?;
            if deleted > 1 {
                Err(InternalError::new(format!(
                    "{} remember tokens deleted in logout",
                    deleted
                )))?;
            }
        }
    }

    transaction.commit().await.map_err(InternalError::new)?;

    let csrf_token = generate_token(config.csrf.token_length);

    build_json_response(json!({
        "success": true,
        "data": null,
        &config.csrf.response_field: csrf_token,
    }), |res| {
        let res = set_cookie(
            res,
            &config.csrf.cookie,
            &csrf_token,
            Some(config.csrf.cookie_lifetime),
            &config,
        );
        let res = remove_cookie(res, &config.session.cookie, &config);
        if maybe_remember_token.is_some() {
            remove_cookie(res, &config.remember_token.cookie, &config)
        } else {
            res
        }
    })
}

#[handler]
async fn logout_all_sessions(
    config: Data<&Config>,
    cookies: &CookieJar,
    db: Data<&Pool>,
    user: Data<&CurrentUser>,
) -> Result<Response> {
    let mut db = db.get().await.map_err(InternalError::new)?;
    let transaction = db.transaction().await.map_err(InternalError::new)?;

    let deleted = transaction
        .execute(
            r#"DELETE FROM "sessions" WHERE "user_id" = $1"#,
            &[&user.id],
        )
        .await
        .map_err(InternalError::new)?;
    if deleted == 0 {
        Err(InternalError::new(format!(
            "{} sessions deleted in logout_all_sessions",
            deleted
        )))?;
    }

    transaction
        .execute(
            r#"DELETE FROM "remember_tokens" WHERE "user_id" = $1"#,
            &[&user.id],
        )
        .await
        .map_err(InternalError::new)?;

    transaction.commit().await.map_err(InternalError::new)?;

    let csrf_token = generate_token(config.csrf.token_length);

    build_json_response(json!({
        "success": true,
        "data": null,
        &config.csrf.response_field: csrf_token
    }), |res| {
        let mut res = set_cookie(
            res,
            &config.csrf.cookie,
            &csrf_token,
            Some(config.csrf.cookie_lifetime),
            &config,
        );
        if let Some(_) = cookies.get(&config.remember_token.cookie) {
            res = remove_cookie(res, &config.remember_token.cookie, &config);
        }
        remove_cookie(res, &config.session.cookie, &config)
    })
}

#[handler]
async fn restore_session(
    config: Data<&Config>,
    cookies: &CookieJar,
    db: Data<&Pool>,
) -> Result<Response> {
    let error = |error: AuthError, delete_remember_cookie: bool, delete_session_cookie: bool| {
        let mut cookies = vec![];
        if delete_remember_cookie {
            cookies.push(clear_cookie(&config.remember_token.cookie, &config));
        }
        if delete_session_cookie {
            cookies.push(clear_cookie(&config.session.cookie, &config));
        }
        Err(match error {
            AuthError::AccountDisabled(_) => AuthError::AccountDisabled(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
            AuthError::AlreadyLoggedIn(_) => AuthError::AlreadyLoggedIn(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
            AuthError::Forbidden(_) => AuthError::Forbidden(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
            AuthError::InvalidCredentials(_) => AuthError::InvalidCredentials(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
            AuthError::InvalidRememberToken(_) => {
                AuthError::InvalidRememberToken(Some(ErrorData {
                    cookies,
                    ..Default::default()
                }))
            }
            AuthError::InvalidTotp(_) => AuthError::InvalidTotp(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
            AuthError::MissingRememberToken(_) => {
                AuthError::MissingRememberToken(Some(ErrorData {
                    cookies,
                    ..Default::default()
                }))
            }
            AuthError::MissingTotp(_) => AuthError::MissingTotp(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
            AuthError::NotLoggedIn(_) => AuthError::NotLoggedIn(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
            AuthError::PasswordChangeRequired(_) => {
                AuthError::PasswordChangeRequired(Some(ErrorData {
                    cookies,
                    ..Default::default()
                }))
            }
            AuthError::RememberTokenSecretMismatch(_) => {
                AuthError::RememberTokenSecretMismatch(Some(ErrorData {
                    cookies,
                    ..Default::default()
                }))
            }
            AuthError::SessionExpired(_) => AuthError::SessionExpired(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
            AuthError::TotpReuse(_) => AuthError::TotpReuse(Some(ErrorData {
                cookies,
                ..Default::default()
            })),
        }
        .into())
    };

    if cookies.get(&config.session.cookie).is_some() {
        return Err(AuthError::AlreadyLoggedIn(None).into());
    }

    let remember_cookie = match cookies.get(&config.remember_token.cookie) {
        Some(remember_cookie) => remember_cookie,
        None => return Err(AuthError::MissingRememberToken(None).into()),
    };
    let (remember_token_id, remember_token_secret) = match remember_cookie
        .value_str()
        .split_once(&config.remember_token.separator)
    {
        Some(parts) => parts,
        None => return error(AuthError::InvalidRememberToken(None), true, false),
    };
    let remember_token_id_hash = hash(remember_token_id);

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
        .map_err(InternalError::new)?;
    let Some(data) = data else {
        return error(AuthError::InvalidRememberToken(None), true, false);
    };

    let user_id = data.get::<_, String>("user_id");

    if SecStr::from(hash(remember_token_secret)) != SecStr::from(data.get::<_, &[u8]>("secret")) {
        // Possible session hijack attempt, invalidate sessions
        if let Err(err) = transaction
            .execute(
                r#"DELETE FROM "sessions" WHERE "user_id" = $1"#,
                &[&user_id],
            )
            .await
        {
            InternalError::new(err);
        }
        if let Err(err) = transaction
            .execute(
                r#"DELETE FROM "remember_tokens" WHERE "user_id" = $1"#,
                &[&user_id],
            )
            .await
        {
            InternalError::new(err);
        }
        match transaction
            .execute(
                r#"
                UPDATE "users" SET "password_change_reason" = 'remember_token_compromise'
                WHERE "id" = $1
            "#,
                &[&user_id],
            )
            .await
        {
            Ok(updated) => {
                if updated != 1 {
                    Err(InternalError::new(format!(
                        "password_change_reason updated for {} users in restore_session",
                        updated
                    )))?;
                }
            }
            Err(err) => Err(InternalError::new(err))?,
        }

        if let Err(err) = transaction.commit().await {
            InternalError::new(err);
        }
        let delete_session_cookie = cookies.get(&config.session.cookie).is_some();
        return error(
            AuthError::RememberTokenSecretMismatch(None),
            true,
            delete_session_cookie,
        );
    }

    if !data.get::<_, bool>("active") {
        let delete_session_cookie = cookies.get(&config.session.cookie).is_some();
        return error(
            AuthError::AccountDisabled(None),
            true,
            delete_session_cookie,
        );
    }

    let csrf_token = generate_token(config.csrf.token_length);
    let session_id = generate_token(config.session.id_length);
    let session_expires = utc_now() + config.session.lifetime;
    transaction
        .execute(
            r#"
                INSERT INTO "sessions"("id", "user_id", "csrf_token", "expires", "sudo_until")
                    VALUES ($1, $2, $3, $4, NULL)
            "#,
            &[&hash(&session_id), &user_id, &csrf_token, &session_expires],
        )
        .await
        .map_err(InternalError::new)?;

    let new_secret = generate_token(config.remember_token.secret_length);
    transaction
        .execute(
            r#"UPDATE "remember_tokens" SET "secret" = $1 WHERE "id" = $2"#,
            &[&hash(&new_secret), &remember_token_id_hash],
        )
        .await
        .map_err(InternalError::new)?;

    transaction.commit().await.map_err(InternalError::new)?;
    build_json_response(json!({ "csrf_token": csrf_token }), |res| {
        let res = set_cookie(
            res,
            &config.csrf.cookie,
            &csrf_token,
            Some(config.csrf.cookie_lifetime),
            &config,
        );
        let res = set_cookie(
            res,
            &config.remember_token.cookie,
            &[remember_token_id, &new_secret].join(&config.remember_token.separator),
            Some(config.remember_token.cookie_lifetime),
            &config,
        );
        set_cookie(res, &config.session.cookie, &session_id, None, &config)
    })
}

pub fn routes(config: &Config) -> Route {
    Route::new()
        .at("/csrf-token", get!(get_csrf_token))
        .at("/login", post(login.with(Csrf::new(config.clone()))))
        .at(
            "/logout",
            post(
                logout
                    .with(AuthRequired::new(
                        AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON,
                        config.clone(),
                    ))
                    .with(Csrf::new(config.clone())),
            ),
        )
        .at(
            "/logout/all-sessions",
            post(
                logout
                    .with(AuthRequired::new(
                        AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON,
                        config.clone(),
                    ))
                    .with(Csrf::new(config.clone())),
            ),
        )
        .at("/restore-session", post(restore_session))
}
