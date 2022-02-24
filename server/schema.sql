DROP TABLE IF EXISTS "sessions", "remember_tokens", "new_totp_keys", "permissions", "users";
DROP TYPE IF EXISTS "locale", "password_change_reason", "permission";

CREATE TYPE "locale" AS ENUM('en-US', 'fi-FI');
CREATE TYPE "password_change_reason" AS ENUM('session-compromise');
CREATE TABLE "users" (
    "id" text PRIMARY KEY CHECK (length("id") = 43),
    "username" text NOT NULL CHECK (length("username") > 0 AND length("username") <= 20),
    "password_hash" text NOT NULL CHECK (length("password_hash") = 228),
    "totp_key" text CHECK (length("totp_key") = 184),
    "last_used_totp" text CHECK (length("last_used_totp") = 6),
    "password_change_reason" password_change_reason,
    "disabled" boolean NOT NULL DEFAULT false,
    "icon" text,
    "locale" locale NOT NULL
);
CREATE UNIQUE INDEX "users_username_key" ON "users" (lower("username"));

CREATE TABLE "new_totp_keys" (
  "user_id" text UNIQUE NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
  "key" text NOT NULL CHECK (length("key") = 184),
  "expires" timestamp(0) with time zone NOT NULL
);

CREATE TYPE "permission" AS ENUM ('delete_user', 'edit_user', 'ignore_rate_limits');
CREATE TABLE "permissions" (
  "user_id" text REFERENCES "users"("id") ON DELETE CASCADE,
  "permission" permission,
  PRIMARY KEY ("user_id", "permission")
);

CREATE TABLE "remember_tokens" (
  "id" text PRIMARY KEY CHECK (length("id") = 43),
  "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
  "secret_hash" text NOT NULL CHECK (length("secret_hash") = 64)
);

CREATE TABLE "sessions" (
  "id" text PRIMARY KEY CHECK (length("id") = 43),
  "csrf_token" text NOT NULL CHECK (length("id") = 43),
  "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
  "expires" timestamp(0) with time zone NOT NULL,
  "sudo_until" timestamp(0) with time zone
);

CREATE TABLE "websocket_tokens" (
  "id" text PRIMARY KEY CHECK (length("id") = 43),
  "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
  "expires" timestamp(0) with time zone NOT NULL
);

INSERT INTO "users"("id", "username", "password_hash", "totp_key", "icon", "locale")
VALUES (
  'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
  'A',
  -- Password: asdfasdf
  'gAAAAABiD1TUe3sT-1U053ZqFurkVX2sCGtqoZWHefyxEgrxUdF8g9v1LXpvzA14Rdt2-O0xGnOcK-L5CatPYQu4PUBWL8t4AhkxWk2nCpwXLOz0tsF_i8ES8eTwDS1reF98MZBrLGse9W_DDkLLwZmWWwvj0CY-WGGwgXpYzL4noh4R_AJR9gwvxxGA-q7SqKy2M7FqH4FWZbbX-p8yO8CtQAExJwAiLQ==',
  -- TOTP key: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
  'gAAAAABiD2t7G_X55KmyrEaVw4o6lbba0UUvZ4ifOnCnS_5etUw0kLNeQrA_b1JAfRPu3bj-JCWVVoPEU_b_RCpErEpl3pAHAmgv4S6F3MCBENGurVmPJpS6mHyH5Pmt77GUwGJMg5CB-y_dAoIsTU7H-NXeRscOga-R7DPKYOSc6XZ4v2uRduY=',
  'https://reqres.in/img/faces/6-image.jpg',
  'en-US'
);

INSERT INTO "permissions"("user_id", "permission") VALUES
  ('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', 'delete_user'),
  ('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', 'edit_user'),
  ('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', 'ignore_rate_limits');
