import Ajv from "ajv/dist/jtd.js";
import type { JTDSchemaType } from "ajv/dist/jtd.js";

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
  csrfToken?: string;
  errors: APIAlert[];
  warnings?: APIAlert[];
}

const apiErrorResponseSchema: JTDSchemaType<APIErrorResponse> = {
  properties: {
    errors: { elements: apiAlertSchema },
  },
  optionalProperties: {
    csrfToken: { type: "string" },
    warnings: { elements: apiAlertSchema },
  },
};

export const validateAPIErrors = ajv.compile(apiErrorResponseSchema);


export interface UserResponse {
  id: string;
  username: string;
  totpEnabled: boolean;
  passwordChangeReason: string | null;
  icon: string | null;
  locale: string;
  sudoUntil: string | null;
}
const userResponseSchema: JTDSchemaType<UserResponse> = {
  properties: {
    id: { type: "string" },
    username: { type: "string" },
    totpEnabled: { type: "boolean" },
    passwordChangeReason: { type: "string", nullable: true },
    icon: { type: "string", nullable: true },
    locale: { type: "string" },
    sudoUntil: { type: "string", nullable: true },
  },
};
export const validateUserResponse = ajv.compile(userResponseSchema);


interface PutUsernamePasswordResponse {
  errors?: APIAlert[];
  passwordChangeReason?: string | null;
  passwordUpdated?: boolean;
  sudoUntil?: string;
  username?: string;
  warnings?: APIAlert[];
}
const putUsernamePasswordResponseSchema: JTDSchemaType<PutUsernamePasswordResponse> = {
  optionalProperties: {
    errors: { elements: apiAlertSchema },
    passwordChangeReason: { type: "string", nullable: true },
    passwordUpdated: { type: "boolean" },
    sudoUntil: { type: "string" },
    username: { type: "string" },
    warnings: { elements: apiAlertSchema },
  },
};
export const validatePutUsernamePasswordResponse = ajv.compile(putUsernamePasswordResponseSchema);


interface TOTPKeyResponse {
  expires: string;
  key: string;
  qrCode: string;
}
const totpKeyResponseSchema: JTDSchemaType<TOTPKeyResponse> = {
  properties: {
    expires: { type: "string" },
    key: { type: "string" },
    qrCode: { type: "string" },
  },
};
export const validateTOTPKeyResponse = ajv.compile(totpKeyResponseSchema);


interface TOTPResponse {
  sudoUntil?: string;
  totpEnabled: boolean;
}
const totpResponseSchema: JTDSchemaType<TOTPResponse> = {
  properties: {
    totpEnabled: { type: "boolean" },
  },
  optionalProperties: {
    sudoUntil: { type: "string" },
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
  csrfToken: string;
  user: UserResponse;
  warnings?: APIAlert[];
}
const loginResponseSchema: JTDSchemaType<LoginResponse> = {
  properties: {
    csrfToken: { type: "string" },
    user: userResponseSchema,
  },
  optionalProperties: {
    warnings: { elements: apiAlertSchema },
  },
};
export const validateLoginResponse = ajv.compile(loginResponseSchema);


interface CSRFTokenResponse {
  csrfToken: string;
}
const csrfTokenResponseSchema: JTDSchemaType<CSRFTokenResponse> = {
  properties: {
    csrfToken: { type: "string" },
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
