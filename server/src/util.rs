use std::str::from_utf8;

use aes_gcm_siv::{Aes256GcmSiv, Nonce, aead::Aead};
use argon2::{hash_encoded, verify_encoded, Config as Argon2Config};
use blake2::{Blake2s256, Digest};
use chrono::{DateTime, Duration, SubsecRound, Utc};
use deadpool_postgres::{Client, Pool};
use poem::{http::header, web::cookie::Cookie, Body, Request, Response, ResponseBuilder, Result};
use rand::{distributions::{Alphanumeric, Standard}, thread_rng, Rng};
use secstr::SecStr;
use serde::{Deserialize, Deserializer};
use serde_json::Value as JsonValue;
use totp_lite::totp_custom;

use crate::{error::InternalError, CONFIG};


// COOKIE UTILS

fn add_cookie_fields(cookie: &mut Cookie) {
    cookie.set_http_only(true);
    cookie.set_path(&CONFIG.cookie.path);
    cookie.set_same_site(CONFIG.cookie.same_site);
    cookie.set_secure(CONFIG.cookie.secure);
}

pub fn clear_cookie(name: &str) -> Cookie {
    let mut cookie = Cookie::named(name);
    add_cookie_fields(&mut cookie);
    cookie.set_max_age(Duration::zero().to_std().unwrap());
    cookie
}

pub fn remove_cookie(res: ResponseBuilder, name: &str) -> ResponseBuilder {
    res.header(header::SET_COOKIE, clear_cookie(name).to_string())
}

pub fn set_cookie(res: ResponseBuilder, name: &str, value: &str, max_age: Option<Duration>) -> ResponseBuilder {
    let mut cookie = Cookie::new_with_str(name, value);
    add_cookie_fields(&mut cookie);
    if let Some(max_age) = max_age {
        cookie.set_max_age(max_age.to_std().unwrap());
    }
    res.header(header::SET_COOKIE, cookie.to_string())
}

// CRYPTOGRAPHIC UTILS

pub fn base64_urlsafe(data: &[u8]) -> String {
    base64::encode_config(data, base64::URL_SAFE_NO_PAD)
}

pub fn blake2(text: &str) -> Vec<u8> {
    let mut hasher = Blake2s256::new();
    hasher.update(text.as_bytes());
    hasher.finalize().to_vec()
}

const AES_GCM_SIV_NONCE_BYTES: usize = 12;

pub fn encrypt<R: Rng>(plaintext: &[u8], cipher: &Aes256GcmSiv, rng: &mut R) -> Result<Vec<u8>, InternalError> {
    let mut nonce = [0u8; AES_GCM_SIV_NONCE_BYTES];
    rng.try_fill(&mut nonce).map_err(InternalError::new)?;
    let nonce = Nonce::from_slice(&nonce);

    let mut encrypted = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| InternalError::new("encryption failed"))?;
    encrypted.extend_from_slice(nonce);
    Ok(encrypted)
}

pub fn decrypt(ciphertext_with_nonce: &[u8], cipher: &Aes256GcmSiv) -> Result<Vec<u8>, InternalError> {
    let input_length = ciphertext_with_nonce.len();
    if input_length < AES_GCM_SIV_NONCE_BYTES {
        return Err(InternalError::new("encrypted data too short to contain nonce"));
    }
    let (ciphertext, nonce) =
        ciphertext_with_nonce.split_at(input_length - AES_GCM_SIV_NONCE_BYTES);
    Ok(cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext.as_ref())
        .map_err(|_| InternalError::new("decryption failed"))?)
}

pub fn generate_token(length: u16) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length as usize)
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

pub async fn get_session(req: &Request) -> Result<Session, SessionError> {
    let session_cookie = req
        .cookie()
        .get(&CONFIG.session.cookie)
        .ok_or_else(|| SessionError::NoCookie)?
        .value_str()
        .to_owned();

    let db = get_db(req).await.map_err(|err| SessionError::InternalError(err))?;
    let query = r#"SELECT "csrf_token", "expires" FROM "sessions" WHERE "id" = $1"#;
    let row = db
        .query_opt(query, &[&blake2(&session_cookie)])
        .await
        .map_err(|err| SessionError::InternalError(InternalError::new(err)))?
        .ok_or_else(|| SessionError::InvalidSession)?;

    if row.get::<_, DateTime<Utc>>("expires") < utc_now() {
        Err(SessionError::ExpiredSession)
    } else {
        Ok(Session { csrf_token: row.get("csrf_token") })
    }
}


// PASSWORD UTILS

#[allow(dead_code)]
pub fn hash_encrypt_password(password: &str, cipher: &Aes256GcmSiv) -> Result<Vec<u8>, InternalError> {
    let mut rng = thread_rng();
    let salt: Vec<u8> = thread_rng()
        .sample_iter(&Standard)
        .take(CONFIG.security.password_salt_bytes)
        .collect();
    let hash = hash_encoded(password.as_bytes(), &salt, &Argon2Config::default())
        .map_err(InternalError::new)?;

    encrypt(hash.as_bytes(), cipher, &mut rng)
}

pub fn verify_password(
    password: &str,
    encrypted_hash_and_nonce: Vec<u8>,
    cipher: &Aes256GcmSiv,
) -> Result<bool, InternalError> {
    let hash_bytes = decrypt(&encrypted_hash_and_nonce, cipher)?;
    let hash = from_utf8(&hash_bytes).map_err(InternalError::new)?;
    Ok(verify_encoded(hash, password.as_bytes()).map_err(InternalError::new)?)
}

// REDIS UTILS

pub fn redis_join(parts: &[&str]) -> String {
    parts.join(&CONFIG.redis.key_separator)
}

// RESPONSE UTILS

pub fn json_response(body: JsonValue) -> Result<Response> {
    Ok(Response::builder()
        .content_type("application/json")
        .body(Body::from_json(body).map_err(InternalError::new)?))
}

pub fn build_json_response<F>(body: JsonValue, mutate: F) -> Result<Response>
    where F: FnOnce(ResponseBuilder) -> ResponseBuilder
{
    let res = mutate(Response::builder().content_type("application/json"));
    Ok(res.body(Body::from_json(body).map_err(InternalError::new)?))
}

// ROUTE UTILS

macro_rules! get {
    ($endpoint:ident) => {
        poem::get($endpoint).head($endpoint)
    }
}
pub(crate) use get;

// SERDE UTILS

pub fn optional<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where T: Deserialize<'de>,
          D: Deserializer<'de>
{
    Deserialize::deserialize(deserializer).map(Some)
}

// TIME UTILS

pub fn utc_now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(0)
}

// TOTP UTILS

#[derive(Deserialize)]
pub enum TotpAlgorithm {
    #[serde(rename = "SHA-1")]
    Sha1,
    #[serde(rename = "SHA-256")]
    Sha256,
    #[serde(rename = "SHA-512")]
    Sha512,
}

pub fn generate_totp_key() -> Vec<u8> {
    thread_rng()
        .sample_iter(&Standard)
        .take(CONFIG.totp.key_bytes)
        .collect()
}

pub fn generate_totp(key: &[u8], time: u64) -> String {
    let totp_fn = match CONFIG.totp.algorithm {
        TotpAlgorithm::Sha1 => totp_custom::<totp_lite::Sha1>,
        TotpAlgorithm::Sha256 => totp_custom::<totp_lite::Sha256>,
        TotpAlgorithm::Sha512 => totp_custom::<totp_lite::Sha512>,
    };
    totp_fn(CONFIG.totp.time_step, CONFIG.totp.digits, key, time)
}

pub fn verify_totp(key: &[u8], totp: &str) -> bool {
    let now = utc_now().timestamp() as u64;
    let totp = SecStr::from(totp);
    let verify = |time| totp == SecStr::from(generate_totp(key, time));

    if verify(0) {
        return true;
    }
    for i in 1..=CONFIG.totp.time_window {
        let i = i as u64;
        if verify(now - i) || verify(now + i) {
            return true;
        }
    }
    false
}
