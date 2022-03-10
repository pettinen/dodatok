use std::fmt;

use poem::{
    error::{Error as PoemError, ResponseError},
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    web::cookie::Cookie,
    Body, Response,
};
use serde::ser::{Serialize, Serializer, SerializeMap};
use serde_json::{json, Value as JsonValue};
use tracing::error;

use crate::{util::{clear_cookie, set_cookie}, CONFIG};
use macros::error_enum;

pub trait Error: Serialize {
    fn to_tuple(&self) -> (String, String, Option<String>);
}

#[error_enum]
pub enum AuthError {
    AccountDisabled,
    AlreadyLoggedIn,
    InvalidCredentials,
    InvalidTotp,
    MissingTotp,
    NotLoggedIn,
    PasswordChangeRequired,
    SessionExpired,
    TotpReuse,
}

#[error_enum]
pub enum CsrfError {
    InvalidHeader,
    MissingCookie,
    MissingHeader,
    Mismatch,
}

#[error_enum]
pub enum GeneralError {
    InvalidData(String),
}

#[error_enum]
pub enum WebSocketError {
    AlreadyInRoom,
    InvalidMessageType,
    NotInRoom,
}

#[derive(Debug, thiserror::Error)]
#[error("forbidden")]
pub struct Forbidden;

impl ResponseError for Forbidden {
    fn status(&self) -> StatusCode {
        StatusCode::FORBIDDEN
    }
}

#[derive(Debug, thiserror::Error)]
#[error("bad-request")]
pub struct BadRequest<E> {
    error: E,
    csrf_token: Option<String>,
    cookies: Vec<Cookie>,
}

impl<E: Error> BadRequest<E> {
    pub fn new(error: E) -> BadRequest<E> {
        BadRequest {
            error,
            csrf_token: None,
            cookies: Vec::new(),
        }
    }

    pub fn csrf(mut self, csrf_token: String) -> Self {
        self.csrf_token = Some(csrf_token);
        self
    }

    pub fn remove_cookie(mut self, name: &str) -> Self {
        self.cookies.push(clear_cookie(name));
        self
    }
}

enum ResponseComplete {
    True,
}

impl<E: Error> ResponseError for BadRequest<E> {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn as_response(&self) -> Response {
        let mut res = Response::builder()
            .status(self.status())
            .content_type("application/json")
            .extension(ResponseComplete::True);
        let (src, id, details) = self.error.to_tuple();
        let mut body = single_error(&src, &id, details);
        for cookie in &self.cookies {
            res = res.header(header::SET_COOKIE, cookie.to_string());
        }
        if let Some(csrf_token) = &self.csrf_token {
            body.as_object_mut().unwrap().insert(
                CONFIG.csrf.response_field.clone(),
                csrf_token.to_owned().into(),
            );
            res = set_cookie(res, &CONFIG.csrf.cookie, &csrf_token);
        }
        res.body(Body::from_json(body).unwrap())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("internal-server-error")]
pub struct InternalError;

impl Serialize for InternalError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("source", "general")?;
        map.serialize_entry("id", &self.to_string())?;
        map.end()
    }
}

impl InternalError {
    pub fn new<E: fmt::Display>(err: E) -> InternalError {
        error!("internal error: {}", err);
        InternalError
    }
}

impl Error for InternalError {
    fn to_tuple(&self) -> (String, String, Option<String>) {
        ("general".to_owned(), self.to_string(), None)
    }
}

impl ResponseError for InternalError {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[derive(Debug, thiserror::Error)]
#[error("not-found")]
pub struct NotFound;

impl ResponseError for NotFound {
    fn status(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
}

pub async fn error_handler(err: PoemError) -> Response {
    let res = err.as_response();
    if let Some(ResponseComplete::True) = res.data::<ResponseComplete>() {
        return res;
    }

    let (parts, body) = res.into_parts();
    let (mut parts, body) = if parts.status.is_server_error() && !err.is::<InternalError>() {
        InternalError::new(&err).as_response().into_parts()
    } else {
        (parts, body)
    };

    let original_body = match body.into_string().await {
        Ok(body) => body,
        Err(err) => return InternalError::new(err).as_response(),
    };

    let status_code_name = match parts.status.canonical_reason() {
        Some(reason) => slugify(reason),
        None => {
            return InternalError::new(format!("unexpected status code {}", parts.status))
                .as_response()
        }
    };

    let (body, details) = if slugify(&original_body) == status_code_name {
        (status_code_name, None)
    } else {
        (status_code_name, Some(original_body))
    };
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Response::from_parts(
        parts,
        Body::from_json(single_error("general", &body, details)).unwrap(),
    )
}

fn api_error(source: &str, id: &str, details: Option<String>) -> JsonValue {
    let mut error = json!({ "source": source, "id": id });
    match details {
        Some(details) => {
            error
                .as_object_mut()
                .unwrap()
                .insert("details".to_owned(), JsonValue::String(details));
        }
        None => (),
    }
    error
}

fn single_error(source: &str, id: &str, details: Option<String>) -> JsonValue {
    json!({ "errors": [api_error(source, id, details)] })
}

fn slugify(value: &str) -> String {
    value.replace(' ', "-").to_lowercase()
}
