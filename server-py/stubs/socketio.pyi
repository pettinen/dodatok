from logging import Logger
from types import ModuleType
from typing import Literal

from quart import Quart


class AsyncNamespace:
    def __init__(self, namespace: str): ...

    async def emit(self, event: str, data: object, *, to: str) -> None: ...

    def enter_room(self, sid: str, room: str) -> None: ...


class AsyncServer:
    def __init__(
        self,
        *,
        async_mode: Literal["asgi"],
        cors_allowed_origins: str,
        json: ModuleType,
        logger: Logger,
        engineio_logger: Logger,
    ): ...

    def register_namespace(self, namespace_handler: AsyncNamespace) -> None: ...

class ASGIApp:
    def __init__(
        self,
        socketio_server: AsyncServer,
        other_asgi_app: Quart,
        socketio_path: str
    ): ...
