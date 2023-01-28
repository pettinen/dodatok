use poem::{handler, web::{Json, Data}, Response, Result, Route};
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::{AuthError, ErrorData, GeneralError, InternalError},
    util::{get, optional, json_response},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SomeData {
    s: String,
    n: u64,
    #[serde(default, deserialize_with = "optional")]
    o: Option<String>,
    b: bool,
}

#[handler]
async fn parse_json(Json(data): Json<SomeData>) -> Result<Response> {
    if data.o.is_none() {
        return Err(AuthError::InvalidCredentials(None).into());
    }
    if data.s.is_empty() {
        return Err(GeneralError::InvalidData(Some(ErrorData {
            details: Some("expected `s`".to_owned()),
            ..Default::default()
        })).into())
    }
    json_response(data)
}

#[handler]
async fn invalid_data() -> Result<Response> {
    return Err(GeneralError::InvalidData(Some(ErrorData {
        details: Some("the deets".to_owned()),
        ..Default::default()
    })).into())
}

#[handler]
async fn panic() -> Result<Response> {
    panic!("panicking");
}

#[handler]
async fn internal_error() -> Result<Response> {
    Err(InternalError::new("something went fuggy wuggy").into())
}

#[handler]
async fn totp() -> Result<Response> {
    use totp_lite::{totp_custom, Sha1};
    let totp1 = totp_custom::<Sha1>(1, 6, b"aaaa", 55826180);
    let totp2 = totp_custom::<Sha1>(30, 6, b"aaaa", 1674785411);
    json_response([totp1, totp2, (254u8 as i8).to_string()])
}

#[handler]
fn hash_password(config: Data<&Config>) -> Result<Response> {
    let hash = crate::util::hash_encrypt_password("AAAA", &config)?;
    let hash_str = std::str::from_utf8(&hash).map_err(InternalError::new)?;
    let Ok(correct) = crate::util::verify_password("AAAA", &hash, &config) else {
        return Err(InternalError::new("could not verify password").into());
    };
    let Ok(wrong) = crate::util::verify_password("AAAa", &hash, &config) else {
        return Err(InternalError::new("could not verify password").into());
    };
    json_response(serde_json::json!({
        "hash": hash_str,
        "correct": correct,
        "wrong": wrong,
    }))
}

#[handler]
fn err() -> Result<Response> {
    Err(poem::error::RouteError::InvalidPath("x".to_string()).into())
}

pub fn routes(_config: &Config) -> Route {
    Route::new()
        .at("/json", get!(parse_json))
        .at("/inv", get!(invalid_data))
        .at("/panic", get!(panic))
        .at("/ie", get!(internal_error))
        .at("/hash", get!(hash_password))
        .at("/totp", get!(totp))
        .at("/err", get!(err))
}
