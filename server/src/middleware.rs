use bitflags::bitflags;
use chrono::{DateTime, Utc};
use poem::{async_trait, error::Error as PoemError, Endpoint, Middleware, Request, Result};
use secstr::SecStr;

use crate::{
    config::Config,
    db::{Locale, PasswordChangeReason, Permission},
    error::{AuthError, BadRequest, CsrfError, InternalError},
    util::{blake2, generate_token, get_db, get_session, utc_now, Session, SessionError},
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
pub struct AuthRequired {
    config: Config,
    options: AuthRequiredOptions,
}

impl AuthRequired {
    pub fn new(options: AuthRequiredOptions, config: Config) -> Self {
        Self { config, options }
    }

    pub fn defaults(config: Config) -> Self {
        Self {
            config,
            options: AuthRequiredOptions::default(),
        }
    }
}

impl<E: Endpoint> Middleware<E> for AuthRequired {
    type Output = AuthRequiredImpl<E>;

    fn transform(&self, endpoint: E) -> Self::Output {
        AuthRequiredImpl {
            config: self.config.clone(),
            endpoint,
            options: self.options,
        }
    }
}

pub struct AuthRequiredImpl<E> {
    config: Config,
    endpoint: E,
    options: AuthRequiredOptions,
}

#[async_trait]
impl<E: Endpoint> Endpoint for AuthRequiredImpl<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let session_id = req
            .cookie()
            .get(&self.config.session.cookie)
            .ok_or_else(|| BadRequest::new(AuthError::NotLoggedIn, &self.config))?
            .value_str()
            .to_owned();
        let session_id_hash = blake2(&session_id);

        let db = get_db(&req).await?;
        let mut columns = vec![
            r#""users"."id""#,
            r#""users"."active""#,
            r#""sessions"."expires""#,
        ];

        if self.options.contains(AuthRequiredOptions::WITH_USERNAME) {
            columns.push(r#""users"."username""#);
        }
        if self
            .options
            .contains(AuthRequiredOptions::WITH_PASSWORD_HASH)
        {
            columns.push(r#""users"."password""#);
        }
        if self.options.contains(AuthRequiredOptions::WITH_TOTP_STATUS) {
            columns.push(r#""users"."totp_key" IS NOT NULL AS "totp_enabled""#);
        }
        if self
            .options
            .contains(AuthRequiredOptions::WITH_PASSWORD_CHANGE_REASON)
            || !self
                .options
                .contains(AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON)
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
                BadRequest::new(AuthError::NotLoggedIn, &self.config)
                    .remove_cookie(&self.config.session.cookie)
            })?;

        if row.get::<_, DateTime<Utc>>("expires") < utc_now() {
            Err(BadRequest::new(AuthError::SessionExpired, &self.config)
                .remove_cookie(&self.config.session.cookie))?;
        }

        if !row.get::<_, bool>("active") {
            Err(BadRequest::new(AuthError::AccountDisabled, &self.config)
                .remove_cookie(&self.config.remember_token.cookie)
                .remove_cookie(&self.config.session.cookie))?;
        }

        if !self
            .options
            .contains(AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON)
        {
            if let Some(_) = row.get::<_, Option<PasswordChangeReason>>("password_change_reason") {
                Err(BadRequest::new(
                    AuthError::PasswordChangeRequired,
                    &self.config,
                ))?;
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
            user.password_hash = Some(row.get("password"));
        }
        if self.options.contains(AuthRequiredOptions::WITH_TOTP_STATUS) {
            user.totp_enabled = Some(row.get("totp_enabled"));
        }
        if self
            .options
            .contains(AuthRequiredOptions::WITH_PASSWORD_CHANGE_REASON)
        {
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

async fn csrf_error(error: CsrfError, req: &Request, config: &Config) -> PoemError {
    let mut res = BadRequest::new(error, config);
    match get_session(req, &config).await {
        Ok(Session { csrf_token }) => return res.csrf(csrf_token).into(),
        Err(SessionError::NoCookie) => {}
        Err(SessionError::ExpiredSession) | Err(SessionError::InvalidSession) => {
            res = res.remove_cookie(&config.session.cookie);
        }
        Err(SessionError::InternalError(err)) => return err.into(),
    }
    res.csrf(generate_token(config.csrf.token_length)).into()
}

pub struct Csrf {
    config: Config,
}

impl Csrf {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl<E: Endpoint> Middleware<E> for Csrf {
    type Output = CsrfImpl<E>;

    fn transform(&self, endpoint: E) -> Self::Output {
        CsrfImpl {
            config: self.config.clone(),
            endpoint,
        }
    }
}

pub struct CsrfImpl<E> {
    config: Config,
    endpoint: E,
}

#[async_trait]
impl<E: Endpoint> Endpoint for CsrfImpl<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let csrf_cookie = match req.cookie().get(&self.config.csrf.cookie) {
            Some(cookie) => cookie.value_str().to_owned(),
            None => return Err(csrf_error(CsrfError::MissingCookie, &req, &self.config).await),
        };
        let csrf_cookie = SecStr::from(csrf_cookie);

        let csrf_header = match req.headers().get(&self.config.csrf.header) {
            Some(header) => header,
            None => return Err(csrf_error(CsrfError::MissingHeader, &req, &self.config).await),
        };
        let csrf_header = match csrf_header.to_str() {
            Ok(header) => header,
            Err(_) => return Err(csrf_error(CsrfError::InvalidHeader, &req, &self.config).await),
        };
        let csrf_header = SecStr::from(csrf_header);
        if csrf_cookie != csrf_header {
            return Err(csrf_error(CsrfError::Mismatch, &req, &self.config).await);
        }
        self.endpoint.call(req).await
    }
}
