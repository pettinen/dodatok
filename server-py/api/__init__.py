import asyncio
import logging
from collections import deque
from collections.abc import AsyncGenerator, Awaitable
from functools import wraps
from typing import cast

import socketio
from quart import Quart, Response, ResponseReturnValue, make_response, request
from werkzeug.exceptions import HTTPException

import api.json
from api._services import Services
from api.types import Config


quart_app = Quart(__name__)
quart_app.config.from_object(f"config.development")


config = cast(Config, quart_app.config)
logger = quart_app.logger

logging.getLogger().setLevel(config["LOG_LEVEL"])

services = Services(config)

quart_app.json_encoder = api.json.CustomJSONEncoder

if config["CORS"]:
    @quart_app.after_request
    def add_cors_headers(response: Response) -> Response:
        response.headers.add("Access-Control-Allow-Credentials", "true")
        response.headers.add("Access-Control-Allow-Headers", "X-CSRF-Token")
        response.headers.add("Access-Control-Allow-Origin", config["CLIENT_ORIGIN"])
        response.headers.add("Vary", "Origin")
        return response


@quart_app.errorhandler(HTTPException)
def handle_error(error: Exception) -> ResponseReturnValue:
    from api.utils import api_error

    assert isinstance(error, HTTPException)
    if error.response is not None:
        return error.response, error.code
    if error.code is not None and error.code < 500:
        name = error.name.replace(" ", "-").lower()
    else:
        name = "unexpected"
    return {"errors": [api_error("general", name)]}, error.code


engineio_logger = logging.getLogger("engineio")
socketio_logger = logging.getLogger("socketio")

sio = socketio.AsyncServer(
    async_mode="asgi",
    cors_allowed_origins=config["CLIENT_ORIGIN"],
    json=api.json,
    logger=socketio_logger,
    engineio_logger=engineio_logger,
)

from api.sockets import account_socket

sio.register_namespace(account_socket)

from api.account import account
from api.auth import auth
from api.users import users

quart_app.register_blueprint(account)
quart_app.register_blueprint(auth)
quart_app.register_blueprint(users)

app = socketio.ASGIApp(sio, quart_app, socketio_path=config["SOCKETIO_PATH"])


def initialize(*, reset: bool = False, populate: bool = False) -> None:
    import json
    import secrets
    from minio.deleteobjects import DeleteObject
    from minio.error import S3Error
    from api.account import generate_totp_key
    from api.utils import sha3_256

    async def run_reset() -> None:
        tasks: set[Awaitable[object]] = set()

        def delete_bucket(bucket_name: str) -> None:
            try:
                delete_objects = (
                    DeleteObject(obj.object_name)
                    for obj in services.s3.list_objects(bucket, recursive=True)
                )
                deque(services.s3.remove_objects(bucket, delete_objects), 0)
                services.s3.remove_bucket(bucket)
            except S3Error as exc:
                if exc.code != "NoSuchBucket":
                    raise

        for bucket in {config["S3_FILES_BUCKET"], config["S3_USER_ICONS_BUCKET"]}:
            tasks.add(asyncio.to_thread(delete_bucket, bucket))

        tasks.add(
            services.db.execute(
                """
                DROP TABLE IF EXISTS
                    "websocket_tokens",
                    "sessions",
                    "remember_tokens",
                    "new_totp_keys",
                    "permissions",
                    "users";
                DROP TYPE IF EXISTS "locale", "password_change_reason", "permission";
                """
            )
        )

        await asyncio.gather(*tasks)

    users = {
        "a": {
            "id": "".join("0" for _ in secrets.token_urlsafe(config["USER_ID_BYTES"])),
            "password_hash": services.fernet.encrypt(
                services.password_hasher.hash("a")
            ),
            "username": "A" * config["USERNAME_MIN_LENGTH"],
        },
        "b": {
            "id": "".join("A" for _ in secrets.token_urlsafe(config["USER_ID_BYTES"])),
            "password_hash": services.fernet.encrypt(
                services.password_hasher.hash("a")
            ),
            "username": "B" * config["USERNAME_MAX_LENGTH"],
            "totp_key": services.fernet.encrypt(
                "".join("A" for _ in generate_totp_key())
            ),
        },
    }

    async def run_create() -> None:
        tasks: set[Awaitable[object]] = set()

        def make_bucket(bucket_name: str) -> None:
            try:
                services.s3.make_bucket(bucket)
                policy = {
                    "Version": "2012-10-17",
                    "Statement": [
                        {
                            "Effect": "Allow",
                            "Principal": {"AWS": "*"},
                            "Action": "s3:GetObject",
                            "Resource": f"arn:aws:s3:::{bucket}/*",
                        },
                    ],
                }
                services.s3.set_bucket_policy(bucket, json.dumps(policy))

            except S3Error as exc:
                if exc.code != "BucketAlreadyOwnedByYou":
                    raise

        for bucket in {config["S3_FILES_BUCKET"], config["S3_USER_ICONS_BUCKET"]}:
            tasks.add(asyncio.to_thread(make_bucket, bucket))

        user_id_length = len(users["a"]["id"])
        password_hash_length = len(users["a"]["password_hash"])
        totp_key_length = len(users["b"]["totp_key"])
        icon_id_length = len(secrets.token_urlsafe(config["USER_ICON_ID_BYTES"]))
        remember_token_id_length = len(
            secrets.token_urlsafe(config["REMEMBER_TOKEN_ID_BYTES"])
        )
        remember_token_secret_length = len(sha3_256(""))
        session_id_length = len(secrets.token_urlsafe(config["SESSION_ID_BYTES"]))
        csrf_token_length = len(secrets.token_urlsafe(config["CSRF_TOKEN_BYTES"]))
        websocket_token_length = len(
            secrets.token_urlsafe(config["WEBSOCKET_TOKEN_BYTES"])
        )
        assert not any("'" in locale for locale in config["LOCALES"])
        locales = ", ".join(f"'{locale}'" for locale in config["LOCALES"])

        create_db = services.db.execute(
            f"""
            CREATE TYPE "locale" AS ENUM({locales});
            CREATE TYPE "password_change_reason" AS ENUM('session-compromise');
            CREATE TYPE "permission" AS ENUM(
                'delete_user',
                'edit_user',
                'ignore_rate_limits'
            );

            CREATE TABLE "users" (
                "id" text PRIMARY KEY CHECK (length("id") = {user_id_length}),
                "username" text NOT NULL
                  CHECK (length("username") >= {config["USERNAME_MIN_LENGTH"]}
                      AND length("username") <= {config["USERNAME_MAX_LENGTH"]}),
                "password_hash" text NOT NULL
                    CHECK (length("password_hash") = {password_hash_length}),
                "totp_key" text CHECK (length("totp_key") = {totp_key_length}),
                "last_used_totp" text
                    CHECK (length("last_used_totp") = {config["TOTP_DIGITS"]}),
                "password_change_reason" password_change_reason,
                "disabled" boolean NOT NULL DEFAULT false,
                "icon" text CHECK (length("id") = {icon_id_length}),
                "locale" locale NOT NULL
            );
            CREATE UNIQUE INDEX "users_username_key" ON "users" (lower("username"));

            CREATE TABLE "new_totp_keys" (
                "user_id" text UNIQUE NOT NULL
                    REFERENCES "users"("id") ON DELETE CASCADE,
                "key" text NOT NULL CHECK (length("key") = {totp_key_length}),
                "expires" timestamp(0) with time zone NOT NULL
            );

            CREATE TABLE "permissions" (
                "user_id" text REFERENCES "users"("id") ON DELETE CASCADE,
                "permission" permission,
                PRIMARY KEY ("user_id", "permission")
            );

            CREATE TABLE "remember_tokens" (
              "id" text PRIMARY KEY CHECK (length("id") = {remember_token_id_length}),
              "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
              "secret_hash" text NOT NULL
                  CHECK (length("secret_hash") = {remember_token_secret_length})
            );

            CREATE TABLE "sessions" (
              "id" text PRIMARY KEY CHECK (length("id") = {session_id_length}),
              "csrf_token" text NOT NULL CHECK (length("id") = {csrf_token_length}),
              "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
              "expires" timestamp(0) with time zone NOT NULL,
              "sudo_until" timestamp(0) with time zone
            );

            CREATE TABLE "websocket_tokens" (
              "id" text PRIMARY KEY CHECK (length("id") = {websocket_token_length}),
              "user_id" text NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
              "expires" timestamp(0) with time zone NOT NULL
            );
            """
        )
        tasks.add(create_db)

        await asyncio.gather(*tasks)

    async def run_populate() -> None:
        tasks: set[Awaitable[object]] = set()

        tasks.add(
            asyncio.to_thread(
                services.s3.fput_object,
                config["S3_FILES_BUCKET"],
                "default-user-icon.png",
                "files/default-user-icon.png",
                content_type="image/png",
            )
        )

        populate_db = services.db.execute(
            f"""
            INSERT INTO "users"("id", "username", "password_hash", "totp_key", "locale")
            VALUES
                (
                    '{users["a"]["id"]}',
                    '{users["a"]["username"]}',
                    '{users["a"]["password_hash"]}',
                    NULL,
                    '{config["LOCALES"][0]}'
                ),
                (
                    '{users["b"]["id"]}',
                    '{users["b"]["username"]}',
                    '{users["b"]["password_hash"]}',
                    '{users["b"]["totp_key"]}',
                    '{config["LOCALES"][-1]}'
                );

            INSERT INTO "permissions"("user_id", "permission") VALUES
              ('{users["a"]["id"]}', 'delete_user'),
              ('{users["a"]["id"]}', 'edit_user'),
              ('{users["a"]["id"]}', 'ignore_rate_limits');
            """
        )
        tasks.add(populate_db)

        await asyncio.gather(*tasks)

    async def run() -> None:
        if reset:
            await run_reset()
        await run_create()
        if populate:
            await run_populate()

    asyncio.run(run())
