import { DateTime } from "luxon";

import { is_user_response } from "helpers";

import { is_language, set_language } from "$i18n";
import { browser } from "$app/environment";
import { config } from "$lib/config";
import { sudo_until, user } from "$lib/stores";
import { date_from_iso, get_preferred_language, log_error } from "$lib/utils";

import type { LayoutLoad } from "./$types";

console.log("hello from +layout.ts");

// on the server gets { url, params, data, route, fetch, setHeaders, depends, parent }
// data comes from +layout.server.ts
export const load: LayoutLoad = async ({ data, fetch, params }) => {
    console.log("+layout.ts load()", data);

    try {
        const res = await fetch(`${config.api.path_prefix}/users/me`);
        if (is_user_response(res)) {
            user.set(res.data.user);
            if (res.data.sudo_until)
                sudo_until.set(date_from_iso(res.data.sudo_until));
        }
    } catch (error) {
        console.log("could not fetch user:", error);
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
    console.log("+layout setting language to", language);
    await set_language(language);
    return { language };
};
