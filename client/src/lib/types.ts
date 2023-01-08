import Ajv from "ajv/dist/jtd.js";
import type { JTDSchemaType } from "ajv/dist/jtd.js";1

import type { APIAlert } from "$lib/errors";


const ajv = new Ajv();

export type JSONPrimitive = boolean | number | string | null;
export type JSONValue = JSONPrimitive | JSONValue[] | { [key: string]: JSONValue };
export type JSONObject = Record<string, JSONValue>;

export type MaybePromise<T> = Promise<T> | T;


const apiAlertSchema: JTDSchemaType<APIAlert> = {
  properties: {
    id: { type: "string" },
    source: { type: "string" },
  },
  optionalProperties: {
    details: { type: "string" },
    params: { values: { type: "int32" } },
  },
};

interface APIErrorResponse {
  csrf_token?: string;
  errors: APIAlert[];
  warnings?: APIAlert[];
}

const apiErrorResponseSchema: JTDSchemaType<APIErrorResponse> = {
  properties: {
    errors: { elements: apiAlertSchema },
  },
  optionalProperties: {
    csrf_token: { type: "string" },
    warnings: { elements: apiAlertSchema },
  },
};

export const validateAPIErrors = ajv.compile(apiErrorResponseSchema);


export interface UserResponse {
  id: string;
  username: string;
  totp_enabled: boolean;
  password_change_reason: string | null;
  icon: string | null;
  locale: string;
  sudo_until: string | null;
}
const userResponseSchema: JTDSchemaType<UserResponse> = {
  properties: {
    id: { type: "string" },
    username: { type: "string" },
    totp_enabled: { type: "boolean" },
    password_change_reason: { type: "string", nullable: true },
    icon: { type: "string", nullable: true },
    locale: { type: "string" },
    sudo_until: { type: "string", nullable: true },
  },
};
export const validateUserResponse = ajv.compile(userResponseSchema);


interface PutUsernamePasswordResponse {
  errors?: APIAlert[];
  password_change_reason?: string | null;
  password_updated?: boolean;
  sudo_until?: string;
  username?: string;
  warnings?: APIAlert[];
}
const putUsernamePasswordResponseSchema: JTDSchemaType<PutUsernamePasswordResponse> = {
  optionalProperties: {
    errors: { elements: apiAlertSchema },
    password_change_reason: { type: "string", nullable: true },
    password_updated: { type: "boolean" },
    sudo_until: { type: "string" },
    username: { type: "string" },
    warnings: { elements: apiAlertSchema },
  },
};
export const validatePutUsernamePasswordResponse = ajv.compile(putUsernamePasswordResponseSchema);


interface TOTPKeyResponse {
  expires: string;
  key: string;
  qr_code: string;
}
const totpKeyResponseSchema: JTDSchemaType<TOTPKeyResponse> = {
  properties: {
    expires: { type: "string" },
    key: { type: "string" },
    qr_code: { type: "string" },
  },
};
export const validateTOTPKeyResponse = ajv.compile(totpKeyResponseSchema);


interface TOTPResponse {
  sudo_until?: string;
  totp_enabled: boolean;
}
const totpResponseSchema: JTDSchemaType<TOTPResponse> = {
  properties: {
    totp_enabled: { type: "boolean" },
  },
  optionalProperties: {
    sudo_until: { type: "string" },
  },
  additionalProperties: false,
};
export const validateTOTPResponse = ajv.compile(totpResponseSchema);


interface UserIconResponse {
  icon: string | null;
}
const userIconResponseSchema: JTDSchemaType<UserIconResponse> = {
  properties: {
    icon: { type: "string", nullable: true },
  },
};
export const validateUserIconResponse = ajv.compile(userIconResponseSchema);


interface PutUserLocaleResponse {
  locale: string;
}
const putUserLocaleResponseSchema: JTDSchemaType<PutUserLocaleResponse> = {
  properties: {
    locale: { type: "string" },
  },
};
export const validatePutUserLocaleResponse = ajv.compile(putUserLocaleResponseSchema);


interface UsernameAvailableResponse {
  available: boolean;
}
const usernameAvailableResponseSchema: JTDSchemaType<UsernameAvailableResponse> = {
  properties: {
    available: { type: "boolean" },
  },
};
export const validateUsernameAvailableResponse = ajv.compile(usernameAvailableResponseSchema);


interface LoginResponse {
  csrf_token: string;
  user: UserResponse;
  warnings?: APIAlert[];
}
const loginResponseSchema: JTDSchemaType<LoginResponse> = {
  properties: {
    csrf_token: { type: "string" },
    user: userResponseSchema,
  },
  optionalProperties: {
    warnings: { elements: apiAlertSchema },
  },
};
export const validateLoginResponse = ajv.compile(loginResponseSchema);


interface CSRFTokenResponse {
  csrf_token: string;
}
const csrfTokenResponseSchema: JTDSchemaType<CSRFTokenResponse> = {
  properties: {
    csrf_token: { type: "string" },
  },
};
export const validateCSRFTokenResponse = ajv.compile(csrfTokenResponseSchema);


interface WebSocketTokenResponse {
  token: string;
}
const webSocketTokenResponseSchema: JTDSchemaType<WebSocketTokenResponse> = {
  properties: {
    token: { type: "string" },
  },
};
export const validateWebSocketTokenResponse = ajv.compile(webSocketTokenResponseSchema);


export const isJSONObject = (object: unknown): object is JSONObject =>
  Boolean(object && typeof object === "object" && !Array.isArray(object));
