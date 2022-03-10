use chrono::Duration;
use deadpool_postgres::Config as DbConfig;
use poem::http::HeaderValue;
use serde::Deserialize;
use toml;

use crate::util::TotpAlgorithm;

#[derive(Deserialize)]
struct ClientConfigInput {
    origin: String,
}

pub struct ClientConfig {
    pub origin: HeaderValue,
}

#[derive(Deserialize)]
struct CookieConfigInput {
    path: String,
    secure: bool,
}

pub struct CookieConfig {
    pub path: String,
    pub secure: bool,
}

#[derive(Deserialize)]
struct CsrfConfigInput {
    cookie: String,
    header: String,
    response_field: String,
    token_bits: u16,
}

pub struct CsrfConfig {
    pub cookie: String,
    pub header: String,
    pub response_field: String,
    pub token_length: u16,
}

#[derive(Deserialize)]
struct DbConfigInput {
    application_name: Option<String>,
    dbname: Option<String>,
}

#[derive(Deserialize)]
struct RedisConfigInput {
    url: String,
    key_separator: String,
}

pub struct RedisConfig {
    pub url: String,
    pub key_separator: String,
}

#[derive(Deserialize)]
struct RememberTokenConfigInput {
    cookie: String,
    id_bits: u16,
    secret_bits: u16,
    separator: String,
}

pub struct RememberTokenConfig {
    pub cookie: String,
    pub id_length: u16,
    pub secret_length: u16,
    pub separator: String,
}

#[derive(Deserialize)]
struct SecurityConfigInput {
    aes_key: String,
    password_salt_bits: u16,
}

pub struct SecurityConfig {
    pub aes_key: Vec<u8>,
    pub password_salt_bytes: usize,
}

#[derive(Deserialize)]
struct SessionConfigInput {
    cookie: String,
    id_bits: u16,
    lifetime: u64,

    sudo_lifetime: u64,
}

pub struct SessionConfig {
    pub cookie: String,
    pub id_length: u16,
    pub lifetime: Duration,
    pub sudo_lifetime: Duration,
}

#[derive(Deserialize)]
struct TotpConfigInput {
    algorithm: TotpAlgorithm,
    digits: u8,
    time_step: u16,
    time_window: u8,
}

pub struct TotpConfig {
    pub algorithm: TotpAlgorithm,
    pub digits: u32,
    pub time_step: u64,
    pub time_window: u8,
}

#[derive(Deserialize)]
struct WebSocketConfigInput {
    channel_capacity: u16,
    connection_id_bits: u16,
    token_bits: u16,
    token_lifetime: u32,
}

pub struct WebSocketConfig {
    pub channel_capacity: usize,
    pub connection_id_length: u16,
    pub token_length: u16,
    pub token_lifetime: usize,
}

#[derive(Deserialize)]
struct ConfigInput {
    client: ClientConfigInput,
    cookie: CookieConfigInput,
    csrf: CsrfConfigInput,
    db: DbConfigInput,
    redis: RedisConfigInput,
    remember_token: RememberTokenConfigInput,
    security: SecurityConfigInput,
    session: SessionConfigInput,
    totp: TotpConfigInput,
    websocket: WebSocketConfigInput,
}

pub struct Config {
    pub client: ClientConfig,
    pub cookie: CookieConfig,
    pub csrf: CsrfConfig,
    pub db: DbConfig,
    pub redis: RedisConfig,
    pub remember_token: RememberTokenConfig,
    pub security: SecurityConfig,
    pub session: SessionConfig,
    pub totp: TotpConfig,
    pub websocket: WebSocketConfig,
}

fn alphanum_token_length(bits: u16) -> u16 {
    const ALPHABET_SIZE: f64 = 62.0;
    (bits as f64 / ALPHABET_SIZE.log2()).ceil() as u16
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let input: ConfigInput = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

        Config {
            client: ClientConfig {
                origin: HeaderValue::from_str(&input.client.origin).unwrap(),
            },
            cookie: CookieConfig {
                path: input.cookie.path,
                secure: input.cookie.secure,
            },
            csrf: CsrfConfig {
                cookie: input.csrf.cookie,
                header: input.csrf.header,
                response_field: input.csrf.response_field,
                token_length: alphanum_token_length(input.csrf.token_bits),
            },
            db: {
                let mut db_config = DbConfig::new();
                db_config.application_name = input.db.application_name;
                db_config.dbname = input.db.dbname;
                db_config
            },
            redis: RedisConfig {
                url: input.redis.url,
                key_separator: input.redis.key_separator,
            },
            remember_token: RememberTokenConfig {
                cookie: input.remember_token.cookie,
                id_length: alphanum_token_length(input.remember_token.id_bits),
                secret_length: alphanum_token_length(input.remember_token.secret_bits),
                separator: input.remember_token.separator,
            },
            security: SecurityConfig {
                aes_key: hex::decode(input.security.aes_key).unwrap(),
                password_salt_bytes: (input.security.password_salt_bits as f64 / 8.0).ceil() as usize,
            },
            session: SessionConfig {
                cookie: input.session.cookie,
                id_length: alphanum_token_length(input.session.id_bits),
                lifetime: Duration::seconds(input.session.lifetime as i64),
                sudo_lifetime: Duration::seconds(input.session.sudo_lifetime as i64),
            },
            totp: TotpConfig {
                algorithm: input.totp.algorithm,
                digits: input.totp.digits as u32,
                time_step: input.totp.time_step as u64,
                time_window: input.totp.time_window
            },
            websocket: WebSocketConfig {
                channel_capacity: input.websocket.channel_capacity as usize,
                connection_id_length: alphanum_token_length(input.websocket.connection_id_bits),
                token_length: alphanum_token_length(input.websocket.token_bits),
                token_lifetime: input.websocket.token_lifetime as usize,
            },
        }
    }
}
