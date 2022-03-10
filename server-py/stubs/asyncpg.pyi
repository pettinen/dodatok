from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
from typing import Optional


Record = dict[str, object]

class BaseConnection:
    async def execute(self, query: str, *args: object) -> str: ...

    async def fetch(self, query: str, *args: object) -> list[Record]: ...

    async def fetchrow(self, query: str, *args: object) -> Optional[Record]: ...

    async def fetchval(self, query: str, *args: object) -> object: ...


class Connection(BaseConnection):
    @asynccontextmanager
    def transaction(self) -> AsyncGenerator[None, None]: ...


class Pool(BaseConnection):
    @asynccontextmanager
    def acquire(self) -> AsyncGenerator[Connection, None]: ...


class UniqueViolationError(Exception):
    pass


async def create_pool(*, database: str) -> Pool: ...
