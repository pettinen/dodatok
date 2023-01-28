use std::fmt;

use aes_gcm_siv::{Aes256GcmSiv, KeyInit};
use argon2::Argon2;
use chrono::Duration;
use deadpool_postgres::Config as DbConfig;
use poem::{http::HeaderValue, web::cookie::SameSite};
use serde::Deserialize;
use toml;

use crate::util::{TotpAlgorithm, make_argon2};

#[derive(Deserialize)]
pub struct ClientConfigInput {
    pub origin: String,
}

#[derive(Clone)]
pub struct ClientConfig {
    pub origin: HeaderValue,
}

#[derive(Deserialize)]
pub struct CookieConfigInput {
    pub path: String,
    pub same_site: String,
    pub secure: bool,
}

#[derive(Clone)]
pub struct CookieConfig {
    pub path: String,
    pub same_site: SameSite,
    pub secure: bool,
}

#[derive(Deserialize)]
pub struct CsrfConfigInput {
    pub cookie: String,
    pub cookie_lifetime: u32,
    pub header: String,
    pub response_field: String,
    pub token_bits: u16,
}

#[derive(Clone)]
pub struct CsrfConfig {
    pub cookie: String,
    pub cookie_lifetime: Duration,
    pub header: String,
    pub response_field: String,
    pub token_length: u16,
}

#[derive(Deserialize)]
pub struct DbConfigInput {
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub application_name: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
}

impl DbConfigInput {
    fn as_db_config(self: &DbConfigInput) -> DbConfig {
        DbConfig {
            user: Some(self.user.clone()),
            password: Some(self.password.clone()),
            dbname: Some(self.dbname.clone()),
            application_name: self.application_name.clone(),
            host: self.host.clone(),
            port: self.port,
            ..DbConfig::default()
        }
    }
}

#[derive(Default, Deserialize)]
pub struct DevConfigInput {
    pub debug: bool,
    pub init_db: Option<DbConfigInput>,
    pub testing: bool,
}

#[derive(Clone, Default)]
pub struct DevConfig {
    pub debug: bool,
    pub init_db: Option<DbConfig>,
    pub testing: bool,
}

#[derive(Deserialize)]
pub struct RedisConfigInput {
    pub url: String,
    pub key_separator: String,
}

#[derive(Clone)]
pub struct RedisConfig {
    pub url: String,
    pub key_separator: String,
}

#[derive(Deserialize)]
pub struct RememberTokenConfigInput {
    pub cookie: String,
    pub cookie_lifetime: u32,
    pub id_bits: u16,
    pub secret_bits: u16,
    pub separator: String,
}

#[derive(Clone)]
pub struct RememberTokenConfig {
    pub cookie: String,
    pub cookie_lifetime: Duration,
    pub id_length: u16,
    pub secret_length: u16,
    pub separator: String,
}

#[derive(Deserialize)]
pub struct SecurityConfigInput {
    pub aes_key: String,
    pub argon2_time_cost: u32,
    pub argon2_memory_cost: u32,
    pub argon2_parallelism: u32,
    pub password_salt_bits: u16,
}

#[derive(Clone)]
pub struct SecurityConfig {
    pub password_salt_bytes: usize,
}

#[derive(Deserialize)]
pub struct SessionConfigInput {
    pub cookie: String,
    pub id_bits: u16,
    pub lifetime: u32,
    pub sudo_lifetime: u32,
}

#[derive(Clone)]
pub struct SessionConfig {
    pub cookie: String,
    pub id_length: u16,
    pub lifetime: Duration,
    pub sudo_lifetime: Duration,
}

#[derive(Deserialize)]
pub struct TotpConfigInput {
    pub algorithm: TotpAlgorithm,
    pub digits: u8,
    pub key_length: u16,
    pub time_step: u16,
    pub time_window: u8,
}

#[derive(Clone)]
pub struct TotpConfig {
    pub algorithm: TotpAlgorithm,
    pub digits: u32,
    pub key_length: usize,
    pub time_step: u16,
    pub time_window: u8,
}

#[derive(Deserialize)]
pub struct UserConfigInput {
    pub id_bits: u16,
    pub icon_id_bits: u16,
    pub username_min_length: u8,
    pub username_max_length: u8,
    pub password_min_length: u8,
    pub password_max_length: u16,
}

#[derive(Clone)]
pub struct UserConfig {
    pub id_length: u16,
    pub icon_id_length: u16,
    pub username_min_length: u8,
    pub username_max_length: u8,
    pub password_min_length: u8,
    pub password_max_length: u16,
}

#[derive(Deserialize)]
pub struct WebSocketConfigInput {
    pub channel_capacity: u16,
    pub connection_id_bits: u16,
    pub token_bits: u16,
    pub token_lifetime: u16,
}

#[derive(Clone)]
pub struct WebSocketConfig {
    pub channel_capacity: usize,
    pub connection_id_length: u16,
    pub token_length: u16,
    pub token_lifetime: usize,
}

#[derive(Deserialize)]
pub struct ConfigInput {
    pub client: ClientConfigInput,
    pub cookie: CookieConfigInput,
    pub csrf: CsrfConfigInput,
    pub db: DbConfigInput,
    pub dev: Option<DevConfigInput>,
    pub redis: RedisConfigInput,
    pub remember_token: RememberTokenConfigInput,
    pub security: SecurityConfigInput,
    pub session: SessionConfigInput,
    pub totp: TotpConfigInput,
    pub user: UserConfigInput,
    pub websocket: WebSocketConfigInput,
}

#[derive(Clone)]
pub struct Config {
    pub aes: Aes256GcmSiv,
    pub argon2: Argon2<'static>,
    pub client: ClientConfig,
    pub cookie: CookieConfig,
    pub csrf: CsrfConfig,
    pub db: DbConfig,
    pub dev: DevConfig,
    pub redis: RedisConfig,
    pub remember_token: RememberTokenConfig,
    pub security: SecurityConfig,
    pub session: SessionConfig,
    pub totp: TotpConfig,
    pub user: UserConfig,
    pub websocket: WebSocketConfig,
}

fn bits_to_bytes(bits: u16, name: &str) -> usize {
    if bits % 8 != 0 {
        panic!("{name} must be a multiple of 8");
    }
    (bits / 8).into()
}

fn alphanum_token_length(bits: u16) -> u16 {
    const ALPHABET_SIZE: f64 = 62.0;
    let result = (Into::<f64>::into(bits) / ALPHABET_SIZE.log2()).ceil();
    assert!(result <= u16::MAX.into());
    result as u16
}

impl Config {
    pub fn new(input: &ConfigInput) -> Self {
        Self {
            aes: {
                let aes_key = hex::decode(input.security.aes_key.clone()).unwrap();
                Aes256GcmSiv::new_from_slice(&aes_key).unwrap()
            },
            argon2: make_argon2(
                input.security.argon2_memory_cost,
                input.security.argon2_time_cost,
                input.security.argon2_parallelism,
            ).unwrap(),
            client: ClientConfig {
                origin: HeaderValue::from_str(&input.client.origin).unwrap(),
            },
            cookie: CookieConfig {
                path: input.cookie.path.clone(),
                same_site: match input.cookie.same_site.as_str() {
                    "None" => SameSite::None,
                    "Lax" => SameSite::Lax,
                    "Strict" => SameSite::Strict,
                    _ => panic!("invalid config value for cookie.same_site"),
                },
                secure: input.cookie.secure,
            },
            csrf: CsrfConfig {
                cookie: input.csrf.cookie.clone(),
                cookie_lifetime: Duration::seconds(input.csrf.cookie_lifetime.into()),
                header: input.csrf.header.clone(),
                response_field: input.csrf.response_field.clone(),
                token_length: alphanum_token_length(input.csrf.token_bits),
            },
            db: input.db.as_db_config(),
            dev: if let Some(dev_config) = &input.dev {
                DevConfig {
                    debug: dev_config.debug,
                    init_db: dev_config
                        .init_db
                        .as_ref()
                        .map(|config| config.as_db_config()),
                    testing: dev_config.testing,
                }
            } else {
                DevConfig::default()
            },
            redis: RedisConfig {
                url: input.redis.url.clone(),
                key_separator: input.redis.key_separator.clone(),
            },
            remember_token: RememberTokenConfig {
                cookie: input.remember_token.cookie.clone(),
                cookie_lifetime: Duration::seconds(input.remember_token.cookie_lifetime.into()),
                id_length: alphanum_token_length(input.remember_token.id_bits),
                secret_length: alphanum_token_length(input.remember_token.secret_bits),
                separator: input.remember_token.separator.clone(),
            },
            security: SecurityConfig {
                password_salt_bytes: bits_to_bytes(
                    input.security.password_salt_bits,
                    "security.password_salt_bits",
                ),
            },
            session: SessionConfig {
                cookie: input.session.cookie.clone(),
                id_length: alphanum_token_length(input.session.id_bits),
                lifetime: Duration::seconds(input.session.lifetime.into()),
                sudo_lifetime: Duration::seconds(input.session.sudo_lifetime.into()),
            },
            totp: TotpConfig {
                algorithm: input.totp.algorithm.clone(),
                digits: input.totp.digits.into(),
                key_length: input.totp.key_length.into(),
                time_step: {
                    if input.totp.time_step == 0 {
                        panic!("totp.time_step must be greater than 0");
                    }
                    input.totp.time_step
                },
                time_window: input.totp.time_window,
            },
            user: UserConfig {
                id_length: alphanum_token_length(input.user.id_bits),
                icon_id_length: alphanum_token_length(input.user.icon_id_bits),
                username_min_length: input.user.username_min_length,
                username_max_length: input.user.username_max_length,
                password_min_length: input.user.password_min_length,
                password_max_length: input.user.password_max_length,
            },
            websocket: WebSocketConfig {
                channel_capacity: input.websocket.channel_capacity.into(),
                connection_id_length: alphanum_token_length(input.websocket.connection_id_bits),
                token_length: alphanum_token_length(input.websocket.token_bits),
                token_lifetime: input.websocket.token_lifetime.into(),
            },
        }
    }

    pub fn from_file(path: &str) -> Self {
        let input = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        Self::new(&input)
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Config")
    }
}
