use std::{collections::HashMap, sync::Arc};

use aes_gcm_siv::{Aes256GcmSiv, aead::NewAead};
use deadpool_postgres::tokio_postgres::NoTls;
use lazy_static::lazy_static;
use poem::{listener::TcpListener, middleware::CookieJarManager, EndpointExt, Route, Server};
use redis::Client as RedisClient;
use tokio::sync::Mutex as AsyncMutex;

use config::Config;
use error::error_handler;
use websocket::{AccountRooms, AccountConnections};

mod config;
mod db;
mod error;
mod middleware;
mod routes;
mod util;
mod websocket;

lazy_static! {
    static ref CONFIG: Config = Config::from_file("config.toml");
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let aes = Aes256GcmSiv::new_from_slice(&CONFIG.security.aes_key).unwrap();
    let db = CONFIG.db.create_pool(None, NoTls).unwrap();
    let redis = RedisClient::open(CONFIG.redis.url.clone()).unwrap();
    let account_rooms: AccountRooms = Arc::new(AsyncMutex::new(HashMap::new()));
    let account_connections: AccountConnections = Arc::new(AsyncMutex::new(HashMap::new()));

    let app = Route::new()
        .nest("/account", routes::account::routes())
        .nest("/auth", routes::auth::routes())
        .nest("/users", routes::users::routes())
        .catch_all_error(error_handler)
        .with(CookieJarManager::new())
        .data(aes)
        .data(db)
        .data(redis)
        .data(account_rooms)
        .data(account_connections);

    Server::new(TcpListener::bind("0.0.0.0:5000"))
        .run(app)
        .await
}
