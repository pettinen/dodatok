use bitflags::bitflags;
use chrono::{DateTime, Utc};
use poem::{async_trait, error::Error as PoemError, Endpoint, Middleware, Request, Result};
use secstr::SecStr;

use crate::{
    db::{Locale, PasswordChangeReason, Permission},
    error::{AuthError, BadRequest, CsrfError, InternalError},
    util::{blake2, generate_token, get_db, utc_now},
    CONFIG,
};

bitflags! {
    #[derive(Default)]
    pub struct AuthRequiredOptions: u16 {
        const WITH_USERNAME = 1 << 0;
        const WITH_PASSWORD_HASH = 1 << 1;
        const WITH_TOTP_STATUS = 1 << 2;
        const WITH_PASSWORD_CHANGE_REASON = 1 << 3;
        const WITH_ICON = 1 << 4;
        const WITH_LOCALE = 1 << 5;
        const WITH_PERMISSIONS = 1 << 6;
        const WITH_SUDO_UNTIL = 1 << 7;
        const ALLOW_PASSWORD_CHANGE_REASON = 1 << 8;
    }
}

#[derive(Default)]
pub struct CurrentUser {
    pub id: String,
    pub session_id_hash: Vec<u8>,
    pub username: Option<String>,
    pub password_hash: Option<String>,
    pub totp_enabled: Option<bool>,
    pub password_change_reason: Option<Option<PasswordChangeReason>>,
    pub icon: Option<Option<String>>,
    pub locale: Option<Locale>,
    pub permissions: Option<Vec<Permission>>,
    pub sudo_until: Option<Option<DateTime<Utc>>>,
}

#[derive(Default)]
pub struct AuthRequired {
    options: AuthRequiredOptions,
}

impl AuthRequired {
    pub fn new(options: AuthRequiredOptions) -> AuthRequired {
        AuthRequired { options }
    }
}

impl<E: Endpoint> Middleware<E> for AuthRequired {
    type Output = AuthRequiredImpl<E>;

    fn transform(&self, endpoint: E) -> Self::Output {
        AuthRequiredImpl {
            endpoint,
            options: self.options,
        }
    }
}

pub struct AuthRequiredImpl<E> {
    endpoint: E,
    options: AuthRequiredOptions,
}

#[async_trait]
impl<E: Endpoint> Endpoint for AuthRequiredImpl<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let session_id = req
            .cookie()
            .get(&CONFIG.session.cookie)
            .ok_or_else(|| BadRequest::new(AuthError::NotLoggedIn))?
            .value_str()
            .to_owned();
        let session_id_hash = blake2(&session_id);

        let db = get_db(&req).await?;
        let mut columns = vec![
            r#""users"."id""#,
            r#""users"."disabled""#,
            r#""sessions"."expires""#,
        ];

        if self.options.contains(AuthRequiredOptions::WITH_USERNAME) {
            columns.push(r#""users"."username""#);
        }
        if self.options.contains(AuthRequiredOptions::WITH_PASSWORD_HASH) {
            columns.push(r#""users"."password_hash""#);
        }
        if self.options.contains(AuthRequiredOptions::WITH_TOTP_STATUS) {
            columns.push(r#""users"."totp_key" IS NOT NULL AS "totp_enabled""#);
        }
        if self.options.contains(AuthRequiredOptions::WITH_PASSWORD_CHANGE_REASON)
            || !self.options.contains(AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON)
        {
            columns.push(r#""users"."password_change_reason""#);
        }
        if self.options.contains(AuthRequiredOptions::WITH_ICON) {
            columns.push(r#""users"."icon""#);
        }
        if self.options.contains(AuthRequiredOptions::WITH_LOCALE) {
            columns.push(r#""users"."locale""#);
        }
        if self.options.contains(AuthRequiredOptions::WITH_SUDO_UNTIL) {
            columns.push(r#""sessions"."sudo_until""#);
        }

        let query = format!(
            r#"
            SELECT {}
            FROM "users" JOIN "sessions" ON "users"."id" = "sessions"."user_id"
            WHERE "sessions"."id" = $1
            "#,
            columns.join(", "),
        );
        let row = db
            .query_opt(&query, &[&session_id_hash])
            .await
            .map_err(InternalError::new)?
            .ok_or_else(|| {
                BadRequest::new(AuthError::NotLoggedIn).remove_cookie(&CONFIG.session.cookie)
            })?;

        if row.get::<_, DateTime<Utc>>("expires") < utc_now() {
            Err(BadRequest::new(AuthError::SessionExpired).remove_cookie(&CONFIG.session.cookie))?;
        }

        if row.get("disabled") {
            Err(
                BadRequest::new(AuthError::AccountDisabled)
                    .remove_cookie(&CONFIG.remember_token.cookie)
                    .remove_cookie(&CONFIG.session.cookie)
            )?;
        }

        if !self.options.contains(AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON) {
            if let Some(_) = row.get::<_, Option<PasswordChangeReason>>("password_change_reason") {
                Err(BadRequest::new(AuthError::PasswordChangeRequired))?;
            }
        }

        let mut user = CurrentUser::default();
        user.id = row.get("id");
        user.session_id_hash = session_id_hash;

        if self.options.contains(AuthRequiredOptions::WITH_USERNAME) {
            user.username = Some(row.get("username"));
        }
        if self
            .options
            .contains(AuthRequiredOptions::WITH_PASSWORD_HASH)
        {
            user.password_hash = Some(row.get("password_hash"));
        }
        if self.options.contains(AuthRequiredOptions::WITH_TOTP_STATUS) {
            user.totp_enabled = Some(row.get("totp_enabled"));
        }
        if self.options.contains(AuthRequiredOptions::WITH_PASSWORD_CHANGE_REASON) {
            user.password_change_reason = Some(row.get("password_change_reason"));
        }
        if self.options.contains(AuthRequiredOptions::WITH_ICON) {
            user.icon = Some(row.get("icon"));
        }
        if self.options.contains(AuthRequiredOptions::WITH_LOCALE) {
            user.locale = Some(row.get("locale"));
        }
        if self.options.contains(AuthRequiredOptions::WITH_SUDO_UNTIL) {
            user.sudo_until = Some(row.get("sudo_until"));
        }

        if self.options.contains(AuthRequiredOptions::WITH_PERMISSIONS) {
            let query = r#"
                SELECT "permissions"."permission" FROM "permissions" WHERE "user_id" = $1
            "#;
            user.permissions = Some(
                db.query(query, &[&user.id])
                    .await
                    .map_err(InternalError::new)?
                    .into_iter()
                    .map(|row| row.get("permission"))
                    .collect(),
            );
        }

        req.set_data(user);
        self.endpoint.call(req).await
    }
}

async fn csrf_error(error: CsrfError, req: &Request) -> PoemError {
    let mut res = BadRequest::new(error);

    if let Some(session_id) = req.cookie().get(&CONFIG.session.cookie) {
        let session_id = session_id.value_str();
        let db = match get_db(req).await {
            Ok(client) => client,
            Err(err) => return err.into(),
        };
        let query = r#"SELECT "csrf_token" FROM "sessions" WHERE "id" = $1"#;
        match db.query_opt(query, &[&session_id]).await {
            Ok(Some(row)) => return res.csrf(row.get("csrf_token")).into(),
            Ok(None) => res = res.remove_cookie(&CONFIG.session.cookie),
            Err(err) => return InternalError::new(err).into(),
        };
    }

    res.csrf(generate_token(CONFIG.csrf.token_length)).into()
}

pub async fn csrf<E: Endpoint>(next: E, req: Request) -> Result<E::Output> {
    let csrf_cookie = match req.cookie().get(&CONFIG.csrf.cookie) {
        Some(cookie) => cookie.value_str().to_owned(),
        None => return Err(csrf_error(CsrfError::MissingCookie, &req).await),
    };
    let csrf_cookie = SecStr::from(csrf_cookie);

    let csrf_header = match req.headers().get(&CONFIG.csrf.header) {
        Some(header) => header,
        None => return Err(csrf_error(CsrfError::MissingHeader, &req).await),
    };
    let csrf_header = match csrf_header.to_str() {
        Ok(header) => header,
        Err(_) => return Err(csrf_error(CsrfError::InvalidHeader, &req).await),
    };
    let csrf_header = SecStr::from(csrf_header);

    if csrf_cookie != csrf_header {
        Err(csrf_error(CsrfError::Mismatch, &req).await)
    } else {
        next.call(req).await
    }
}
