use aes_gcm_siv::{Aes256GcmSiv, aead::NewAead};
use chrono::Duration;
use deadpool_postgres::{tokio_postgres::NoTls, ClientWrapper, Config, Object, Pool};
use futures::executor;
use rand::{distributions::Standard, thread_rng, Rng};

use dodatok::{
    db::{self, Locale},
    util::{blake2, encrypt, generate_token, generate_totp_key, hash_encrypt_password, utc_now},
    CONFIG,
};

pub struct TestUser {
    pub id: String,
    pub username: String,
    pub password: String,
    pub totp_key: Option<Vec<u8>>,
    pub locale: Locale,
}

pub struct TestDb {
    dbname: String,
    pool: Pool,
}

impl TestDb {
    pub async fn new(dbname: &str) -> Self {
        db::init_db(true, Some(dbname)).await;
        let mut config = Config::new();
        config.application_name = Some(dbname.to_owned());
        config.dbname = Some(dbname.to_owned());
        let pool = config.create_pool(None, NoTls).unwrap();
        Self {
            dbname: dbname.to_owned(),
            pool,
        }
    }

    pub async fn get(&self) -> ClientWrapper {
        Object::take(self.pool.get().await.unwrap())
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let db = executor::block_on(self.get());
        executor::block_on(db.execute(&format!("DROP DATABASE {}", self.dbname), &[])).unwrap();
    }
}

pub async fn add_session(user: &TestUser, expired: bool) -> (String, String) {
    let db_pool = CONFIG.db.create_pool(None, NoTls).unwrap();
    let db = db_pool.get().await.unwrap();

    let session_id = generate_token(CONFIG.session.id_length);
    let csrf_token = generate_token(CONFIG.csrf.token_length);
    let session_expires = if expired {
        utc_now() - Duration::seconds(1)
    } else {
        utc_now() + CONFIG.session.lifetime
    };

    db.execute(
        r#"
        INSERT INTO "sessions"("id", "user_id", "csrf_token", "expires")
        VALUES ($1, $2, $3, $4)
        "#,
        &[&blake2(&session_id), &user.id, &csrf_token, &session_expires],
    ).await.unwrap();
    (session_id, csrf_token)
}

pub async fn add_user(username_char: char, totp: bool) -> TestUser {
    let db_pool = CONFIG.db.create_pool(None, NoTls).unwrap();
    let db = db_pool.get().await.unwrap();
    let aes = Aes256GcmSiv::new_from_slice(&CONFIG.security.aes_key).unwrap();

    let id = generate_token(CONFIG.user.id_length);
    let username = (0..CONFIG.user.username_min_length).map(|_| username_char).collect::<String>();
    let password: Vec<char> = thread_rng()
        .sample_iter(Standard)
        .take(CONFIG.user.password_min_length as usize)
        .collect();
    let password = String::from_iter(password);
    let password_hash = hash_encrypt_password(&password, &aes).unwrap();
    let locale = Locale::en_US;

    db.execute(
        r#"INSERT INTO "users"("id", "username", "password", "locale") VALUES ($1, $2, $3, $4)"#,
        &[&id, &username, &password_hash, &locale],
    ).await.unwrap();

    let totp_key = if totp {
        let totp_key = generate_totp_key();
        let aes = Aes256GcmSiv::new_from_slice(&CONFIG.security.aes_key).unwrap();
        let encrypted_totp_key = encrypt(&totp_key, &aes, &mut thread_rng()).unwrap();
        db.execute(
            r#"UPDATE "users" SET "totp_key" = $1 WHERE "id" = $2"#,
            &[&encrypted_totp_key, &id],
        ).await.unwrap();
        Some(totp_key)
    } else {
        None
    };

    tracing::info!("added user {}:{}", username, password);
    TestUser {
        id,
        username,
        password,
        totp_key,
        locale,
    }
}
