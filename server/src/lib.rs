use std::{collections::HashMap, sync::Arc};

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
    let db = config.db.create_pool(None, NoTls).unwrap();
    let redis = RedisClient::open(config.redis.url.clone()).unwrap();
    let account_rooms: AccountRooms = Arc::new(Mutex::new(HashMap::new()));
    let account_connections: AccountConnections = Arc::new(Mutex::new(HashMap::new()));

    if config.dev.init_db.is_some() {
        db::init_db(true, &config).await;
        if !config.dev.testing {
            db::populate_db(&config).await;
        }
    }

    let mut routes = Route::new()
        .nest("/account", routes::account::routes(&config))
        .nest("/auth", routes::auth::routes(&config))
        .nest("/users", routes::users::routes(&config));
    if config.dev.debug {
        routes = routes.nest("/test", routes::test::routes(&config))
    };
    routes
        .catch_all_error(error_handler)
        .with(CookieJarManager::new())
        .data(config)
        .data(db)
        .data(redis)
        .data(account_rooms)
        .data(account_connections)
}
