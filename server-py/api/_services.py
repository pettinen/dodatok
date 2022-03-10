from __future__ import annotations

import base64
from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
from typing import Optional, TypedDict

import aioredis
from argon2 import PasswordHasher
from asyncpg import Connection, Record, create_pool
from cryptography.fernet import Fernet as BaseFernet
from minio import Minio
from miscreant.aes.siv import SIV

from api.types import Config, DatabaseConfig


class AESSIV:
    def __init__(self, key: bytes):
        self._siv = SIV(key)

    def encrypt(self, plaintext: str) -> str:
        return base64.urlsafe_b64encode(self._siv.seal(plaintext.encode())).decode()


class Database:
    def __init__(self, config: DatabaseConfig):
        self.config = config
        self.initialized = False

    async def _ensure_initialized(self) -> None:
        if not self.initialized:
            self._pool = await create_pool(**self.config)
            self.initialized = True

    @asynccontextmanager
    async def acquire(self) -> AsyncGenerator[Connection, None]:
        await self._ensure_initialized()
        async with self._pool.acquire() as connection:
            yield connection

    async def execute(self, query: str, *args: object) -> str:
        await self._ensure_initialized()
        return await self._pool.execute(query, *args)

    async def fetch(self, query: str, *args: object) -> list[Record]:
        await self._ensure_initialized()
        return await self._pool.fetch(query, *args)

    async def fetchval(self, query: str, *args: object) -> object:
        await self._ensure_initialized()
        return await self._pool.fetchval(query, *args)

    async def fetchrow(self, query: str, *args: object) -> Optional[Record]:
        await self._ensure_initialized()
        return await self._pool.fetchrow(query, *args)


class Fernet:
    def __init__(self, key: bytes):
        self._fernet = BaseFernet(key)

    def decrypt(self, ciphertext: str) -> str:
        return self._fernet.decrypt(ciphertext.encode()).decode()

    def encrypt(self, plaintext: str) -> str:
        return self._fernet.encrypt(plaintext.encode()).decode()


class Services:
    def __init__(self, config: Config):
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
