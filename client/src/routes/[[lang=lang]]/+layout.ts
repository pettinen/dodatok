import { is_language, set_language } from "$i18n";
import { browser } from "$app/environment";
import { config } from "$lib/config";
import type { User } from "$lib/stores";
import { get_preferred_language } from "$lib/utils";

import type { LayoutLoad } from "./$types";

console.log("hello from +layout.ts");

// on the server gets { url, params, data, route, fetch, setHeaders, depends, parent }
// data comes from +layout.server.ts
export const load: LayoutLoad = async ({ data, params }) => {
    console.log("+layout.ts load()", data);

    // TODO
    let user: User | null = null;
    try {
        user = (await (await fetch(`${config.api.host}${config.api.path_prefix}/me`)).json()) as User | null;
    } catch (error) {
        console.error(error);
    }

    let local_language = null;
    if (browser) {
        const maybe_local_language = localStorage.getItem(
            config.storage.local.language.key,
        );
        if (maybe_local_language !== null) {
            if (user) {
                localStorage.setItem(
                    config.storage.local.language.key,
                    user.language,
                );
            }
            if (is_language(maybe_local_language))
                local_language = maybe_local_language;
            else localStorage.removeItem(config.storage.local.language.key);
        }
    }

    const language = get_preferred_language(
        user,
        local_language,
        params.lang ?? null,
    );
    await set_language(language);
    return { language };
};
