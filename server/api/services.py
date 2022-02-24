import base64
from contextlib import asynccontextmanager

import asyncpg
from cryptography.fernet import Fernet as BaseFernet
from miscreant.aes.siv import SIV


class AESSIV:
    def __init__(self, key: bytes):
        self._siv = SIV(key)

    def encrypt(self, plaintext: str) -> str:
        return base64.urlsafe_b64encode(self._siv.seal(plaintext.encode())).decode()


class Database:
    def __init__(self, config):
        self.config = config
        self.initialized = False

    async def ensure_initialized(self):
        if not self.initialized:
            self.pool = await asyncpg.create_pool(**self.config)
            self.initialized = True

    @asynccontextmanager
    async def acquire(self, *args, **kwargs):
        await self.ensure_initialized()
        async with self.pool.acquire(*args, **kwargs) as connection:
            yield connection

    async def execute(self, *args, **kwargs):
        await self.ensure_initialized()
        return await self.pool.execute(*args, **kwargs)

    async def fetchval(self, *args, **kwargs):
        await self.ensure_initialized()
        return await self.pool.fetchval(*args, **kwargs)

    async def fetchrow(self, *args, **kwargs):
        await self.ensure_initialized()
        return await self.pool.fetchrow(*args, **kwargs)


class Fernet:
    def __init__(self, key: bytes):
        self._fernet = BaseFernet(key)

    def decrypt(self, ciphertext: str) -> str:
        return self._fernet.decrypt(ciphertext.encode()).decode()

    def encrypt(self, plaintext: str) -> str:
        return self._fernet.encrypt(plaintext.encode()).decode()
