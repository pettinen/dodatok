import asyncio
import secrets
from datetime import datetime, timedelta
from typing import Optional, TypedDict

from argon2.exceptions import InvalidHash, VerificationError, VerifyMismatchError
from asyncpg import Connection
from cryptography.fernet import InvalidToken
from pyotp import TOTP
from quart import Blueprint, ResponseReturnValue, current_app, g, make_response, request

from api import services
from api.utils import (
    APIError,
    UnexpectedError,
    api_error,
    auth_required,
    camel_case,
    csrf_protected,
    delete_auth_cookies,
    delete_cookies,
    rate_limit,
    run_task,
    sha3_256,
    single_error,
    try_insert_unique,
    unauthenticated_only,
    utcnow,
    validate_json_schema,
)


auth = Blueprint("auth", __name__)

login_schema = {
    "type": "object",
    "required": ["username", "password", "remember"],
    "properties": {
        "username": {"type": "string"},
        "password": {"type": "string"},
        "remember": {"type": "boolean"},
        "totp": {"type": "string"},
    },
    "additionalProperties": False,
}


class LoginSchemaBase(TypedDict):
    username: str
    password: str
    remember: bool

class LoginSchema(LoginSchemaBase, total=False):
    totp: str


class UserResponse(TypedDict):
    id: str
    username: str
    totpEnabled: bool
    passwordChangeReason: Optional[str]
    icon: Optional[str]
    locale: str
    sudoUntil: Optional[datetime]


class LoginResponseBase(TypedDict):
    csrfToken: str
    user: UserResponse

class LoginResponse(LoginResponseBase, total=False):
    warnings: list[APIError]


async def insert_remember_token(
    user_id: str,
    *,
    connection: Connection,
) -> tuple[str, str]:
    remember_secret = secrets.token_urlsafe(
        current_app.config["REMEMBER_TOKEN_SECRET_BYTES"]
    )

    async def execute_insert_remember_token() -> str:
        remember_id = secrets.token_urlsafe(
            current_app.config["REMEMBER_TOKEN_ID_BYTES"]
        )
        await connection.execute(
            """
            INSERT INTO "remember_tokens"("id", "user_id", "secret_hash")
            VALUES ($1, $2, $3)
            """,
            remember_id,
            user_id,
            sha3_256(remember_secret),
        )
        return remember_id

    remember_id = await try_insert_unique(
        execute_insert_remember_token, "remember token"
    )
    return remember_id, remember_secret


async def insert_session(
    user_id: str,
    csrf_token: str,
    *,
    sudo: bool,
    connection = None
) -> tuple[str, datetime]:
    if connection is None:
        connection = services.db

    now = utcnow()
    session_expires = now + current_app.config["SESSION_LIFETIME"]
    if sudo:
        sudo_until = (now + current_app.config["SUDO_LIFETIME"]).replace(microsecond=0)
    else:
        sudo_until = None

    async def execute_insert_session() -> str:
        session_id = secrets.token_urlsafe(current_app.config["SESSION_ID_BYTES"])
        await connection.execute(
            """
            INSERT INTO "sessions"(
                "id",
                "csrf_token",
                "user_id",
                "expires",
                "sudo_until"
            )
            VALUES ($1, $2, $3, $4, $5)
            """,
            session_id,
            csrf_token,
            user_id,
            session_expires,
            sudo_until,
        )
        return session_id

    session_id = await try_insert_unique(execute_insert_session, "session")
    return session_id, sudo_until


@auth.get("/csrf-token")
@rate_limit("csrf-token", limit=50, seconds=60)
@unauthenticated_only
async def csrf_token() -> ResponseReturnValue:
    csrf_token = secrets.token_urlsafe(current_app.config["CSRF_TOKEN_BYTES"])
    response = await make_response({"csrfToken": csrf_token})
    response.set_cookie(
        current_app.config["CSRF_COOKIE_NAME"],
        csrf_token,
        httponly=True,
        path=current_app.config["COOKIE_PATH"],
        samesite=current_app.config["COOKIE_SAMESITE"],
        secure=current_app.config["COOKIE_SECURE"],
    )
    return response


@auth.post("/get-session")
@rate_limit("get-session", limit=10, seconds=60)
async def get_session() -> ResponseReturnValue:
    if current_app.config["REMEMBER_TOKEN_COOKIE_NAME"] not in request.cookies:
        return single_error("auth", "no-valid-remember-token")

    try:
        remember_id, remember_secret = request.cookies[
            current_app.config["REMEMBER_TOKEN_COOKIE_NAME"]
        ].split(":")
    except ValueError:
        return await delete_auth_cookies(
            single_error("auth", "no-valid-remember-token")
        )
    if not remember_id or not remember_secret:
        return await delete_auth_cookies(
            single_error("auth", "no-valid-remember-token")
        )

    token = await services.db.fetchrow(
        """
        SELECT
            "remember_tokens"."user_id",
            "remember_tokens"."secret_hash",
            "users"."disabled"
        FROM "remember_tokens"
            JOIN "users" ON "remember_tokens"."user_id" = "users"."id"
        WHERE "remember_tokens"."id" = $1
        """,
        remember_id,
    )

    if token is None:
        return await delete_auth_cookies(
            single_error("auth", "no-valid-remember-token")
        )

    if token["disabled"]:
        return await delete_auth_cookies(single_error("auth", "account-disabled"))

    if not secrets.compare_digest(token["secret_hash"], sha3_256(remember_secret)):
        # Possible session hijack attempt, invalidate sessions
        run_task(
            services.db.execute(
                'DELETE FROM "sessions" WHERE "user_id" = $1',
                token["user_id"],
            ),
            "session-compromise-delete-sessions",
        )
        run_task(
            services.db.execute(
                'DELETE FROM "remember_tokens" WHERE "user_id" = $1',
                token["user_id"],
            ),
            "session-compromise-delete-remember_tokens",
        )
        run_task(
            services.db.execute(
                """
                UPDATE "users" SET "password_change_reason" = 'session-compromise'
                WHERE "id" = $1
                """,
                token["user_id"],
            ),
            "session-compromise-update-password_change_reason",
        )
        return await delete_auth_cookies(
            single_error("auth", "remember-token-secret-mismatch")
        )

    csrf_token = secrets.token_urlsafe(current_app.config["CSRF_TOKEN_BYTES"])
    session_id, _sudo_until = await insert_session(
        token["user_id"], csrf_token, sudo=False
    )

    new_secret = secrets.token_urlsafe(
        current_app.config["REMEMBER_TOKEN_SECRET_BYTES"]
    )
    await services.db.execute(
        'UPDATE "remember_tokens" SET "secret_hash" = $1 WHERE "id" = $2',
        sha3_256(new_secret),
        remember_id,
    )

    response = await make_response({"csrfToken": csrf_token})
    response.set_cookie(
        current_app.config["CSRF_COOKIE_NAME"],
        csrf_token,
        httponly=True,
        path=current_app.config["COOKIE_PATH"],
        samesite=current_app.config["COOKIE_SAMESITE"],
        secure=current_app.config["COOKIE_SECURE"],
    )
    response.set_cookie(
        current_app.config["SESSION_COOKIE_NAME"],
        session_id,
        httponly=True,
        path=current_app.config["COOKIE_PATH"],
        samesite=current_app.config["COOKIE_SAMESITE"],
        secure=current_app.config["COOKIE_SECURE"],
    )
    response.set_cookie(
        current_app.config["REMEMBER_TOKEN_COOKIE_NAME"],
        f"{remember_id}:{new_secret}",
        httponly=True,
        max_age=current_app.config["PERSISTENT_COOKIE_MAX_AGE"],
        path=current_app.config["COOKIE_PATH"],
        samesite=current_app.config["COOKIE_SAMESITE"],
        secure=current_app.config["COOKIE_SECURE"],
    )
    return response


@auth.post("/login")
@rate_limit("login", limit=10, seconds=60)
@csrf_protected
@validate_json_schema(login_schema)
async def login(*, request_data: LoginSchema) -> ResponseReturnValue:
    warnings = []

    data = await services.db.fetchrow(
        """
        SELECT
            "id",
            "username",
            "password_hash",
            "totp_key",
            "last_used_totp",
            "password_change_reason",
            "disabled",
            "icon",
            "locale"
        FROM "users" WHERE lower("username") = lower($1)
        """,
        request_data["username"],
    )

    if data is None:
        return single_error("auth", "invalid-credentials")

    try:
        password_hash = services.fernet.decrypt(data["password_hash"])
    except InvalidToken:
        raise UnexpectedError(
            f"Password hash of user {data['id']} could not be decrypted"
        )

    try:
        services.password_hasher.verify(password_hash, request_data["password"])
    except VerifyMismatchError:
        return single_error("auth", "invalid-credentials")
    except (InvalidHash, VerificationError) as exc:
        raise UnexpectedError(f"Could not verify password of user {data['id']}: {exc}")

    if True or services.password_hasher.check_needs_rehash(password_hash):
        new_hash = services.password_hasher.hash(request_data["password"])
        new_encrypted_hash = services.fernet.encrypt(new_hash)
        run_task(
            services.db.execute(
                'UPDATE "users" SET "password_hash" = $1 WHERE "id" = $2',
                new_encrypted_hash,
                data["id"],
            ),
            "login-rehash-password",
        )

    if data["disabled"]:
        return single_error("auth", "account-disabled")

    if data["totp_key"] is not None:
        if "totp" not in request_data:
            return single_error("auth", "totp-required")
        totp = TOTP(services.fernet.decrypt(data["totp_key"]))
        if not totp.verify(
            request_data["totp"],
            valid_window=current_app.config["TOTP_VALID_WINDOW"],
        ):
            return single_error("auth", "invalid-totp")
        if request_data["totp"] == data["last_used_totp"]:
            return single_error("login", "totp-already-used")
        run_task(
            services.db.execute(
                'UPDATE "users" SET "last_used_totp" = $1 WHERE "id" = $2',
                request_data["totp"],
                data["id"],
            ),
            "update-users-last_used_totp",
        )
    elif request_data.get("totp"):
        warnings.append(api_error("auth", "unused-totp"))

    csrf_token = secrets.token_urlsafe(current_app.config["CSRF_TOKEN_BYTES"])

    async with services.db.acquire() as connection:
        async with connection.transaction():
            session_id, sudo_until = await insert_session(
                data["id"], csrf_token, sudo=True, connection=connection
            )
            if request_data["remember"]:
                remember_id, remember_secret = await insert_remember_token(
                    data["id"],
                    connection=connection,
                )

    response_data: LoginResponse = {
        "csrfToken": csrf_token,
        "user": {
            "id": data["id"],
            "username": data["username"],
            "totpEnabled": data["totp_key"] is not None,
            "passwordChangeReason": data["password_change_reason"],
            "icon": data["icon"],
            "locale": data["locale"],
            "sudoUntil": sudo_until,
        },
    }
    if warnings:
        response_data["warnings"] = warnings

    response = await make_response(response_data)
    response.set_cookie(
        current_app.config["SESSION_COOKIE_NAME"],
        session_id,
        httponly=True,
        path=current_app.config["COOKIE_PATH"],
        samesite=current_app.config["COOKIE_SAMESITE"],
        secure=current_app.config["COOKIE_SECURE"],
    )
    response = await delete_cookies(response, {current_app.config["CSRF_COOKIE_NAME"]})
    if request_data["remember"]:
        response.set_cookie(
            current_app.config["REMEMBER_TOKEN_COOKIE_NAME"],
            f"{remember_id}:{remember_secret}",
            httponly=True,
            max_age=current_app.config["PERSISTENT_COOKIE_MAX_AGE"],
            path=current_app.config["COOKIE_PATH"],
            samesite=current_app.config["COOKIE_SAMESITE"],
            secure=current_app.config["COOKIE_SECURE"],
        )
    else:
        response = await delete_cookies(
            response, {current_app.config["REMEMBER_TOKEN_COOKIE_NAME"]}
        )
    return response


@auth.post("/logout")
@auth_required()
@rate_limit("logout", limit=10, seconds=60)
@csrf_protected
async def logout() -> ResponseReturnValue:
    session_id = request.cookies[current_app.config["SESSION_COOKIE_NAME"]]

    async with services.db.acquire() as connection:
        async with connection.transaction():
            await connection.execute(
                'DELETE FROM "sessions" WHERE "id" = $1', session_id
            )

            if current_app.config["REMEMBER_TOKEN_COOKIE_NAME"] in request.cookies:
                try:
                    remember_id, _ = request.cookies[
                        current_app.config["REMEMBER_TOKEN_COOKIE_NAME"]
                    ].split(":")
                    await connection.execute(
                        'DELETE FROM "remember_tokens" WHERE "id" = $1', remember_id
                    )
                except ValueError:
                    pass

    csrf_token = secrets.token_urlsafe(current_app.config["CSRF_TOKEN_BYTES"])
    response = await delete_auth_cookies({"csrfToken": csrf_token})
    response.set_cookie(
        current_app.config["CSRF_COOKIE_NAME"],
        csrf_token,
        httponly=True,
        path=current_app.config["COOKIE_PATH"],
        samesite=current_app.config["COOKIE_SAMESITE"],
        secure=current_app.config["COOKIE_SECURE"],
    )
    return response


@auth.post("/logout/all-sessions")
@auth_required()
@rate_limit("logout", limit=10, seconds=60)
@csrf_protected
async def logout_all_sessions() -> ResponseReturnValue:
    async with services.db.acquire() as connection:
        async with connection.transaction():
            await connection.execute(
                'DELETE FROM "sessions" WHERE "user_id" = $1',
                g.user["id"],
            )
            await connection.execute(
                'DELETE FROM "remember_tokens" WHERE "user_id" = $1',
                g.user["id"],
            )

    csrf_token = secrets.token_urlsafe(current_app.config["CSRF_TOKEN_BYTES"])
    response = await delete_auth_cookies({"csrfToken": csrf_token})
    response.set_cookie(
        current_app.config["CSRF_COOKIE_NAME"],
        csrf_token,
        httponly=True,
        path=current_app.config["COOKIE_PATH"],
        samesite=current_app.config["COOKIE_SAMESITE"],
        secure=current_app.config["COOKIE_SECURE"],
    )
    return response
