import { createIs, createEquals } from "typia";

import type { Language } from "./i18n";

export { default_language, languages } from "./i18n";
export type { Language };

export type AlertParams = Record<string, number>;

export interface Alert {
    source: string;
    id: string;
    params?: AlertParams;
}

interface ResponseBase {
    success: boolean;
    warnings?: Alert[];
    csrf_token?: string;
    sudo_until?: string;
}

interface ResponseWithCsrfToken {
    csrf_token: string;
}
export const has_csrf_token = createIs<ResponseWithCsrfToken>();

interface SuccessResponse<T> extends ResponseBase {
    success: true;
    data: T;
}

interface ResponseWithWarnings {
    warnings: Alert[];
}
export const has_warnings = createIs<ResponseWithWarnings>();

interface FailureResponse extends ResponseBase {
    success: false;
    errors: Alert[];
}
export const is_failure = createEquals<FailureResponse>();

type PasswordChangeReason = "found-in-breach";

export interface User {
    id: string;
    username: string;
    totp_enabled: boolean;
    password_change_reason: PasswordChangeReason | null;
    icon: string | null;
    language: Language;
}
interface UserResponse {
    user: User | null;
    sudo_until?: string;
}
export const is_user_response = createEquals<SuccessResponse<UserResponse>>();

interface TotpKey {
    expires: string;
    key: string;
    qr_code: string;
}
export const is_totp_key_response = createEquals<SuccessResponse<TotpKey>>();

interface UsernameAvailable {
    available: boolean;
}
export const is_username_available_response =
    createEquals<SuccessResponse<UsernameAvailable>>();

interface WebsocketToken {
    token: string;
}
export const is_websocket_token_response =
    createEquals<SuccessResponse<WebsocketToken>>();
