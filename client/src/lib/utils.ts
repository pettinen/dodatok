import { get, derived, writable } from "svelte/store";
import type { Writable } from "svelte/store";

import { dev } from "$app/env";
import { base } from "$app/paths";
import { session } from "$app/stores";

import { cacheURL } from "$lib/cache";
import { config } from "$lib/config";
import { errors, warnings } from "$lib/errors";
import type { APIAlert } from "$lib/errors";
import type { JSONObject, JSONValue, User } from "$lib/types";


interface APIResponse {
  [key: string]: JSONValue;
  csrfToken?: string;
  errors?: APIAlert[];
  warnings?: APIAlert[];
}

interface APIFetchOptions {
  body?: FormData | string;
  headers: Record<string, string>;
  method: string;
}

const isAPIResponse = (data: unknown): data is APIResponse => {
  if (typeof data !== "object" || data === null)
    return false;
  if ("csrfToken" in data && typeof data.csrfToken !== "string")
    return false;
  if ("errors" in data && !Array.isArray(data.errors))
    return false;
  if ("warnings" in data && !Array.isArray(data.warnings))
    return false;
  return true;
};

export const apiFetch = async (
  url: string,
  method = "GET",
  body: FormData | JSONObject | null = null,
): Promise<APIResponse> => {
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
    const csrfToken = localStorage.getItem(config.csrfTokenField);
    if (csrfToken !== null)
      options.headers["X-CSRF-Token"] = csrfToken;
  }

  let response: Response;
  try {
    response = await fetch(`${base}/api${url}`, options);
  } catch (error: unknown) {
    errors.add("general.errors.could-not-connect-to-server");
    throw error instanceof Error ? error : new Error(String(error));
  }

  let data: APIResponse;
  try {
    const _data: unknown = await response.json();
    if (isAPIResponse(_data))
      data = _data;
    else
      throw new Error("Unexpected API response data");
  } catch (error: unknown) {
    errors.add("validation.errors.invalid-data");
    throw error instanceof Error ? error : new Error(String(error));
  }
  if (data.csrfToken)
    localStorage.setItem(config.csrfTokenField, data.csrfToken);
  if (data.errors)
    errors.addFromAPI(data.errors);
  if (data.warnings)
    warnings.addFromAPI(data.warnings);
  return data;
};

export const catchError = (error: unknown): void => {
  errors.add("general.errors.unexpected");
  if (dev)
    console.error(error);
};

interface ModalStore extends Readable<string[]> {
  pop: () => string | null;
  push: (name: string) => void;
}

const createModalStore = (): ModalStore => {
  const modalStore: Writable<string[]> = writable([]);

  return {
    pop: (): string | null => {
      const current = get(modalStore);
      if (current.length === 0)
        return null;
      const rv = current[0];
      modalStore.update((store) => store.slice(1));
      return rv;
    },
    push: (name: string): void => {
      modalStore.update((store) => [name, ...store.filter((existing) => existing !== name)]);
    },
    remove: (name: string): void => {
      modalStore.update((store) => store.filter((existing) => existing !== name));
    },
    subscribe: modalStore.subscribe,
  }
}

export const modals = createModalStore();

export const userIconURL = (iconID: string): string => `${config.baseURLs.userIcon}/${iconID}.webp`;

type APIUser = User;

export const convertAPIUser = (apiUser: APIUser | null): User | null => {
  if (!apiUser)
    return null;
  return {
    ...apiUser,
    icon: apiUser.icon && userIconURL(apiUser.icon),
  };
};

export const userIcon = derived([cacheURL, session], ([$cacheURL, $session]) => {
  const defaultIcon = $cacheURL(config.defaultUserIcon);
  if ($session.user?.icon)
    return $cacheURL($session.user.icon) ?? defaultIcon;
  return defaultIcon;
});
