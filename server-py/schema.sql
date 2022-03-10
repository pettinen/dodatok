DROP TABLE IF EXISTS "sessions", "remember_tokens", "new_totp_keys", "permissions", "users";
DROP TYPE IF EXISTS "locale", "password_change_reason", "permission";

CREATE TYPE "locale" AS ENUM('en-US', 'fi-FI');
CREATE TYPE "password_change_reason" AS ENUM('session-compromise');
CREATE TABLE "users" (
    "id" text PRIMARY KEY CHECK (length("id") = 43),
    "username" text NOT NULL CHECK (length("username") > 0 AND length("username") <= 20),
    "password_hash" bytea NOT NULL CHECK (length("password_hash") = 228),
    "totp_key" text CHECK (length("totp_key") = 184),
    "last_used_totp" text CHECK (length("last_used_totp") = 6),
    "password_change_reason" password_change_reason,
    "disabled" boolean NOT NULL DEFAULT false,
    "icon" text CHECK (length("id") = 43),
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
  "secret_hash" bytea NOT NULL CHECK (length("secret_hash") = 32)
);

CREATE TABLE "sessions" (
  "id" bytea PRIMARY KEY CHECK (length("id") = 32),
  "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
  "csrf_token" text NOT NULL CHECK (length("id") = 43),
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
  'A',2
  -- Password: a
  'gAAAAABiGexqzf3zS9eGqLt3b0nScMPUoL9V3SoE4X38ItYqir8JygtlEii7NDZgEA9PWcMYHV2iXe8dUrwCWxo_6QSzdeF-y-Ge-u2RLuyHK90banIavyFkQ5AJSPVLr4AGeOcKPsrt9QE28Bd0O8HtNqNHGwo8PathJO0xdZ2VH_3yBbbbLdNkjk4Gkt3AEwimv23rkRCVHsWwfLm01m6MRfNeoY3AfQ==',
  -- TOTP key: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
  'gAAAAABiD2t7G_X55KmyrEaVw4o6lbba0UUvZ4ifOnCnS_5etUw0kLNeQrA_b1JAfRPu3bj-JCWVVoPEU_b_RCpErEpl3pAHAmgv4S6F3MCBENGurVmPJpS6mHyH5Pmt77GUwGJMg5CB-y_dAoIsTU7H-NXeRscOga-R7DPKYOSc6XZ4v2uRduY=',
  'https://reqres.in/img/faces/6-image.jpg',
  'en-US'
);

INSERT INTO "permissions"("user_id", "permission") VALUES
  ('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', 'delete_user'),
  ('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', 'edit_user'),
  ('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', 'ignore_rate_limits');
