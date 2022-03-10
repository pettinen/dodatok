import json
from datetime import datetime

from quart.json import JSONEncoder


class CustomJSONEncoder(JSONEncoder):
    def default(self, obj: object) -> object:
        if isinstance(obj, datetime):
            return obj.isoformat()
        return super().default(obj)


def dumps(obj: object, **kwargs: object) -> str:
    return CustomJSONEncoder(**kwargs).encode(obj)


def loads(string: str, **kwargs: object) -> object:
    return json.loads(string, **kwargs)
