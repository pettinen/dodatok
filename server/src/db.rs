use aes_gcm_siv::{aead::NewAead, Aes256GcmSiv};
use deadpool_postgres::{
    tokio_postgres::{error::SqlState, NoTls},
    Config as DbConfig,
};
use postgres_types::{FromSql, ToSql};
use serde::Serialize;

use crate::{
    config::Config,
    util::{blake2, encrypt, generate_token, generate_totp_key, hash_encrypt_password},
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
    variants
        .into_iter()
        .map(|name| format!("'{}'", name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn sanitize_db_identifier(value: &str) -> String {
    if value.contains('\0') {
        panic!("Postgres identifiers must not contain null characters");
    }
    value.replace('"', "\"\"")
}

pub async fn init_db(drop_existing: bool, config: &Config) {
    let Some(init_config) = config.dev.init_db.as_ref() else {
        return;
    };
    let pool = init_config.create_pool(None, NoTls).unwrap();
    let db = pool.get().await.unwrap();
    let dbname = sanitize_db_identifier(config.db.dbname.as_ref().unwrap());
    let user = sanitize_db_identifier(config.db.user.as_ref().unwrap());
    let password = config.db.password.as_ref().unwrap().replace('\'', "''");
    if let Err(err) = db
        .execute(
            &format!(
                r#"CREATE DATABASE "{}""#, dbname
            ),
            &[],
        )
        .await
    {
        if err.code() != Some(&SqlState::DUPLICATE_DATABASE) {
            panic!("{}", err);
        }
    }
    if let Err(err) = db
        .execute(
            &format!(r#"CREATE USER "{}""#, user),
            &[],
        )
        .await
    {
        if err.code() != Some(&SqlState::DUPLICATE_OBJECT) {
            panic!("{}", err);
        }
    }
    db.execute(
        &format!(r#"ALTER ROLE "{}" PASSWORD '{}'"#, user, password),
        &[],
    )
    .await
    .unwrap();

    let init_config = DbConfig{
        dbname: config.db.dbname.clone(),
        ..init_config.clone()
    };
    let pool = init_config.create_pool(None, NoTls).unwrap();
    let db = pool.get().await.unwrap();

    let aes = Aes256GcmSiv::new_from_slice(&config.security.aes_key).unwrap();

    let password_hash_length = hash_encrypt_password("a", &aes, config).unwrap().len();
    let totp_key_length = encrypt(&generate_totp_key(config), &aes, &mut rand::thread_rng())
        .unwrap()
        .len();

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
        )
        .await
        .unwrap();
    }

    db.batch_execute(&format!(
        r#"
        DO $$ BEGIN
            CREATE TYPE "locale" AS ENUM({locales});
        EXCEPTION
            WHEN duplicate_object THEN null;
        END $$;
        DO $$ BEGIN
        CREATE TYPE "password_change_reason" AS ENUM({password_change_reasons});
        EXCEPTION
            WHEN duplicate_object THEN null;
        END $$;
        CREATE TABLE IF NOT EXISTS "users" (
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
        CREATE UNIQUE INDEX IF NOT EXISTS "users_username_key" ON "users" (lower("username"));

        CREATE TABLE IF NOT EXISTS "new_totp_keys" (
            "user_id" text UNIQUE NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
            "key" text NOT NULL CHECK (length("key") = {totp_key_length}),
            "expires" timestamp(0) with time zone NOT NULL
        );

        DO $$ BEGIN
            CREATE TYPE "permission" AS ENUM ({permissions});
        EXCEPTION
            WHEN duplicate_object THEN null;
        END $$;
        CREATE TABLE IF NOT EXISTS "permissions" (
            "user_id" text REFERENCES "users"("id") ON DELETE CASCADE,
            "permission" permission,
            PRIMARY KEY ("user_id", "permission")
        );

        CREATE TABLE IF NOT EXISTS "remember_tokens" (
            "id" bytea PRIMARY KEY CHECK (length("id") = {blake2_output_length}),
            "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
            "secret" bytea NOT NULL CHECK (length("secret") = {blake2_output_length})
        );

        CREATE TABLE IF NOT EXISTS "sessions" (
            "id" bytea PRIMARY KEY CHECK (length("id") = {blake2_output_length}),
            "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
            "csrf_token" text NOT NULL CHECK (length("csrf_token") = {csrf_token_length}),
            "expires" timestamp(0) with time zone NOT NULL,
            "sudo_until" timestamp(0) with time zone
        );
        "#,
        user_id_length = config.user.id_length,
        username_min_length = config.user.username_min_length,
        username_max_length = config.user.username_max_length,
        totp_digits = config.totp.digits,
        icon_id_length = config.user.icon_id_length,
        blake2_output_length = blake2("").len(),
        csrf_token_length = config.csrf.token_length,
    ))
    .await
    .unwrap();

    db.execute(
        &format!(r#"GRANT ALL ON ALL TABLES IN SCHEMA "public" TO "{}""#, user),
        &[],
    )
    .await
    .unwrap();
}

pub async fn populate_db(config: &Config, aes: &Aes256GcmSiv) {
    let pool = config.db.create_pool(None, NoTls).unwrap();
    let db = pool.get().await.unwrap();

    let user_id = generate_token(config.user.id_length);
    let password_hash = hash_encrypt_password("b", &aes, config).unwrap();
    if let Err(err) = db
        .execute(
            r#"
            INSERT INTO "users"("id", "active", "username", "password", "locale") VALUES
                ($1, true, 'a', $2, 'en-US');
            "#,
            &[&user_id, &password_hash],
        )
        .await
    {
        if err.code() != Some(&SqlState::UNIQUE_VIOLATION) {
            panic!("{}", err);
        }
    }
}
