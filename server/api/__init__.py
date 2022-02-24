import asyncio
from collections.abc import AsyncGenerator
from dataclasses import dataclass
from datetime import datetime
from functools import wraps
from json import JSONEncoder

import aioredis
import asyncpg
import socketio
from argon2 import PasswordHasher
from minio import Minio
from minio.error import S3Error
from quart import Quart, ResponseReturnValue, request
from werkzeug.exceptions import HTTPException

from api.services import Database, Fernet, AESSIV


class CustomJSONEncoder(JSONEncoder):
    def default(self, obj: object) -> object:
        if isinstance(obj, datetime):
            return obj.isoformat()
        return super().default(obj)


class Services:
    def __init__(self, config):
        self.aes_siv = AESSIV(config["AES_SIV_KEY"])
        self.db = Database(config["DB_PARAMS"])
        self.fernet = Fernet(config["FERNET_KEY"])
        self.password_hasher = PasswordHasher()
        self.redis = aioredis.from_url(config["REDIS_URI"])
        self.s3 = Minio(
            config["S3_ENDPOINT"],
            access_key=config["S3_ACCESS_KEY"],
            secret_key=config["S3_SECRET_KEY"],
            secure=config["S3_USE_TLS"],
        )


app = Quart(__name__)
app.config.from_object(f"config.development")

global services
services = Services(app.config)

from api.account import account
from api.auth import auth
from api.users import users

app.register_blueprint(account)
app.register_blueprint(auth)
app.register_blueprint(users)

app.json_encoder = CustomJSONEncoder

for bucket in {app.config["S3_USER_ICONS_BUCKET"]}:
    try:
        services.s3.make_bucket(bucket)
    except S3Error as exc:
        if exc.code != "BucketAlreadyOwnedByYou":
            raise

@app.after_request
def add_cors_header(response):
    response.headers.add("Access-Control-Allow-Credentials", "true")
    response.headers.add("Access-Control-Allow-Headers", "X-CSRF-Token")
    response.headers.add("Access-Control-Allow-Origin", app.config["CLIENT_ORIGIN"])
    response.headers.add("Vary", "Origin")
    return response

@app.errorhandler(HTTPException)
def handle_error(error: HTTPException) -> ResponseReturnValue:
    from api.utils import single_error

    if error.response is not None:
        return error.response, error.code
    if error.code < 500:
        name = error.name.replace(" ", "-").lower()
    else:
        name = "unexpected"
    return single_error("general", name, code=error.code)

global sio
sio = socketio.AsyncServer(
    async_mode="asgi",
    logger=app.logger,
    cors_allowed_origins=app.config["CLIENT_ORIGIN"],
)

from api import websocket

#from hypercorn.middleware import DispatcherMiddleware

#the_app = DispatcherMiddleware({
#    "/socket":
the_app = socketio.ASGIApp(sio, app, socketio_path=app.config["SOCKETIO_PATH"])
#    "/": app,
#})
