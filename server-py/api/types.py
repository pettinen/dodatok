from collections.abc import Iterable, Sequence
from datetime import timedelta
from typing import Literal, TypedDict, Union

import quart
import werkzeug


class DatabaseConfig(TypedDict, total=False):
    database: str


class Config(TypedDict):
    APP_NAME: str
    ENV: Literal["development", "production"]
    LOG_LEVEL: int
    LOCALES: Sequence[str]

    CLIENT_ORIGIN: str
    CORS: bool

    AES_SIV_KEY: bytes
    FERNET_KEY: bytes

    SOCKETIO_PATH: str

    COOKIE_PATH: str
    COOKIE_SAMESITE: Literal["Lax", "Strict"]
    COOKIE_SECURE: bool
    PERSISTENT_COOKIE_MAX_AGE: int
    CSRF_COOKIE_NAME: str
    REMEMBER_TOKEN_COOKIE_NAME: str
    SESSION_COOKIE_NAME: str

    CSRF_TOKEN_BYTES: int
    REMEMBER_TOKEN_ID_BYTES: int
    REMEMBER_TOKEN_SECRET_BYTES: int
    SESSION_ID_BYTES: int
    TOTP_KEY_BYTES: int
    TOTP_DIGITS: int
    TOTP_VALID_WINDOW: int
    USER_ID_BYTES: int
    USER_ICON_ID_BYTES: int
    WEBSOCKET_TOKEN_BYTES: int

    SESSION_LIFETIME: timedelta
    SUDO_LIFETIME: timedelta
    NEW_TOTP_KEY_LIFETIME: timedelta
    WEBSOCKET_TOKEN_LIFETIME: timedelta

    DB_PARAMS: DatabaseConfig
    DB_UNIQUE_RETRIES: int

    REDIS_URI: str
    REDIS_KEY_SEPARATOR: str

    S3_ENDPOINT: str
    S3_ACCESS_KEY: str
    S3_SECRET_KEY: str
    S3_USE_TLS: bool
    S3_PART_SIZE: int
    S3_FILES_BUCKET: str
    S3_USER_ICONS_BUCKET: str

    USERNAME_MIN_LENGTH: int
    USERNAME_MAX_LENGTH: int
    PASSWORD_MIN_LENGTH: int
    PASSWORD_MAX_LENGTH: int

    ACCEPTED_IMAGE_FORMATS: Iterable[str]
    USER_ICON_MAX_DIMENSION_RATIO: int
    USER_ICON_SIZE: int
    USER_ICON_MIN_SIZE: int


Response = Union[quart.Response, werkzeug.Response]
