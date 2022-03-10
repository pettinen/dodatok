from datetime import datetime
from typing import Optional, TypedDict, cast

from socketio import AsyncNamespace

from api import config, services, sio
from api.utils import redis_key, run_task, utcnow


def user_room(user_id: str) -> str:
    return f"user:{user_id}"


class AccountSocket(AsyncNamespace):
    async def on_connect(
        self,
        sid: str,
        _environ: dict[str, object],
        auth: object
    ) -> bool:
        if not isinstance(auth, dict) or not isinstance(auth.get("token"), str):
            return False

        class DBResponse(TypedDict):
            user_id: str
            expires: datetime
            disabled: bool

        data = cast(
            Optional[DBResponse],
            await services.db.fetchrow(
                """
                SELECT
                    "websocket_tokens"."user_id",
                    "websocket_tokens"."expires",
                    "users"."disabled"
                FROM "websocket_tokens" JOIN "users"
                    ON "users"."id" = "websocket_tokens"."user_id"
                WHERE "websocket_tokens"."id" = $1
                """,
                auth["token"],
            ),
        )
        if data is None or data["expires"] < utcnow() or data["disabled"]:
            return False

        self.enter_room(sid, user_room(data["user_id"]))
        return True

    async def on_disconnect(self, sid: str) -> None:
        print("disconnect", sid)


account_socket = AccountSocket("/account")
