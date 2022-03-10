from quart import current_app

from api.utils import Alert, api_error


def validate_locale(value: str) -> list[Alert]:
    if value not in current_app.config["LOCALES"]:
        return [api_error("validation", "locale.invalid")]
    return []


def validate_password(value: str, field_name: str) -> list[Alert]:
    if not value:
        return [api_error("validation", f"{field_name}.empty")]
    if len(value) < current_app.config["PASSWORD_MIN_LENGTH"]:
        return [
            api_error(
                "validation",
                f"{field_name}.too-short",
                {"minLength": current_app.config["PASSWORD_MIN_LENGTH"]},
            )
        ]
    if len(value) > current_app.config["PASSWORD_MAX_LENGTH"]:
        return [
            api_error(
                "validation",
                f"{field_name}.too-long",
                {"maxLength": current_app.config["PASSWORD_MAX_LENGTH"]},
            )
        ]
    return []


def validate_username(value: str, field_name: str) -> list[Alert]:
    if not value:
        return [api_error("validation", f"{field_name}.empty")]
    if len(value) < current_app.config["USERNAME_MIN_LENGTH"]:
        return [
            api_error(
                "validation",
                f"{field_name}.too-short",
                {"minLength": current_app.config["USERNAME_MIN_LENGTH"]},
            )
        ]
    if len(value) > current_app.config["USERNAME_MAX_LENGTH"]:
        return [
            api_error(
                "validation",
                f"{field_name}.too-long",
                {"maxLength": current_app.config["USERNAME_MAX_LENGTH"]},
            )
        ]
    return []
