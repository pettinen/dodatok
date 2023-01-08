use aes_gcm_siv::{Aes256GcmSiv, aead::NewAead};
use deadpool_postgres::{tokio_postgres::NoTls, Config};
use postgres_types::{FromSql, ToSql};
use serde::Serialize;

use crate::{
    util::{blake2, encrypt, generate_totp_key, hash_encrypt_password},
    CONFIG,
};
use macros::sql_enum;

#[allow(non_camel_case_types)]
#[sql_enum]
pub enum Locale {
    #[name("en-US")]
    en_US,
    #[name("fi-FI")]
    fi_FI,
}

#[sql_enum]
pub enum PasswordChangeReason {
    SessionCompromise,
}

#[derive(PartialEq)]
#[sql_enum]
pub enum Permission {
    ViewUser,
    EditUser,
    DeleteUser,
    IgnoreRateLimits,
}

fn enum_variants(variants: Vec<String>) -> String {
    variants.into_iter().map(|name| format!("'{}'", name)).collect::<Vec<_>>().join(", ")
}

pub async fn drop_db(name: &str) {
    let mut config = Config::new();
    config.application_name = Some(name.to_owned());
    config.dbname = Some(name.to_owned());
    let pool = config.create_pool(None, NoTls).unwrap();
    let db = pool.get().await.unwrap();
    db.execute(&format!("DROP DATABASE {}", name), &[]).await.unwrap();
}

pub async fn init_db(drop_existing: bool, name: Option<&str>) {
    let db_pool = if let Some(name) = name {
        let mut default_config = Config::new();
        default_config.application_name = Some(name.to_owned());
        default_config.dbname = Some("postgres".to_owned());
        let default_pool = default_config.create_pool(None, NoTls).unwrap();
        let default_db = default_pool.get().await.unwrap();
        default_db.execute(&format!("CREATE DATABASE {}", name), &[]).await.unwrap();

        let mut config = Config::new();
        config.application_name = Some(name.to_owned());
        config.dbname = Some(name.to_owned());
        config.create_pool(None, NoTls).unwrap()
    } else {
        CONFIG.db.create_pool(None, NoTls).unwrap()
    };
    let db = db_pool.get().await.unwrap();
    let aes = Aes256GcmSiv::new_from_slice(&CONFIG.security.aes_key).unwrap();

    let password_hash_length = hash_encrypt_password("a", &aes).unwrap().len();
    let totp_key_length = encrypt(&generate_totp_key(), &aes, &mut rand::thread_rng()).unwrap().len();

    let locales = enum_variants(Locale::variants());
    let password_change_reasons = enum_variants(PasswordChangeReason::variants());
    let permissions = enum_variants(Permission::variants());

    if drop_existing {
        db.batch_execute(
            r#"
            DROP TABLE IF EXISTS
                "sessions",
                "remember_tokens",
                "new_totp_keys",
                "permissions",
                "users";
            DROP TYPE IF EXISTS "locale", "password_change_reason", "permission";
            "#,
        ).await.unwrap();
    }

    db.batch_execute(
        &format!(
            r#"
            CREATE TYPE "locale" AS ENUM({locales});
            CREATE TYPE "password_change_reason" AS ENUM({password_change_reasons});
            CREATE TABLE "users" (
                "id" text PRIMARY KEY CHECK (length("id") = {user_id_length}),
                "active" boolean NOT NULL DEFAULT true,
                "username" text NOT NULL CHECK (
                    length("username") >= {username_min_length}
                    AND length("username") <= {username_max_length}
                ),
                "password" bytea NOT NULL CHECK (length("password") = {password_hash_length}),
                "totp_key" bytea CHECK (length("totp_key") = {totp_key_length}),
                "last_used_totp" text CHECK (length("last_used_totp") = {totp_digits}),
                "password_change_reason" password_change_reason,
                "icon" text CHECK (length("icon") = {icon_id_length}),
                "locale" locale NOT NULL
            );
            CREATE UNIQUE INDEX "users_username_key" ON "users" (lower("username"));

            CREATE TABLE "new_totp_keys" (
                "user_id" text UNIQUE NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                "key" text NOT NULL CHECK (length("key") = {totp_key_length}),
                "expires" timestamp(0) with time zone NOT NULL
            );

            CREATE TYPE "permission" AS ENUM ({permissions});
            CREATE TABLE "permissions" (
                "user_id" text REFERENCES "users"("id") ON DELETE CASCADE,
                "permission" permission,
                PRIMARY KEY ("user_id", "permission")
            );

            CREATE TABLE "remember_tokens" (
                "id" bytea PRIMARY KEY CHECK (length("id") = {blake2_output_length}),
                "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                "secret" bytea NOT NULL CHECK (length("secret") = {blake2_output_length})
            );

            CREATE TABLE "sessions" (
                "id" bytea PRIMARY KEY CHECK (length("id") = {blake2_output_length}),
                "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                "csrf_token" text NOT NULL CHECK (length("csrf_token") = {csrf_token_length}),
                "expires" timestamp(0) with time zone NOT NULL,
                "sudo_until" timestamp(0) with time zone
            );
            "#,
            user_id_length = CONFIG.user.id_length,
            username_min_length = CONFIG.user.username_min_length,
            username_max_length = CONFIG.user.username_max_length,
            totp_digits = CONFIG.totp.digits,
            icon_id_length = CONFIG.user.icon_id_length,
            blake2_output_length = blake2("").len(),
            csrf_token_length = CONFIG.csrf.token_length,
        )
    ).await.unwrap();
}
