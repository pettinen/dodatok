import type { Readable, Writable } from "svelte/store";

import { dev } from "$app/environment";
import { default_language, is_language } from "$i18n";
import type { Language } from "$i18n";
import { errors } from "$lib/alerts";
import type { User } from "$lib/stores";

export const get_preferred_language = (
    user: User | null,
    local: Language | null,
    param: string | null,
): Language => {
    if (user) return user.language;
    if (local && is_language(local)) return local;
    if (param && is_language(param)) return param;
    return default_language;
};

export const log = (...args: unknown[]): void => {
    if (dev) console.error(...args);
};

export const noop = (): void => {
    // Nothing interesting happens.
};

export const unexpected = (...args: unknown[]): void => {
    errors.add_from_api({ source: "general", id: "unexpected" });
    log(...args);
};

export const readonly_store = <T>(
    writable_store: Writable<T>,
): Readable<T> => ({
    subscribe: writable_store.subscribe,
});
