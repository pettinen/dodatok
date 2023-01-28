import { Lock as AsyncLock } from "async-await-mutex-lock";
import { setupI18n as setup_i18n, type MessageDescriptor } from "@lingui/core";
import { derived, get, writable } from "svelte/store";

import { browser, dev } from "$app/environment";
import { config } from "$lib/config";
import { readonly_store } from "$lib/utils";

import { default_language, languages } from "helpers";
import type { Language } from "helpers";

export const is_language = (value: string): value is Language =>
    languages.includes(value as Language);

const init_i18n = () => {
    const exclamation_mark_emoji = "\u2757";
    const i18n = setup_i18n({
        missing: (_lang: Language, id: string) => {
            if (dev)
                return `${exclamation_mark_emoji}${id}${exclamation_mark_emoji}`;
            return "";
        },
    });

    const _language = writable<Language>(default_language);
    const language = readonly_store(_language);

    const _tl = writable(
        (id: MessageDescriptor | string, params?: object | null) => {
            if (params) return i18n._(id, params);
            return i18n._(id);
        },
    );
    const tl = readonly_store(_tl);

    const loaded_languages = writable(new Set<Language>());
    const load_lock = new AsyncLock<Language>();

    const loaded = derived(
        loaded_languages,
        ($loaded_languages) => (lang: Language) => $loaded_languages.has(lang),
    );

    interface Messages {
        messages: Record<string, string>;
    }

    const load_language = async (lang: Language) => {
        await load_lock.acquire(lang);
        if (get(loaded_languages).has(lang)) {
            load_lock.release(lang);
            return;
        }
        try {
            console.log('loading language', lang);
            const [{ messages }, plurals] = await Promise.all([
                import(`./${lang}/messages.mjs`) as Promise<Messages>,
                import("make-plural/plurals"),
            ]);
            i18n.loadLocaleData(lang, { plurals: plurals[lang] });
            i18n.load(lang, messages);
            loaded_languages.update((language_set) => language_set.add(lang));
        } finally {
            load_lock.release(lang);
        }
    };

    const set_language = async (lang: Language) => {
        console.log('changing to', lang);
        try {
            await load_language(lang);
        } catch (error) {
            console.error(error); // TODO: handle error
            return;
        }
        i18n.activate(lang);
        _tl.update((x) => x);
        _language.set(lang);
        if (browser)
            localStorage.setItem(config.storage.local.language.key, lang);
    };

    return { language, load_language, loaded, set_language, tl };
};

export const { language, load_language, loaded, set_language, tl } =
    init_i18n();

export { default_language, languages };
export type { Language };
