use chrono::Duration;
use deadpool_postgres::{tokio_postgres::NoTls, ClientWrapper, Object};
use poem::{Endpoint, Response};
use rand::{distributions::Standard, thread_rng, Rng};

use dodatok::{
    config::{Config, ConfigInput},
    db::Language,
    util::{encrypt, generate_token, generate_totp_key, hash, hash_encrypt_password, utc_now},
};

pub struct TestUser {
    pub id: String,
    pub username: String,
    pub password: String,
    pub totp_key: Option<String>,
    pub language: Language,
}

pub async fn init(test_name: &str) -> (impl Endpoint<Output = Response>, ClientWrapper, Config) {
    let mut config_data: ConfigInput =
        toml::from_str(&std::fs::read_to_string("config.test.toml").unwrap()).unwrap();
    config_data.db.application_name = Some(test_name.to_owned());
    config_data.db.dbname = test_name.to_owned();
    config_data.db.user = test_name.to_owned();
    let init_db_config = config_data.dev.as_mut().unwrap().init_db.as_mut().unwrap();
    init_db_config.application_name = Some(test_name.to_owned());

    let config = Config::new(&config_data);
    let endpoint = dodatok::create_app(config.clone()).await;

    let pool = config.db.create_pool(None, NoTls).unwrap();
    let db = Object::take(pool.get().await.unwrap());
    (endpoint, db, config)
}

pub async fn add_session(user: &TestUser, expired: bool, config: &Config) -> (String, String) {
    let db_pool = config.db.create_pool(None, NoTls).unwrap();
    let db = db_pool.get().await.unwrap();

    let session_id = generate_token(config.session.id_length);
    let csrf_token = generate_token(config.csrf.token_length);
    let session_expires = if expired {
        utc_now() - Duration::seconds(1)
    } else {
        utc_now() + config.session.lifetime
    };

    db.execute(
        r#"
        INSERT INTO "sessions"("id", "user_id", "csrf_token", "expires")
        VALUES ($1, $2, $3, $4)
        "#,
        &[&hash(&session_id), &user.id, &csrf_token, &session_expires],
    )
    .await
    .unwrap();
    (session_id, csrf_token)
}

pub async fn add_user(username_char: char, totp: bool, config: &Config) -> TestUser {
    let db_pool = config.db.create_pool(None, NoTls).unwrap();
    let db = db_pool.get().await.unwrap();

    let id = generate_token(config.user.id_length);
    let username = (0..config.user.username_min_length)
        .map(|_| username_char)
        .collect::<String>();
    let password: Vec<char> = thread_rng()
        .sample_iter(Standard)
        .take(config.user.password_min_length.into())
        .collect();
    let password = String::from_iter(password);
    let password_hash = hash_encrypt_password(&password, config).unwrap();
    let language = Language::en_US;

    db.execute(
        r#"INSERT INTO "users"("id", "username", "password", "language") VALUES ($1, $2, $3, $4)"#,
        &[&id, &username, &password_hash, &language],
    )
    .await
    .unwrap();

    let totp_key = if totp {
        let totp_key = generate_totp_key(config);
        let encrypted_totp_key = encrypt(&totp_key, &config, &mut thread_rng()).unwrap();
        db.execute(
            r#"UPDATE "users" SET "totp_key" = $1 WHERE "id" = $2"#,
            &[&encrypted_totp_key, &id],
        )
        .await
        .unwrap();
        Some(std::str::from_utf8(&totp_key).unwrap().to_owned())
    } else {
        None
    };

    tracing::info!("added user {}:{}", username, password);
    TestUser {
        id,
        username,
        password,
        totp_key,
        language,
    }
}
