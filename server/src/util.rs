use std::str::from_utf8;

use aes_gcm_siv::{aead::Aead, Nonce};
use argon2::Argon2;
use base64::engine::{general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, SubsecRound, Utc};
use deadpool_postgres::{Client, Pool};
use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use poem::{http::header, web::cookie::Cookie, Body, Request, Response, ResponseBuilder};
use rand::{
    distributions::{Alphanumeric, Standard},
    thread_rng, Rng,
};
use secstr::SecStr;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use totp_lite::totp_custom;

use crate::{
    config::Config,
    db::{Language, PasswordChangeReason, Permission},
    error::InternalError,
};

// COOKIE UTILS

fn add_cookie_fields(cookie: &mut Cookie, config: &Config) {
    cookie.set_http_only(true);
    cookie.set_path(&config.cookie.path);
    cookie.set_same_site(config.cookie.same_site);
    cookie.set_secure(config.cookie.secure);
}

pub fn clear_cookie(name: &str, config: &Config) -> Cookie {
    let mut cookie = Cookie::named(name);
    add_cookie_fields(&mut cookie, config);
    cookie.set_max_age(Duration::zero().to_std().unwrap());
    cookie
}

pub fn make_cookie(name: &str, value: &str, max_age: Option<Duration>, config: &Config) -> Cookie {
    let mut cookie = Cookie::new_with_str(name, value);
    add_cookie_fields(&mut cookie, config);
    if let Some(max_age) = max_age {
        cookie.set_max_age(max_age.to_std().unwrap());
    }
    cookie
}

pub fn remove_cookie(res: ResponseBuilder, name: &str, config: &Config) -> ResponseBuilder {
    res.header(header::SET_COOKIE, clear_cookie(name, config).to_string())
}

pub fn set_cookie(
    res: ResponseBuilder,
    name: &str,
    value: &str,
    max_age: Option<Duration>,
    config: &Config,
) -> ResponseBuilder {
    let mut cookie = Cookie::new_with_str(name, value);
    add_cookie_fields(&mut cookie, config);
    if let Some(max_age) = max_age {
        cookie.set_max_age(max_age.to_std().unwrap());
    }
    res.header(header::SET_COOKIE, cookie.to_string())
}

// CRYPTOGRAPHIC UTILS

pub fn base64_urlsafe(data: &[u8]) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(data)
}

pub fn hash(text: &str) -> Vec<u8> {
    blake3::hash(text.as_bytes()).as_bytes().to_vec()
}

const AES_GCM_SIV_NONCE_BYTES: usize = 12;

pub fn encrypt<R: Rng>(
    plaintext: &[u8],
    config: &Config,
    rng: &mut R,
) -> Result<Vec<u8>, InternalError> {
    if config.dev.debug {
        return Ok(plaintext.to_vec());
    }

    let mut nonce = [0u8; AES_GCM_SIV_NONCE_BYTES];
    rng.try_fill(&mut nonce).map_err(InternalError::new)?;
    let nonce = Nonce::from_slice(&nonce);

    let mut encrypted = config
        .aes
        .encrypt(nonce, plaintext)
        .map_err(|_| InternalError::new("encryption failed"))?;
    encrypted.extend_from_slice(nonce);
    Ok(encrypted)
}

pub fn decrypt(ciphertext_with_nonce: &[u8], config: &Config) -> Result<Vec<u8>, InternalError> {
    if config.dev.debug {
        return Ok(ciphertext_with_nonce.to_vec());
    }

    let input_length = ciphertext_with_nonce.len();
    if input_length < AES_GCM_SIV_NONCE_BYTES {
        return Err(InternalError::new(
            "encrypted data too short to contain nonce",
        ));
    }
    let (ciphertext, nonce) =
        ciphertext_with_nonce.split_at(input_length - AES_GCM_SIV_NONCE_BYTES);
    Ok(config
        .aes
        .decrypt(Nonce::from_slice(nonce), ciphertext.as_ref())
        .map_err(|_| InternalError::new("decryption failed"))?)
}

pub fn generate_token(length: u16) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length.into())
        .map(char::from)
        .collect()
}

// ENDPOINT INPUT UTILS

pub async fn get_db(req: &Request) -> Result<Client, InternalError> {
    req.data::<Pool>()
        .ok_or_else(|| InternalError::new("no database initialized"))?
        .get()
        .await
        .map_err(InternalError::new)
}

pub struct Session {
    pub csrf_token: String,
}

pub enum SessionError {
    ExpiredSession,
    InternalError(InternalError),
    InvalidSession,
    NoCookie,
}

pub async fn get_session(req: &Request, config: &Config) -> Result<Session, SessionError> {
    let session_cookie = req
        .cookie()
        .get(&config.session.cookie)
        .ok_or(SessionError::NoCookie)?
        .value_str()
        .to_owned();

    let db = get_db(req)
        .await
        .map_err(|err| SessionError::InternalError(err))?;
    let query = r#"SELECT "csrf_token", "expires" FROM "sessions" WHERE "id" = $1"#;
    let row = db
        .query_opt(query, &[&hash(&session_cookie)])
        .await
        .map_err(|err| SessionError::InternalError(InternalError::new(err)))?
        .ok_or(SessionError::InvalidSession)?;

    if row.get::<_, DateTime<Utc>>("expires") < utc_now() {
        Err(SessionError::ExpiredSession)
    } else {
        Ok(Session {
            csrf_token: row.get("csrf_token"),
        })
    }
}

pub enum RestoreSessionError {
    AccountDisabled,
    ExpiredSession,
    InternalError(InternalError),
    InvalidRememberToken,
    InvalidSession,
    NoCookies,
    RememberTokenMismatch,
}

pub struct RestoreSessionUser {
    pub id: String,
    pub active: bool,
    pub username: String,
    pub password_hash: Vec<u8>,
    pub totp_enabled: bool,
    pub password_change_reason: Option<PasswordChangeReason>,
    pub icon: Option<String>,
    pub language: Language,
}

pub struct RestoreSession {
    pub csrf_token: String,
    pub sudo_until: Option<DateTime<Utc>>,
    pub user: RestoreSessionUser,
}

pub async fn restore_session(
    req: &Request,
    config: &Config,
) -> Result<RestoreSession, RestoreSessionError> {
    let session_cookie = req
        .cookie()
        .get(&config.session.cookie)
        .map(|cookie| cookie.value_str().to_owned());
    let remember_cookie = req
        .cookie()
        .get(&config.remember_token.cookie)
        .map(|cookie| cookie.value_str().to_owned());

    let mut session_expired = false;
    let mut session_not_found = false;

    let mut db = get_db(req)
        .await
        .map_err(|err| RestoreSessionError::InternalError(err))?;
    let transaction = db
        .transaction()
        .await
        .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;

    if let Some(session_cookie) = session_cookie {
        let query = r#"
            SELECT
                "sessions"."csrf_token",
                "sessions"."expires",
                "sessions"."sudo_until",
                "users"."id",
                "users"."active",
                "users"."username",
                "users"."password",
                "users"."totp_key" IS NOT NULL AS "totp_enabled",
                "users"."password_change_reason",
                "users"."icon",
                "users"."language"
            FROM "sessions"
                JOIN "users" ON "sessions"."user_id" = "users"."id"
            WHERE "sessions"."id" = $1
        "#;
        let row = transaction
            .query_opt(query, &[&hash(&session_cookie)])
            .await
            .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;
        if let Some(row) = row {
            let active = row.get::<_, bool>("active");
            if row.get::<_, DateTime<Utc>>("expires") < utc_now() {
                session_expired = true;
            } else if !active {
                return Err(RestoreSessionError::AccountDisabled);
            } else {
                return Ok(RestoreSession {
                    csrf_token: row.get("csrf_token"),
                    sudo_until: row.get("sudo_until"),
                    user: RestoreSessionUser {
                        id: row.get("id"),
                        active,
                        username: row.get("username"),
                        password_hash: row.get("password"),
                        totp_enabled: row.get("totp_enabled"),
                        password_change_reason: row.get("password_change_reason"),
                        icon: row.get("icon"),
                        language: row.get("language"),
                    },
                });
            }
        } else {
            session_not_found = true;
        }
    }

    let Some(remember_cookie) = remember_cookie else {
        if session_expired {
            return Err(RestoreSessionError::ExpiredSession);
        }
        if session_not_found {
            return Err(RestoreSessionError::InvalidSession);
        }
        return Err(RestoreSessionError::NoCookies);
    };

    let (remember_token_id, remember_token_secret) =
        match remember_cookie.split_once(&config.remember_token.separator) {
            Some(parts) => parts,
            None => return Err(RestoreSessionError::InvalidRememberToken),
        };

    let query = r#"
        SELECT
            "remember_tokens"."secret",
            "users"."id",
            "users"."active",
            "users"."username",
            "users"."password",
            "users"."totp_key" IS NOT NULL AS "totp_enabled",
            "users"."password_change_reason",
            "users"."icon",
            "users"."language"
        FROM "remember_tokens"
            JOIN "users" ON "remember_tokens"."user_id" = "users"."id"
        WHERE "remember_tokens"."id" = $1
    "#;
    let remember_token_id_hash = hash(&remember_token_id);
    let row = transaction
        .query_opt(query, &[&remember_token_id_hash])
        .await
        .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?
        .ok_or(RestoreSessionError::InvalidRememberToken)?;

    let user_id = row.get::<_, String>("id");

    if SecStr::from(hash(remember_token_secret)) != SecStr::from(row.get::<_, &[u8]>("secret")) {
        // Possible session hijack attempt, invalidate everything
        transaction.execute(
            r#"DELETE FROM "sessions" WHERE "user_id" = $1"#,
            &[&user_id],
        )
            .await
            .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;

        transaction
            .execute(
                r#"DELETE FROM "remember_tokens" WHERE "user_id" = $1"#,
                &[&user_id],
            )
            .await
            .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;

        let updated = transaction
            .execute(
                r#"
                    UPDATE "users" SET "password_change_reason" = $1
                    WHERE "id" = $2
                "#,
                &[&PasswordChangeReason::RememberTokenCompromise, &user_id],
            )
            .await
            .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;

        if updated != 1 {
            return Err(RestoreSessionError::InternalError(InternalError::new(format!(
                "password_change_reason updated for {} users in restore_session",
                updated
            ))));
        }

        transaction
            .commit()
            .await
            .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;
        return Err(RestoreSessionError::RememberTokenMismatch);
    }

    let active = row.get::<_, bool>("active");
    if !active {
        return Err(RestoreSessionError::AccountDisabled);
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
        .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;

    let new_secret = generate_token(config.remember_token.secret_length);
    transaction
        .execute(
            r#"UPDATE "remember_tokens" SET "secret" = $1 WHERE "id" = $2"#,
            &[&hash(&new_secret), &remember_token_id_hash],
        )
        .await
        .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;

    transaction.commit()
        .await
        .map_err(|err| RestoreSessionError::InternalError(InternalError::new(err)))?;

    Ok(RestoreSession {
        csrf_token,
        sudo_until: None,
        user: RestoreSessionUser {
            id: user_id,
            active,
            username: row.get("username"),
            password_hash: row.get("password"),
            totp_enabled: row.get("totp_enabled"),
            password_change_reason: row.get("password_change_reason"),
            icon: row.get("icon"),
            language: row.get("language"),
        },
    })
}

// PASSWORD UTILS

pub fn make_argon2<'a>(
    memory_cost: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<Argon2<'a>, InternalError> {
    let mut argon2_params = argon2::ParamsBuilder::new();
    argon2_params
        .m_cost(memory_cost)
        .map_err(InternalError::new)?;
    argon2_params
        .t_cost(time_cost)
        .map_err(InternalError::new)?;
    argon2_params
        .p_cost(parallelism)
        .map_err(InternalError::new)?;
    let argon2_params = argon2_params.params().map_err(InternalError::new)?;
    assert!(argon2_params.m_cost() == memory_cost);
    Ok(Argon2::new(
        argon2::Algorithm::default(),
        argon2::Version::default(),
        argon2_params,
    ))
}

pub fn hash_encrypt_password(password: &str, config: &Config) -> Result<Vec<u8>, InternalError> {
    let salt = SaltString::b64_encode(
        &thread_rng()
            .sample_iter(&Standard)
            .take(config.security.password_salt_bytes)
            .collect::<Vec<u8>>(),
    )
    .map_err(InternalError::new)?;

    let hash = config
        .argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(InternalError::new)?
        .serialize();

    encrypt(hash.as_bytes(), &config, &mut thread_rng())
}

pub fn verify_password(
    password: &str,
    encrypted_hash_and_nonce: &[u8],
    config: &Config,
) -> Result<bool, InternalError> {
    let decrypted = decrypt(&encrypted_hash_and_nonce, config)?;
    let hash = from_utf8(&decrypted).map_err(InternalError::new)?;
    let parsed_hash =
        PasswordHash::parse(hash, password_hash::Encoding::B64).map_err(InternalError::new)?;
    match config
        .argon2
        .verify_password(password.as_bytes(), &parsed_hash)
    {
        Ok(()) => Ok(true),
        Err(password_hash::Error::Password) => Ok(false),
        Err(err) => Err(InternalError::new(err)),
    }
}

// REDIS UTILS

pub fn redis_join(parts: &[&str], config: &Config) -> String {
    parts.join(&config.redis.key_separator)
}

// RESPONSE UTILS

pub fn json_response<T: Serialize>(data: T) -> poem::Result<Response> {
    let body = Body::from_json(data).map_err(|err| InternalError::new(err))?;
    Ok(Response::builder()
        .content_type("application/json")
        .body(body))
}

pub fn build_json_response<T, F>(data: T, mutate: F) -> poem::Result<Response>
where
    T: Serialize,
    F: FnOnce(ResponseBuilder) -> ResponseBuilder,
{
    let res = mutate(Response::builder().content_type("application/json"));
    Ok(res.body(Body::from_json(data).map_err(InternalError::new)?))
}

// ROUTE UTILS

macro_rules! get {
    ($endpoint:ident) => {
        poem::get($endpoint).head($endpoint)
    };
}
pub(crate) use get;

// SERDE UTILS

pub fn optional<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

// TIME UTILS

pub fn utc_now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(0)
}

// TOTP UTILS

#[derive(Clone, Deserialize)]
pub enum TotpAlgorithm {
    #[serde(rename = "SHA-1")]
    Sha1,
    #[serde(rename = "SHA-256")]
    Sha256,
    #[serde(rename = "SHA-512")]
    Sha512,
}

pub fn generate_totp_key(config: &Config) -> Vec<u8> {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(config.totp.key_length)
        .collect()
}

pub fn generate_totp(key: &[u8], time: u64, config: &Config) -> String {
    let totp_fn = match config.totp.algorithm {
        TotpAlgorithm::Sha1 => totp_custom::<totp_lite::Sha1>,
        TotpAlgorithm::Sha256 => totp_custom::<totp_lite::Sha256>,
        TotpAlgorithm::Sha512 => totp_custom::<totp_lite::Sha512>,
    };
    totp_fn(config.totp.time_step.into(), config.totp.digits, key, time)
}

pub enum VerifyTotpError {
    InvalidTotp,
    InternalError(InternalError),
}

pub fn verify_totp(key: &[u8], totp: &str, config: &Config) -> Result<i64, VerifyTotpError> {
    let now = utc_now().timestamp();
    let now_unsigned = now
        .try_into()
        .map_err(|err| VerifyTotpError::InternalError(InternalError::new(err)))?;
    let totp = SecStr::from(totp);
    let verify = |time| totp == SecStr::from(generate_totp(key, time, config));

    let time_step: i64 = config.totp.time_step.into();
    let time_window: i64 = config.totp.time_window.into();

    if verify(now_unsigned) {
        return Ok(now / time_step);
    }
    if time_window == 0 {
        return Err(VerifyTotpError::InvalidTotp);
    }

    for i in 1..=time_window {
        let time = now - i * time_step;
        let time_unsigned: u64 = time
            .try_into()
            .map_err(|err| VerifyTotpError::InternalError(InternalError::new(err)))?;
        if verify(time_unsigned) {
            return Ok(time / time_step);
        }

        let time = now + i * time_step;
        let time_unsigned: u64 = time
            .try_into()
            .map_err(|err| VerifyTotpError::InternalError(InternalError::new(err)))?;
        if verify(time_unsigned) {
            return Ok(time / time_step);
        }
    }
    Err(VerifyTotpError::InvalidTotp)
}
