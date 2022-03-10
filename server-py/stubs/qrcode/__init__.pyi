from typing import Type

from PIL.Image import Image

from .image.svg import SvgPathImage


class QRCode:
    def add_data(self, data: str) -> None: ...

    def make_image(self, *, image_factory: Type[SvgPathImage]) -> Image: ...
