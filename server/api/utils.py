import hashlib
import secrets
from asyncio import CancelledError, Task, create_task
from collections.abc import Callable, Coroutine, Iterable
from datetime import datetime, timezone
from functools import wraps
from typing import NoReturn, Optional, TypeVar, TypedDict

from asyncpg import UniqueViolationError
from jsonschema import ValidationError, validate
from quart import Response, ResponseReturnValue, current_app, g, make_response, request
from werkzeug.exceptions import BadRequest

from api import services


class UnexpectedError(Exception):
    pass


class APIErrorBase(TypedDict):
    id: str
    source: str


APIErrorValue = int

Route = Callable[..., ResponseReturnValue]


class APIError(APIErrorBase, total=False):
    details: str
    values: dict[str, APIErrorValue]


class ErrorsResponse(TypedDict):
    errors: list[APIError]


def api_error(
    source: str,
    id: str,
    values: Optional[dict[str, APIErrorValue]] = None,
    *,
    details: Optional[str] = None,
) -> APIError:
    error: APIError = {"id": id, "source": source}
    if details is not None:
        error["details"] = details
    if values is not None:
        error["values"] = values
    return error


def single_error(
    source: str,
    id: str,
    values: Optional[dict[str, APIErrorValue]] = None,
    *,
    code: Optional[int] = 400,
    details: Optional[str] = None,
) -> ErrorsResponse:
    response = {"errors": [api_error(source, id, values=values, details=details)]}
    if code is None:
        return response
    return response, code


def bad_request(
    source: str,
    id: str,
    values: Optional[dict[str, APIErrorValue]] = None,
    *,
    details: Optional[str] = None,
):
    response = single_error(source, id, values, details=details, code=None)
    raise BadRequest(response=response)


def camel_case(snake_case: str) -> str:
    return "".join(
        word.title() if i != 0 else word for i, word in enumerate(snake_case.split("_"))
    )


def auth_required(
    extra_fields: Iterable[str] = set(),
    *,
    permissions: bool = False,
) -> Callable[[Route], Route]:
    def decorator(function: Route) -> Route:
        @wraps(function)
        async def wrapper(*args: object, **kwargs: object) -> ResponseReturnValue:
            if current_app.config["SESSION_COOKIE_NAME"] not in request.cookies:
                bad_request("auth", "not-logged-in")

            session_id = request.cookies[current_app.config["SESSION_COOKIE_NAME"]]

            default_fields = {
                '"users"."id"',
                '"users"."disabled"',
                '"sessions"."csrf_token"',
                '"sessions"."expires"',
                '"sessions"."sudo_until"',
            }
            fields = default_fields | {f'"users"."{field}"' for field in extra_fields}

            data = await services.db.fetchrow(
                f"""
                SELECT {", ".join(fields)}
                FROM "users" JOIN "sessions" ON "sessions"."user_id" = "users"."id"
                WHERE "sessions"."id" = $1
                """,
                session_id,
            )

            if data is None:
                raise BadRequest(
                    response=await delete_session(single_error("auth", "not-logged-in"))
                )
            if data["disabled"]:
                raise BadRequest(
                    response=await delete_auth_cookies(
                        single_error("auth", "account-disabled")
                    )
                )
            if data["expires"] < utcnow():
                run_task(
                    services.db.execute(
                        'DELETE FROM "sessions" WHERE "id" = $1', session_id
                    ),
                    "delete-expired-session",
                )
                raise BadRequest(
                    response=await delete_session(
                        single_error("auth", "session-expired")
                    )
                )

            if permissions:
                permission_list = await services.db.fetch(
                    """
                    SELECT "permissions"."permission"
                    FROM "permissions"
                        JOIN "users" ON "permissions"."user_id" = "users"."id"
                    WHERE "users"."id" = $1
                    """,
                    data["id"],
                )
                permission_set = {record["permission"] for record in permission_list}

            g.user = {
                "id": data["id"],
                "sudo_until": data["sudo_until"],
            }
            if permissions:
                print(permission_set)
                g.user["permissions"] = permission_set
            g.csrf_token = data["csrf_token"]

            for field in extra_fields:
                g.user[field] = data[field]
            return await function(*args, **kwargs)

        return wrapper

    return decorator


def unauthenticated_only(function: Route) -> Route:
    @wraps(function)
    async def wrapper(*args: object, **kwargs: object) -> ResponseReturnValue:
        if current_app.config["SESSION_COOKIE_NAME"] not in request.cookies:
            return await function(*args, **kwargs)

        session_id = request.cookies[current_app.config["SESSION_COOKIE_NAME"]]

        authenticated = await services.db.fetchval(
            'SELECT count(*) > 0 FROM "sessions" WHERE "id" = $1 AND now() < "expires"',
            session_id,
        )

        if authenticated:
            bad_request("auth", "already-authenticated")
        return await function(*args, **kwargs)

    return wrapper


async def csrf_error(error_id: str, csrf_token: Optional[str] = None):
    if csrf_token is None:
        csrf_token = secrets.token_urlsafe(current_app.config["CSRF_TOKEN_BYTES"])

    response_dict = single_error("csrf", error_id, code=None)
    response_dict["csrfToken"] = csrf_token
    response = await make_response(response_dict)
    response.set_cookie(
        current_app.config["CSRF_COOKIE_NAME"],
        csrf_token,
        httponly=True,
        path=current_app.config["COOKIE_PATH"],
        samesite=current_app.config["COOKIE_SAMESITE"],
        secure=current_app.config["COOKIE_SECURE"],
    )
    print("setting to", csrf_token)
    print(response)
    raise BadRequest(response=response)


def csrf_protected(function: Route) -> Route:
    @wraps(function)
    async def wrapper(*args: object, **kwargs: object) -> ResponseReturnValue:
        session_csrf_token = getattr(g, "csrf_token", None)
        csrf_headers = request.headers.getlist("X-CSRF-Token")
        if not csrf_headers:
            await csrf_error("missing-csrf-header", session_csrf_token)
        if len(csrf_headers) > 1:
            bad_request("csrf", "multiple-csrf-headers")

        csrf_header = csrf_headers[0]
        if session_csrf_token is None:
            # Unauthenticated; protect against login CSRF
            if current_app.config["CSRF_COOKIE_NAME"] not in request.cookies:
                await csrf_error("missing-csrf-cookie")
            if request.cookies[current_app.config["CSRF_COOKIE_NAME"]] != csrf_header:
                print(request.cookies[current_app.config["CSRF_COOKIE_NAME"]], csrf_header)
                await csrf_error("invalid-csrf-token")
        elif csrf_header != session_csrf_token:
            # g.csrf_token set by auth_required
            await csrf_error("invalid-csrf-token", session_csrf_token)
        return await function(*args, **kwargs)

    return wrapper


async def delete_cookies(
    response: ResponseReturnValue,
    cookies: Iterable[str],
) -> Response:
    response = await make_response(response)
    for cookie in cookies:
        response.delete_cookie(
            cookie,
            httponly=True,
            path=current_app.config["COOKIE_PATH"],
            samesite=current_app.config["COOKIE_SAMESITE"],
            secure=current_app.config["COOKIE_SECURE"],
        )
    return response


async def delete_auth_cookies(response: ResponseReturnValue) -> Response:
    return await delete_cookies(
        response,
        {
            current_app.config["REMEMBER_TOKEN_COOKIE_NAME"],
            current_app.config["SESSION_COOKIE_NAME"],
        },
    )


async def delete_session(response: ResponseReturnValue) -> Response:
    return await delete_cookies(response, {current_app.config["SESSION_COOKIE_NAME"]})


async def has_permission(permission: str) -> bool:
    if getattr(g, "user", None) is None:
        return False
    if "permissions" in g.user:
        return permission in g.user["permissions"]
    return await services.db.fetchval(
        """
        SELECT count(*) > 0 FROM "permissions"
        WHERE "user_id" = $1 AND "permission" = $2
        """,
        g.user["id"],
        permission,
    )


def rate_limit(endpoint: str, *, limit: int, seconds: int) -> Callable[[Route], Route]:
    def decorator(function: Route) -> Route:
        @wraps(function)
        async def wrapper(*args: object, **kwargs: object) -> ResponseReturnValue:
            if not await has_permission("ignore_rate_limits"):
                encrypted_ip = services.aes_siv.encrypt(request.remote_addr)
                redis_key = f"rate-limit:{endpoint}:{encrypted_ip}"
                async with services.redis.pipeline() as pipe:
                    count, _ = (
                        await pipe.incr(redis_key).expire(redis_key, seconds).execute()
                    )
                if count > limit:
                    return single_error(
                        "general",
                        "too-many-requests",
                        {"limit": limit, "windowSeconds": seconds},
                        code=429,
                    )
            return await function(*args, **kwargs)

        return wrapper

    return decorator


def run_task(coroutine: Coroutine[None, None, None], name: str) -> None:
    def log(task: Task[None]) -> None:
        exc = task.exception()
        if exc is not None and not isinstance(exc, CancelledError):
            current_app.logger.warning(
                f"{type(exc).__name__} was raised from task {task.get_name()!r}: {exc}"
            )

    task = create_task(coroutine, name=name)
    task.add_done_callback(log)


def sha3_256(value: str) -> str:
    return hashlib.sha3_256(value.encode()).hexdigest()


def square_box(width: int, height: int) -> tuple[float, float, float, float]:
    if height > width:
        top = (height - width) / 2
        return 0.0, top, float(width), top + width

    left = (width - height) / 2
    return left, 0.0, left + height, float(height)


T = TypeVar("T")

async def try_insert_unique(
    function: Callable[[], Coroutine[None, None, T]],
    name: str,
) -> T:
    retries = current_app.config["DB_UNIQUE_RETRIES"]
    for _ in range(retries):
        try:
            return await function()
            break
        except UniqueViolationError:
            continue
    else:
        raise UnexpectedError(f"Could not insert {name} in {retries} attempts")


def utcnow() -> datetime:
    return datetime.utcnow().replace(tzinfo=timezone.utc)


def validate_json_schema(
    schema: dict[str, object],
    *,
    ignore_non_json: bool = False,
) -> Callable[[Route], Route]:
    def decorator(function: Route) -> Route:
        @wraps(function)
        async def wrapper(*args: object, **kwargs: object) -> ResponseReturnValue:
            try:
                data = await request.get_json(force=True)
            except BadRequest:
                if ignore_non_json:
                    return await function(*args, **kwargs, request_data=None)
                bad_request("validation", "invalid-data", details="Invalid JSON")
            try:
                validate(data, schema)
            except ValidationError as exc:
                bad_request("validation", "invalid-data", details=str(exc))
            return await function(*args, **kwargs, request_data=data)

        return wrapper

    return decorator
