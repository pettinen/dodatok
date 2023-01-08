import Ajv from "ajv/dist/jtd.js";
import { get, writable } from "svelte/store";
import type { Readable, Writable } from "svelte/store";
import { locale as localeStore } from "svelte-i18n";

import { dev } from "$app/env";
import { base } from "$app/paths";

import { config } from "$lib/config";
import {
  convertAPIAlert,
  infoMessages,
  errors as errorStore,
  warnings as warningStore,
} from "$lib/errors";
import type { Alert } from "$lib/errors";
import { validateAPIErrors } from "$lib/types";
import type { JSONObject, UserResponse } from "$lib/types";


export const ajv = new Ajv();

export const log = (...args: unknown[]): void => {
  if (dev)
    console.error(...args);
};

export const unexpected = (...args: unknown[]): void => {
  errorStore.add("general.errors.unexpected");
  log(...args);
};

export const noop = (): void => {
  // Do nothing
};

interface APIFetchOptions {
  body?: FormData | string;
  headers: Record<string, string>;
  method: string;
}

interface APIFetchResponseWithData<T> {
  data: T;
  errors: Alert[] | null;
  warnings: Alert[] | null;
}

interface APIFetchResponseWithoutData {
  data: null;
  errors: Alert[];
  warnings: Alert[] | null;
}

export interface User {
  id: string;
  username: string;
  totpEnabled: boolean;
  passwordChangeReason: string | null;
  icon: string;
  locale: string;
  sudoUntil: string | null;
}

type APIFetchResponse<T> = APIFetchResponseWithData<T> | APIFetchResponseWithoutData;

export const apiFetch = async <T>(
  url: string,
  validate: (data: unknown) => data is T,
  method = "GET",
  body: FormData | JSONObject | null = null,
): Promise<APIFetchResponse<T>> => {
  const errorReturn = (errorID: string): APIFetchResponse<T> => {
    errorStore.add(errorID);
    return {
      data: null,
      errors: ["validation.errors.invalid-data"],
      warnings: null,
    };
  };

  if (!url.startsWith("/")) {
    log(new Error("invalid API fetch URL"));
    return errorReturn("general.errors.unexpected");
  }

  const options: APIFetchOptions = { headers: {}, method };
  if (body) {
    if (body instanceof FormData) {
      options.body = body;
    } else {
      options.body = JSON.stringify(body);
      options.headers["Content-Type"] = "application/json";
    }
  }
  if (method !== "GET") {
    const csrfToken = localStorage.getItem(config.csrfTokenStorageKey);
    if (csrfToken !== null)
      options.headers[config.csrfTokenHeader] = csrfToken;
  }

  let response: Response;
  try {
    response = await fetch(`${base}/api${url}`, options);
  } catch (error) {
    log(error);
    return errorReturn("general.errors.could-not-connect-to-server");
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch (error) {
    log(error);
    return errorReturn("validation.errors.invalid-data");
  }

  if (validate(data)) {
    return { data, errors: null, warnings: null };
  } else if (validateAPIErrors(data)) {
    const errors = data.errors.map(convertAPIAlert("error"));
    errorStore.add(...errors);
    const warnings = data.warnings ? data.warnings.map(convertAPIAlert("warning")) : null;
    if (warnings)
      warningStore.add(...warnings);
    if (data.csrf_token)
      localStorage.setItem(config.csrfTokenStorageKey, data.csrf_token);
    return { data: null, errors, warnings };
  }

  return errorReturn("validation.errors.invalid-data");
};

interface ModalStore extends Readable<string[]> {
  pop: () => string | null;
  push: (name: string) => void;
  remove: (name: string) => void;
}

const createModalStore = (): ModalStore => {
  const modalStore: Writable<string[]> = writable([]);

  return {
    pop: (): string | null => {
      const current = get(modalStore);
      if (current.length === 0)
        return null;
      modalStore.update((store) => store.slice(1));
      return current[0];
    },
    push: (name: string): void => {
      modalStore.update((store) => [name, ...store.filter((existing) => existing !== name)]);
    },
    remove: (name: string): void => {
      modalStore.update((store) => store.filter((existing) => existing !== name));
    },
    subscribe: modalStore.subscribe,
  };
};

export const modals = createModalStore();

export const user: Writable<User | null> = writable(null);

export const userIconURL = (iconID: string): string => `${config.baseURLs.userIcon}/${iconID}.webp`;

export const convertAPIUser = (apiUser: UserResponse): User => ({
  id: apiUser.id,
  username: apiUser.username,
  totpEnabled: apiUser.totp_enabled,
  passwordChangeReason: apiUser.password_change_reason,
  icon: apiUser.icon ? userIconURL(apiUser.icon) : config.defaultUserIcon,
  locale: apiUser.locale,
  sudoUntil: apiUser.sudo_until,
});


type UserUpdate = Partial<UserResponse & { passwordUpdated: boolean }>;

export const updateUser = (data: UserUpdate): void => {
  const {
    icon,
    locale,
    password_change_reason: passwordChangeReason,
    passwordUpdated,
    sudo_until: sudoUntil,
    totp_enabled: totpEnabled,
    username,
  } = data;

  if (icon) {
    user.update((oldUser) => oldUser && { ...oldUser, icon: userIconURL(icon) });
    infoMessages.showTimed("account.icon-updated");
  } else if (icon === null) {
    user.update((oldUser) => oldUser && { ...oldUser, icon: config.defaultUserIcon });
    infoMessages.showTimed("account.icon-removed");
  }

  if (locale) {
    user.update((oldUser) => oldUser && { ...oldUser, locale });
    void localeStore.set(locale);
  }

  if (passwordUpdated)
    infoMessages.showTimed("account.password-changed");

  if (passwordChangeReason || passwordChangeReason === null)
    user.update((oldUser) => oldUser && { ...oldUser, passwordChangeReason });

  if (sudoUntil)
    user.update((oldUser) => oldUser && { ...oldUser, sudoUntil });

  if (typeof totpEnabled === "boolean") {
    user.update((oldUser) => oldUser && { ...oldUser, totpEnabled });
    infoMessages.showTimed(totpEnabled ? "account.totp-enabled" : "account.totp-disabled");
  }

  if (username) {
    user.update((oldUser) => oldUser && { ...oldUser, username });
    infoMessages.showTimed("account.username-changed");
  }
};
