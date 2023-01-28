import type { Readable, Writable } from "svelte/store";

import { dev } from "$app/environment";
import { default_language, is_language } from "$i18n";
import type { Language } from "$i18n";
import { errors } from "$lib/alerts";
import type { User } from "$lib/stores";
import { DateTime } from "luxon";

export const log_error = (...args: unknown[]): void => {
    if (dev) console.error(...args);
};

export const date_from_iso = (string: string): DateTime | null => {
    const date = DateTime.fromISO(string);
    if (date.isValid)
        return date;
    log_error("invalid date", { reason: date.invalidReason, explanation: date.invalidExplanation });
    return null;
};

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

export const noop = (): void => {
    // Nothing interesting happens.
};

export const unexpected = (...args: unknown[]): void => {
    errors.add_from_api({ source: "general", id: "unexpected" });
    log_error(...args);
};

export const readonly_store = <T>(
    writable_store: Writable<T>,
): Readable<T> => ({
    subscribe: writable_store.subscribe,
});
