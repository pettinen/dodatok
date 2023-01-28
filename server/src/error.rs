use std::fmt;

use poem::{
    error::{
        GetDataError, MethodNotAllowedError, NotFoundError, ParseJsonError, ParseMultipartError,
        ParsePathError, ParseQueryError, ReadBodyError, ResponseError, UpgradeError,
    },
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    web::cookie::Cookie,
    Body, Response,
};
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::{json, Value as JsonValue};
use tracing::error;

use macros::alert_enum;

#[derive(Clone, Debug, Default)]
pub struct ErrorData {
    pub cookies: Vec<Cookie>,
    pub csrf_token: Option<(String, String)>,
    pub details: Option<String>,
}

#[alert_enum(response_error)]
pub enum AuthError {
    AccountDisabled,
    AlreadyLoggedIn,
    Forbidden,
    InvalidCredentials,
    InvalidRememberToken,
    InvalidTotp,
    MissingRememberToken,
    MissingTotp,
    NotLoggedIn,
    PasswordChangeRequired,
    RememberTokenSecretMismatch,
    SessionExpired,
    TotpReuse,
}

impl ResponseError for AuthError {
    fn status(&self) -> StatusCode {
        match self {
            Self::AccountDisabled(_) => StatusCode::FORBIDDEN,
            Self::AlreadyLoggedIn(_) => StatusCode::BAD_REQUEST,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::InvalidCredentials(_) => StatusCode::BAD_REQUEST,
            Self::InvalidRememberToken(_) => StatusCode::BAD_REQUEST,
            Self::InvalidTotp(_) => StatusCode::BAD_REQUEST,
            Self::MissingRememberToken(_) => StatusCode::BAD_REQUEST,
            Self::MissingTotp(_) => StatusCode::BAD_REQUEST,
            Self::NotLoggedIn(_) => StatusCode::UNAUTHORIZED,
            Self::PasswordChangeRequired(_) => StatusCode::BAD_REQUEST,
            Self::RememberTokenSecretMismatch(_) => StatusCode::BAD_REQUEST,
            Self::SessionExpired(_) => StatusCode::FORBIDDEN,
            Self::TotpReuse(_) => StatusCode::BAD_REQUEST,
        }
    }
}

#[alert_enum]
pub enum AuthWarning {
    UnusedTotp,
}

#[alert_enum(response_error)]
pub enum CsrfError {
    InvalidHeader,
    MissingCookie,
    MissingHeader,
    Mismatch,
}

impl ResponseError for CsrfError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[alert_enum(response_error)]
pub enum GeneralError {
    InvalidData,
    NotFound,
}

////////////////////

impl ResponseError for GeneralError {
    fn status(&self) -> StatusCode {
        match self {
            Self::InvalidData(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}

#[alert_enum]
pub enum WebSocketError {
    AlreadyInRoom,
    InvalidMessageType,
    NotInRoom,
}

#[derive(Debug, thiserror::Error)]
#[error("internal-server-error")]
pub struct InternalError;

impl InternalError {
    pub fn new<E: fmt::Display>(err: E) -> InternalError {
        error!("internal error: {}", err);
        InternalError
    }

    fn to_tuple(&self) -> (String, String, Option<String>) {
        ("general".to_owned(), self.to_string(), None)
    }
}

impl Serialize for InternalError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        let (src, id, _) = self.to_tuple();
        map.serialize_entry("source", &src)?;
        map.serialize_entry("id", &id)?;
        map.end()
    }
}

impl ResponseError for InternalError {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn as_response(&self) -> Response {
        let res = Response::builder()
            .status(self.status())
            .content_type("application/json");
        let (src, id, details) = self.to_tuple();
        let body = single_error(&src, &id, details);
        res.body(Body::from_json(body).unwrap())
    }
}

pub async fn error_handler(err: poem::Error) -> Response {
    if let Some(err) = err.downcast_ref::<AuthError>() {
        return err.as_response();
    }
    if let Some(err) = err.downcast_ref::<CsrfError>() {
        return err.as_response();
    }
    if let Some(err) = err.downcast_ref::<GeneralError>() {
        return err.as_response();
    }
    if let Some(err) = err.downcast_ref::<InternalError>() {
        return err.as_response();
    }
    if let Some(err) = err.downcast_ref::<GetDataError>() {
        return GeneralError::InvalidData(Some(ErrorData {
            details: Some(err.to_string()),
            ..Default::default()
        }))
        .as_response();
    };
    if let Some(err) = err.downcast_ref::<ParseJsonError>() {
        if let ParseJsonError::Parse(err) = err {
            return GeneralError::InvalidData(Some(ErrorData {
                details: Some(err.to_string()),
                ..Default::default()
            }))
            .as_response();
        }
    };
    if err.is::<ParsePathError>() {
        return GeneralError::NotFound(None).as_response();
    };
    if let Some(err) = err.downcast_ref::<ParseQueryError>() {
        return GeneralError::InvalidData(Some(ErrorData {
            details: Some(err.to_string()),
            ..Default::default()
        }))
        .as_response();
    };
    if let Some(err) = err.downcast_ref::<ReadBodyError>() {
        if let ReadBodyError::BodyHasBeenTaken = err {
            return InternalError::new(err.to_string()).as_response();
        }
    }
    if let Some(err) = err.downcast_ref::<UpgradeError>() {
        if let UpgradeError::NoUpgrade = err {
            return InternalError::new(err.to_string()).as_response();
        }
    }
    if let Some(err) = err.downcast_ref::<poem::error::WebSocketError>() {
        if let poem::error::WebSocketError::UpgradeError(err) = err {
            if let UpgradeError::NoUpgrade = err {
                return InternalError::new(err.to_string()).as_response();
            }
        }
    }

    if !err.is::<MethodNotAllowedError>()    // id = "method-not-allowed"
        && !err.is::<NotFoundError>()        // id = "not-found"
        && !err.is::<ParseJsonError>()       // id = "unsupported-media-type"
        && !err.is::<ParseMultipartError>()  // id = "bad-request" or "unsupported-media-type"
        && !err.is::<ReadBodyError>()        // id = "bad-request" or "payload-too-large"
        && !err.is::<UpgradeError>()
    // id = "bad-request"
    {
        // We're not expecting any other error types from Poem
        return InternalError::new(err).as_response();
    }

    let (mut parts, body) = err.into_response().into_parts();
    let original_body = match body.into_string().await {
        Ok(body) => body,
        Err(err) => return InternalError::new(err).as_response(),
    };

    let error_id = match parts.status.canonical_reason() {
        Some(reason) => slugify(reason),
        None => {
            return InternalError::new(format!("unexpected status code {}", parts.status))
                .as_response()
        }
    };

    let (error_id, details) = if slugify(&original_body) == error_id {
        (error_id, None)
    } else {
        (error_id, Some(original_body))
    };
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Response::from_parts(
        parts,
        Body::from_json(single_error("general", &error_id, details)).unwrap(),
    )
}

fn api_alert(source: &str, id: &str, details: Option<String>) -> JsonValue {
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
    json!({ "errors": [api_alert(source, id, details)] })
}

fn slugify(value: &str) -> String {
    value.replace(' ', "-").to_lowercase()
}
