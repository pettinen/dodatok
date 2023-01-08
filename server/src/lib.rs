use std::{collections::HashMap, sync::Arc};

use aes_gcm_siv::{aead::NewAead, Aes256GcmSiv};
use deadpool_postgres::tokio_postgres::NoTls;
use poem::{middleware::CookieJarManager, Endpoint, EndpointExt, Response, Route};
use redis::Client as RedisClient;
use tokio::sync::Mutex;

pub mod config;
pub mod db;
mod error;
mod middleware;
mod routes;
pub mod util;
mod websocket;

use config::Config;
use error::error_handler;
use websocket::{AccountConnections, AccountRooms};

pub async fn create_app(config: Config) -> impl Endpoint<Output = Response> {
    let aes = Aes256GcmSiv::new_from_slice(&config.security.aes_key).unwrap();
    let db = config.db.create_pool(None, NoTls).unwrap();
    let redis = RedisClient::open(config.redis.url.clone()).unwrap();
    let account_rooms: AccountRooms = Arc::new(Mutex::new(HashMap::new()));
    let account_connections: AccountConnections = Arc::new(Mutex::new(HashMap::new()));
    let config_data = Arc::new(Mutex::new(config.clone()));

    if config.debug {
        db::init_db(true, &config).await;
        if !config.testing {
            db::populate_db(&config, &aes).await;
        }
    }

    Route::new()
        .nest("/account", routes::account::routes(config.clone()))
        .nest("/auth", routes::auth::routes(config.clone()))
        .nest("/users", routes::users::routes(config.clone()))
        .catch_all_error(error_handler)
        .with(CookieJarManager::new())
        .data(aes)
        .data(config_data)
        .data(db)
        .data(redis)
        .data(account_rooms)
        .data(account_connections)
        .data(config.clone())
}
