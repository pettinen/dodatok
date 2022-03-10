from collections.abc import Iterable, Iterator
from typing import BinaryIO

from .deleteobjects import DeleteObject


class Object:
    object_name: str


class Minio:
    def __init__(
        self, endpoint: str, *, access_key: str, secret_key: str, secure: bool
    ): ...

    def fput_object(
        self, bucket_name: str, object_name: str, file_path: str, *, content_type: str
    ) -> None: ...

    def list_objects(
        self, bucket_name: str, *, recursive: bool
    ) -> Iterator[Object]: ...

    def make_bucket(self, bucket_name: str) -> None: ...

    def put_object(
        self,
        bucket_name: str,
        object_name: str,
        stream: BinaryIO,
        length: int,
        *,
        content_type: str,
        part_size: int
    ) -> None: ...

    def remove_bucket(self, bucket_name: str) -> None: ...

    def remove_object(self, bucket_name: str, object_name: str) -> None: ...

    def remove_objects(
        self, bucket_name: str, delete_objects: Iterable[DeleteObject]
    ) -> Iterator[None]: ...

    def set_bucket_policy(self, bucket_name: str, policy: str) -> None: ...
