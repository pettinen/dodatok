import base64
import os
import secrets
from io import BytesIO

from qrcode import QRCode
from qrcode.image.svg import SvgPathImage
from pyotp import TOTP
from quart import Blueprint, ResponseReturnValue, current_app, g

from api import config, services
from api.utils import (
    auth_required,
    rate_limit,
    single_error,
    try_insert_unique,
    utcnow,
)


account = Blueprint("account", __name__, url_prefix="/account")

def generate_qr_code_svg(data: str) -> str:
    qr = QRCode()
    qr.add_data(data)
    qr.make_image(image_factory=SvgPathImage).save(stream := BytesIO())
    uri = f"data:image/svg+xml;base64,{base64.b64encode(stream.getvalue()).decode()}"
    stream.close()
    return uri


def generate_totp_key() -> str:
    return base64.b32encode(os.urandom(config["TOTP_KEY_BYTES"])).decode()


@account.get("/totp-key")
@auth_required({"totp_key", "username"})
@rate_limit("totp-key", limit=5, seconds=60)
async def totp_key() -> ResponseReturnValue:
    if g.user["totp_key"] is not None:
        return await single_error("account", "totp-already-enabled")

    expires = (utcnow() + current_app.config["NEW_TOTP_KEY_LIFETIME"]).replace(
        microsecond=0
    )

    async def execute_insert_totp_key() -> str:
        key = generate_totp_key()
        await services.db.execute(
            """
            INSERT INTO "new_totp_keys"("user_id", "key", "expires") VALUES ($1, $2, $3)
            ON CONFLICT ("user_id")
                DO UPDATE SET "key" = "excluded"."key", "expires" = "excluded"."expires"
            """,
            g.user["id"],
            services.fernet.encrypt(key),
            expires,
        )
        return key

    key = await try_insert_unique(execute_insert_totp_key, "TOTP key")
    totp_uri = TOTP(
        key,
        digits=current_app.config["TOTP_DIGITS"],
        name=g.user["username"],
        issuer=current_app.config["APP_NAME"],
    ).provisioning_uri()
    qr = generate_qr_code_svg(totp_uri)

    return {
        "expires": expires,
        "key": key,
        "qrCode": qr,
    }


@account.get("/websocket-token")
@auth_required()
@rate_limit("websocket-token", limit=50, seconds=60)
async def websocket_token() -> ResponseReturnValue:
    expires = (utcnow() + current_app.config["WEBSOCKET_TOKEN_LIFETIME"]).replace(
        microsecond=0
    )

    async def execute_insert_websocket_token() -> str:
        token = secrets.token_urlsafe(current_app.config["WEBSOCKET_TOKEN_BYTES"])
        await services.db.execute(
            """
            INSERT INTO "websocket_tokens"("id", "user_id", "expires")
            VALUES ($1, $2, $3)
            """,
            token,
            g.user["id"],
            expires,
        )
        return token

    token = await try_insert_unique(execute_insert_websocket_token, "websocket token")
    return {"token": token}
