use bitflags::bitflags;
use chrono::{DateTime, Utc};
use poem::{async_trait, Endpoint, Middleware, Request, Result};
use secstr::SecStr;

use crate::{
    config::Config,
    db::{Language, PasswordChangeReason, Permission},
    error::{AuthError, CsrfError, ErrorData, InternalError},
    util::{
        clear_cookie, generate_token, get_db, get_session, hash, make_cookie, utc_now, Session,
        SessionError,
    },
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
    pub language: Option<Language>,
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
            .ok_or(AuthError::NotLoggedIn(None))?
            .value_str()
            .to_owned();
        let session_id_hash = hash(&session_id);

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
            columns.push(r#""users"."language""#);
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
            .map_err(InternalError::new)?;
        let Some(row) = row else {
            return Err(AuthError::NotLoggedIn(Some(ErrorData {
                cookies: vec![clear_cookie(&self.config.session.cookie, &self.config)],
                ..Default::default()
            })).into());
        };

        if row.get::<_, DateTime<Utc>>("expires") < utc_now() {
            return Err(AuthError::SessionExpired(Some(ErrorData {
                cookies: vec![clear_cookie(&self.config.session.cookie, &self.config)],
                ..Default::default()
            })).into());
        }

        if !row.get::<_, bool>("active") {
            return Err(AuthError::AccountDisabled(Some(ErrorData {
                cookies: vec![
                    clear_cookie(&self.config.remember_token.cookie, &self.config),
                    clear_cookie(&self.config.session.cookie, &self.config),
                ],
                ..Default::default()
            })).into());
        }

        if !self
            .options
            .contains(AuthRequiredOptions::ALLOW_PASSWORD_CHANGE_REASON)
        {
            if let Some(_) = row.get::<_, Option<PasswordChangeReason>>("password_change_reason") {
                return Err(AuthError::PasswordChangeRequired(None).into());
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
            user.language = Some(row.get("language"));
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

async fn csrf_error(
    error: CsrfError,
    req: &Request,
    config: &Config,
) -> Result<CsrfError, InternalError> {
    let (mut cookies, csrf_token) = match get_session(req, &config).await {
        Ok(Session { csrf_token }) => (
            vec![make_cookie(
                &config.csrf.cookie,
                &csrf_token,
                Some(config.csrf.cookie_lifetime),
                config,
            )],
            Some(csrf_token),
        ),
        Err(SessionError::NoCookie) => (vec![], None),
        Err(SessionError::ExpiredSession) | Err(SessionError::InvalidSession) => {
            (vec![clear_cookie(&config.session.cookie, config)], None)
        }
        Err(SessionError::InternalError(err)) => return Err(err),
    };
    let csrf_token = match csrf_token {
        Some(csrf_token) => csrf_token,
        None => {
            let csrf_token = generate_token(config.csrf.token_length);
            cookies.push(make_cookie(
                &config.csrf.cookie,
                &csrf_token,
                Some(config.csrf.cookie_lifetime),
                config,
            ));
            csrf_token
        },
    };

    Ok(match error {
        CsrfError::InvalidHeader(_) => CsrfError::InvalidHeader(Some(ErrorData {
            cookies,
            csrf_token: Some((config.csrf.response_field.clone(), csrf_token)),
            ..Default::default()
        })),
        CsrfError::MissingCookie(_) => CsrfError::MissingCookie(Some(ErrorData {
            cookies,
            csrf_token: Some((config.csrf.response_field.clone(), csrf_token)),
            ..Default::default()
        })),
        CsrfError::MissingHeader(_) => CsrfError::MissingHeader(Some(ErrorData {
            cookies,
            csrf_token: Some((config.csrf.response_field.clone(), csrf_token)),
            ..Default::default()
        })),
        CsrfError::Mismatch(_) => CsrfError::Mismatch(Some(ErrorData {
            cookies,
            csrf_token: Some((config.csrf.response_field.clone(), csrf_token)),
            ..Default::default()
        })),
    })
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
            None => {
                return Err(
                    match csrf_error(CsrfError::MissingCookie(None), &req, &self.config).await {
                        Ok(err) => err.into(),
                        Err(err) => err.into(),
                    },
                );
            }
        };
        let csrf_cookie = SecStr::from(csrf_cookie);

        let csrf_header = match req.headers().get(&self.config.csrf.header) {
            Some(header) => header,
            None => {
                return Err(
                    match csrf_error(CsrfError::MissingHeader(None), &req, &self.config).await {
                        Ok(err) => err.into(),
                        Err(err) => err.into(),
                    },
                );
            }
        };
        let csrf_header = match csrf_header.to_str() {
            Ok(header) => header,
            Err(_) => {
                return Err(
                    match csrf_error(CsrfError::InvalidHeader(None), &req, &self.config).await {
                        Ok(err) => err.into(),
                        Err(err) => err.into(),
                    },
                );
            }
        };
        let csrf_header = SecStr::from(csrf_header);
        if csrf_cookie != csrf_header {
            return Err(
                match csrf_error(CsrfError::Mismatch(None), &req, &self.config).await {
                    Ok(err) => err.into(),
                    Err(err) => err.into(),
                },
            );
        }
        self.endpoint.call(req).await
    }
}
