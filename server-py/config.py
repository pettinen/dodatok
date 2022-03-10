import logging
import os
from datetime import timedelta


class Config:
    APP_NAME = "Simple backend"
    LOCALES = ["en-US", "fi-FI"]

    CLIENT_ORIGIN = "http://kotori.lab:55555"
    CORS = True

    SOCKETIO_PATH = "socket"

    COOKIE_PATH = "/"
    COOKIE_SAMESITE = "Lax"
    COOKIE_SECURE = True
    PERSISTENT_COOKIE_MAX_AGE = 2**31 - 1
    CSRF_COOKIE_NAME = "csrfToken"
    REMEMBER_TOKEN_COOKIE_NAME = "rememberToken"
    SESSION_COOKIE_NAME = "session"

    CSRF_TOKEN_BYTES = 32
    REMEMBER_TOKEN_ID_BYTES = 32
    REMEMBER_TOKEN_SECRET_BYTES = 32
    SESSION_ID_BYTES = 32
    TOTP_KEY_BYTES = 40
    TOTP_DIGITS = 6
    TOTP_VALID_WINDOW = 1
    USER_ID_BYTES = 32
    USER_ICON_ID_BYTES = 32
    WEBSOCKET_TOKEN_BYTES = 32

    SESSION_LIFETIME = timedelta(days=360)
    SUDO_LIFETIME = timedelta(days=1)
    NEW_TOTP_KEY_LIFETIME = timedelta(minutes=10)
    WEBSOCKET_TOKEN_LIFETIME = timedelta(minutes=1)

    DB_PARAMS = {"database": "simple_backend"}
    DB_UNIQUE_RETRIES = 5

    REDIS_URI = "unix:///run/redis/redis.sock?db=0"
    REDIS_KEY_SEPARATOR = "|"

    S3_USE_TLS = True
    S3_PART_SIZE = 5 * 1024**2
    S3_FILES_BUCKET = "files"
    S3_USER_ICONS_BUCKET = "user-icons"

    USERNAME_MIN_LENGTH = 1
    USERNAME_MAX_LENGTH = 20
    PASSWORD_MIN_LENGTH = 8
    PASSWORD_MAX_LENGTH = 1000

    ACCEPTED_IMAGE_FORMATS = ["JPEG", "PNG", "WEBP"]
    USER_ICON_MAX_DIMENSION_RATIO = 3
    USER_ICON_SIZE = 512
    USER_ICON_MIN_SIZE = 20


class development(Config):
    ENV = "development"
    LOG_LEVEL = logging.DEBUG
    COOKIE_SECURE = False
    AES_SIV_KEY = b"\x00" * 64
    FERNET_KEY = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="

    S3_ENDPOINT = "kotori.lab:9000"
    S3_ACCESS_KEY = "simple-backend"
    S3_SECRET_KEY = "simple-backend"
    S3_USE_TLS = False


class production(Config):
    ENV = "production"

    def __init__(self):
        self.FERNET_KEY = os.environ["FERNET_KEY"]
