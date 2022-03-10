import secrets
from datetime import datetime
from io import BytesIO
from typing import Literal, Optional, TypedDict, Union, cast
from urllib.parse import urlunsplit

from argon2.exceptions import InvalidHash, VerificationError, VerifyMismatchError
from asyncpg import UniqueViolationError
from cryptography.fernet import InvalidToken
from PIL import Image, UnidentifiedImageError
from PIL.ImageOps import exif_transpose
from pyotp import TOTP
from quart import Blueprint, ResponseReturnValue, abort, current_app, g, request

from api import services
from api.utils import (
    Alert,
    UnexpectedError,
    api_error,
    auth_required,
    camel_case,
    csrf_protected,
    delete_auth_cookies,
    has_permission,
    rate_limit,
    run_task,
    single_error,
    square_box,
    utcnow,
    validate_json_schema,
)
from api.validation import validate_locale, validate_password, validate_username
from api.sockets import account_socket, user_room


users = Blueprint("users", __name__, url_prefix="/users")

put_schema = {
    "type": "object",
    "properties": {
        "currentPassword": {"type": "string"},
        "totp": {"type": ["string", "null"]},
        "locale": {"type": "string"},
        "newPassword": {"type": "string"},
        "username": {"type": "string"},
    },
    "additionalProperties": False,
}

class PutSchema(TypedDict, total=False):
    currentPassword: str
    totp: Union[str, None]
    locale: str
    newPassword: str
    username: str

class PutResult(TypedDict, total=False):
    errors: list[Alert]
    locale: str
    passwordChangeReason: None
    passwordUpdated: Literal[True]
    sudoUntil: str
    totpEnabled: bool
    username: str
    warnings: list[Alert]

put_icon_schema = {
    "type": "object",
    "required": ["remove"],
    "properties": {
        "remove": {"enum": [True]},
    },
    "additionalProperties": False,
}

class PutIconSchema(TypedDict):
    remove: Literal[True]

delete_schema = {
    "type": "object",
    "required": ["password"],
    "properties": {
        "password": {"type": "string"},
    },
    "additionalProperties": False,
}

class DeleteSchema(TypedDict):
    password: str


async def enable_totp(verification_code: str) -> Alert | Literal[True]:
    if g.user["totp_key"] is not None:
        return api_error("account", "totp-already-enabled")

    class DBResponse(TypedDict):
        key: str
        expires: datetime

    totp_key_data = cast(
        Optional[DBResponse],
        await services.db.fetchrow(
            'SELECT "key", "expires" FROM "new_totp_keys" WHERE "user_id" = $1',
            g.user["id"],
        ),
    )

    if totp_key_data is None or totp_key_data["expires"] < utcnow():
        return api_error("account", "no-totp-key-active")

    totp_key = services.fernet.decrypt(totp_key_data["key"])
    totp = TOTP(totp_key)
    if not totp.verify(
        verification_code, valid_window=current_app.config["TOTP_VALID_WINDOW"]
    ):
        return api_error("account", "invalid-totp-verification")

    encrypted_totp_key = services.fernet.encrypt(totp_key)
    await services.db.execute(
        'UPDATE "users" SET "totp_key" = $1, "last_used_totp" = $2 WHERE "id" = $3',
        encrypted_totp_key,
        verification_code,
        g.user["id"],
    )
    run_task(
        services.db.execute(
            'DELETE FROM "new_totp_keys" WHERE "user_id" = $1', g.user["id"]
        ),
        "delete-used-new_totp_key",
    )
    return True


async def disable_totp(user_id: str) -> None:
    run_task(
        services.db.execute(
            'DELETE FROM "new_totp_keys" WHERE "user_id" = $1',
            user_id,
        ),
        "disable-totp-delete-unused-keys",
    )
    await services.db.execute(
        'UPDATE "users" SET "totp_key" = NULL, "last_used_totp" = NULL WHERE "id" = $1',
        user_id,
    )


async def update_locale(
    user_id: str,
    new_locale: str,
) -> list[Alert] | Literal[True]:
    if errors := validate_locale(new_locale):
        return errors

    await services.db.execute(
        'UPDATE "users" SET "locale" = $1 WHERE "id" = $2',
        new_locale,
        user_id,
    )
    return True


async def update_password(
    user_id: str,
    new_password: str,
) -> list[Alert] | Literal[True]:
    if errors := validate_password(new_password, "new-password"):
        return errors

    new_hash = services.password_hasher.hash(new_password)
    new_encrypted_hash = services.fernet.encrypt(new_hash)
    await services.db.execute(
        """
        UPDATE "users" SET "password_hash" = $1, "password_change_reason" = NULL
        WHERE "id" = $2
        """,
        new_encrypted_hash,
        user_id,
    )
    return True


async def update_username(
    user_id: str,
    new_username: str,
) -> list[Alert] | Literal[True]:
    if errors := validate_username(new_username, "new-username"):
        return errors

    try:
        await services.db.execute(
            'UPDATE "users" SET "username" = $1 WHERE "id" = $2',
            new_username,
            user_id,
        )
    except UniqueViolationError:
        return [api_error("account", "username-not-available")]
    return True


@users.get("/username-available/<string:username>")
@rate_limit("username-available", limit=50, seconds=60)
async def username_available(username: str) -> ResponseReturnValue:
    if errors := validate_username(username, "new-username"):
        return {"errors": errors}, 400

    available = await services.db.fetchval(
        'SELECT count(*) = 0 FROM "users" WHERE lower("username") = lower($1)',
        username,
    )
    return {"available": available}


@users.get("/me")
@auth_required()
@rate_limit("users-me", limit=50, seconds=60)
async def me() -> ResponseReturnValue:
    data = await services.db.fetchrow(
        """
        SELECT
            "id",
            "username",
            "disabled",
            "totp_key",
            "password_change_reason",
            "icon",
            "locale"
        FROM "users" WHERE "id" = $1
        """,
        g.user["id"],
    )
    if data is None:
        abort(404)
    return {
        "id": data["id"],
        "username": data["username"],
        "totpEnabled": data["totp_key"] is not None,
        "passwordChangeReason": data["password_change_reason"],
        "icon": data["icon"],
        "locale": data["locale"],
        "sudoUntil": g.user["sudo_until"],
    }


@users.delete("/<string:user_id>")
@auth_required({"password_hash"})
@rate_limit("delete-user", limit=1, seconds=60)
@csrf_protected
@validate_json_schema(delete_schema, ignore_non_json=True)
async def delete(
    user_id: str,
    *,
    request_data: Optional[DeleteSchema],
) -> ResponseReturnValue:
    deleting_self = user_id == g.user["id"]

    if not deleting_self and not await has_permission("delete_user"):
        abort(403)

    if deleting_self:
        if request_data is None or not request_data["password"]:
            return await single_error("account", "missing-current-password")
        try:
            password_hash = services.fernet.decrypt(g.user["password_hash"])
        except InvalidToken:
            raise UnexpectedError(
                f"Password hash of user {user_id} could not be decrypted"
            )
        try:
            services.password_hasher.verify(password_hash, request_data["password"])
        except VerifyMismatchError:
            return await single_error("account", "invalid-current-password")
        except (InvalidHash, VerificationError) as exc:
            raise UnexpectedError(
                f"Could not verify password of user {user_id}: {exc}"
            )

    await services.db.execute('DELETE FROM "users" WHERE "id" = $1', user_id)

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


@users.put("/<string:user_id>")
@auth_required({"totp_key"})
@rate_limit("put-user", limit=20, seconds=60)
@csrf_protected
@validate_json_schema(put_schema)
async def put(user_id: str, *, request_data: PutSchema) -> ResponseReturnValue:
    updating_self = user_id == g.user["id"]
    if not updating_self and not await has_permission("edit_user"):
        abort(403)

    class DBResponse(TypedDict):
        username: str
        password_hash: str
        password_change_reason: Optional[str]
        icon: Optional[str]
        locale: str

    user_from_db = cast(
        Optional[DBResponse],
        await services.db.fetchrow(
            """
            SELECT
                "username",
                "password_hash",
                "password_change_reason",
                "icon",
                "locale"
            FROM "users" WHERE "id" = $1
            """,
            user_id,
        )
    )

    if user_from_db is None:
        abort(404)

    old_user = user_from_db

    old_password_hash = None
    old_password_verified = (
        g.user["sudo_until"] is not None and utcnow() < g.user["sudo_until"]
    )
    invalid_old_password = False
    update_sudo_until = False

    def verify_password(password: Optional[str] = None) -> Union[Alert, bool]:
        nonlocal old_password_hash, old_password_verified, invalid_old_password

        if password is None:
            if old_password_verified:
                return True
            if invalid_old_password:
                return False
            if not (password := request_data.get("currentPassword")):
                invalid_old_password = True
                return api_error("validation", "current-password.empty")
            verifying_old_password = True
        else:
            verifying_old_password = False

        if old_password_hash is None:
            try:
                old_password_hash = services.fernet.decrypt(
                    old_user["password_hash"]
                )
            except InvalidToken:
                raise UnexpectedError(
                    f"Password hash of user {user_id} could not be decrypted"
                )

        try:
            services.password_hasher.verify(old_password_hash, password)
        except VerifyMismatchError:
            if verifying_old_password:
                invalid_old_password = True
                return api_error("account", "invalid-current-password")
            else:
                return False
        except (InvalidHash, VerificationError) as exc:
            if verifying_old_password:
                raise UnexpectedError(
                    f"Could not verify password of user {user_id}: {exc}"
                )
            else:
                raise UnexpectedError(
                    f"Could not verify new password of user {user_id}: {exc}"
                )

        if verifying_old_password:
            old_password_verified = True
        return True

    errors = []
    warnings = []
    rv: PutResult = {}
    unexpected_errors = False

    if "username" in request_data:
        new_username = request_data["username"].strip()
        if old_user["username"] != new_username:
            verify_password_result: Union[Alert, bool] = True
            if (
                updating_self
                and old_user["username"].lower() != new_username.lower()
            ):
                verify_password_result = verify_password()

            if not isinstance(verify_password_result, bool):
                errors.append(verify_password_result)
            elif verify_password_result:
                update_sudo_until = True
                try:
                    username_result = await update_username(user_id, new_username)
                    if username_result is True:
                        rv["username"] = new_username
                    else:
                        errors.extend(username_result)
                except Exception as exc:
                    current_app.logger.error(exc)
                    unexpected_errors = True

    if "newPassword" in request_data:
        if not updating_self:
            errors.append(api_error(
                "account", "cannot-update-password-for-others"
            ))
        else:
            verify_password_result = verify_password()
            if not isinstance(verify_password_result, bool):
                errors.append(verify_password_result)
            elif verify_password_result:
                update_sudo_until = True
                new_password = request_data["newPassword"]
                if verify_password(new_password):
                    if old_user["password_change_reason"]:
                        errors.append(api_error("account", "no-change-in-password"))
                    else:
                        warnings.append(api_error("account", "no-change-in-password"))
                else:
                    try:
                        password_result = await update_password(user_id, new_password)
                        if password_result is True:
                            rv["passwordUpdated"] = True
                            rv["passwordChangeReason"] = None
                        else:
                            errors.extend(password_result)
                    except Exception as exc:
                        current_app.logger.error(exc)
                        unexpected_errors = True

    if "locale" in request_data:
        new_locale = request_data["locale"]
        if new_locale != old_user["locale"]:
            try:
                locale_result = await update_locale(user_id, new_locale)
                if locale_result is True:
                    rv["locale"] = new_locale
                else:
                    errors.extend(locale_result)
            except Exception as exc:
                current_app.logger.error(exc)
                unexpected_errors = True

    if "totp" in request_data and request_data["totp"] is None:
        verify_password_result = True
        if updating_self:
            verify_password_result = verify_password()
        if not isinstance(verify_password_result, bool):
            errors.append(verify_password_result)
        elif verify_password_result:
            update_sudo_until = True
            try:
                await disable_totp(user_id)
                rv["totpEnabled"] = False
            except Exception as exc:
                current_app.logger.error(exc)
                unexpected_errors = True

    elif "totp" in request_data and request_data["totp"] is not None:
        if not updating_self:
            errors.append(api_error("account", "cannot-enable-totp-for-others"))
        else:
            verify_password_result = verify_password()
            if not isinstance(verify_password_result, bool):
                errors.append(verify_password_result)
            elif verify_password_result:
                update_sudo_until = True
                try:
                    enable_totp_result = await enable_totp(request_data["totp"])
                    if enable_totp_result is True:
                        rv["totpEnabled"] = True
                    else:
                        errors.append(enable_totp_result)
                except Exception as exc:
                    current_app.logger.error(exc)
                    unexpected_errors = True

    if update_sudo_until:
        now = utcnow()
        sudo_until = (now + current_app.config["SUDO_LIFETIME"]).replace(microsecond=0)
        try:
            await services.db.execute(
                'UPDATE "sessions" SET "sudo_until" = $1 WHERE "id" = $2',
                sudo_until,
                request.cookies[current_app.config["SESSION_COOKIE_NAME"]],
            )
            rv["sudoUntil"] = sudo_until
        except Exception as exc:
            current_app.logger.error(exc)
            unexpected_errors = True

    if unexpected_errors:
        errors.append(api_error("general", "unexpected"))

    if rv:
        run_task(
            account_socket.emit("user_updated", rv, to=user_room(user_id)),
            "emit-user_updated",
        )

    if warnings:
        rv["warnings"] = warnings
    if errors:
        rv["errors"] = errors
        return rv, 400
    return rv


@users.put("/<string:user_id>/icon")
@auth_required()
@rate_limit("put-user-icon", limit=5, seconds=60)
@csrf_protected
@validate_json_schema(put_icon_schema, ignore_non_json=True)
async def put_icon(
    user_id: str,
    *,
    request_data: Optional[PutIconSchema],
) -> ResponseReturnValue:
    class PutIconResponse(TypedDict):
        icon: Optional[str]

    rv: PutIconResponse

    updating_self = user_id == g.user["id"]

    if not updating_self and not has_permission("edit_user"):
        abort(403)

    if request_data is not None and request_data["remove"]:
        current_icon = cast(
            Optional[str],
            await services.db.fetchval(
                'SELECT "icon" FROM "users" WHERE "id" = $1', user_id
            ),
        )
        if current_icon is not None:
            services.s3.remove_object(
                current_app.config["S3_USER_ICONS_BUCKET"], current_icon
            )
        await services.db.execute(
            'UPDATE "users" SET "icon" = NULL WHERE "id" = $1', user_id
        )
        rv = {"icon": None}
        run_task(
            account_socket.emit("user_updated", rv, to=user_room(user_id)),
            "emit-user_updated",
        )
        return rv

    files = (await request.files).getlist("icon")
    if len(files) == 0:
        return await single_error("validation", "user-icon.no-files")
    if len(files) > 1:
        return await single_error("account", "user-icon.multiple-files")

    accepted_formats = current_app.config["ACCEPTED_IMAGE_FORMATS"]
    try:
        image = Image.open(files[0], formats=accepted_formats)
    except UnidentifiedImageError:
        return await single_error(
            "validation",
            "user-icon.invalid-image",
            details=f"Accepted formats: {', '.join(accepted_formats)}",
        )

    min_size = current_app.config["USER_ICON_MIN_SIZE"]
    if image.width < min_size or image.height < min_size:
        return await single_error(
            "validation", "user-icon.too-small", {"minDimension": min_size}
        )

    max_ratio = current_app.config["USER_ICON_MAX_DIMENSION_RATIO"]
    if image.width / image.height > max_ratio:
        return await single_error(
            "validation", "user-icon.too-wide", {"maxRatio": max_ratio}
        )
    elif image.width / image.height > max_ratio:
        return await single_error(
            "validation", "user-icon.too-tall", {"maxRatio": max_ratio}
        )

    icon_size = min(image.width, image.height, current_app.config["USER_ICON_SIZE"])
    cropped = image.resize(
        (icon_size, icon_size), Image.LANCZOS, square_box(image.width, image.height)
    )
    rotated = exif_transpose(cropped)
    rotated.save(stream := BytesIO(), "WEBP", quality=100)
    stream.seek(0)

    protocol = "https" if current_app.config["S3_USE_TLS"] else "http"
    host = current_app.config["S3_ENDPOINT"]
    bucket = current_app.config["S3_USER_ICONS_BUCKET"]
    icon_id = secrets.token_urlsafe(current_app.config["USER_ICON_ID_BYTES"])

    services.s3.put_object(
        bucket,
        f"{icon_id}.webp",
        stream,
        -1,
        content_type="image/webp",
        part_size=current_app.config["S3_PART_SIZE"],
    )
    stream.close()
    await services.db.execute(
        'UPDATE "users" SET "icon" = $1 WHERE "id" = $2', icon_id, user_id
    )
    rv = {"icon": icon_id}
    run_task(
        account_socket.emit("user_updated", rv, to=user_room(user_id)),
        "emit-user_updated",
    )
    return rv
