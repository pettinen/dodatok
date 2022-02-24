export type JSONValue = null | boolean | number | string | JSONValue[] | Record<string, JSONValue>;
export type JSONObject = Record<string, JSONValue>;

export type MaybePromise<T> = T | Promise<T>;

export interface User {
  id: string;
  username: string;
  totpEnabled: boolean;
  passwordChangeReason: string | null;
  disabled: boolean;
  icon: string | null;
  locale: string;
  sudoUntil: string | null;
}
