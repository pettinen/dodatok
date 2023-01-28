import {
    /* parse as parseCookies, */ serialize as serializeCookie,
} from "cookie";
import {
    parse as parse_set_cookie,
    splitCookiesString as split_set_cookie,
} from "set-cookie-parser";
import type { Handle } from "@sveltejs/kit";

import { has_csrf_token, has_warnings, is_failure } from "helpers";

import { dev } from "$app/environment";
import { languages, load_language } from "$i18n";
import { config } from "$lib/config";
import { errors, warnings } from "$lib/alerts";
import { get_preferred_language } from "$lib/utils";

console.log("hello from hooks.server.ts");

const { log } = console;
log(languages);
await Promise.all(languages.map(load_language));

const get_cookies_from_response = (response: Response) => {
    let remember_token: string | null = null;
    let session_id: string | null = null;
    const set_cookie_string = response.headers.get("Set-Cookie") ?? "";
    const cookies = parse_set_cookie(split_set_cookie(set_cookie_string));
    for (const cookie of cookies) {
        if (cookie.name === config.cookies.remember_token)
            remember_token = cookie.value;
        else if (cookie.name === config.cookies.session)
            session_id = cookie.value;
    }
    return { remember_token, session_id };
};

const restore_session = async (old_remember_token: string) => {
    let csrf_token: string | null = null;
    let remember_token: string | null = null;
    let session_id: string | null = null;
    let session_response: Response | null = null;
    try {
        session_response = await fetch(`${API_URL}/auth/restore-session`, {
            headers: {
                Cookie: serializeCookie("remember_token", old_remember_token),
            },
            method: "POST",
        });
    } catch (error) {
        log(error);
        errors.add_from_api({ source: "auth", id: "session_fetch_failed" });
    }
    if (session_response) {
        ({ remember_token, session_id } =
            get_cookies_from_response(session_response));

        let data: unknown;
        try {
            data = await session_response.json();
        } catch (error) {
            log(error);
            errors.add_from_api({ source: "auth", id: "session_fetch_failed" });
        }

        if (data !== undefined) {
            if (has_csrf_token(data)) {
                ({ csrf_token } = data);
                if (has_warnings(data)) warnings.add_from_api(...data.warnings);
                if (is_failure(data)) errors.add_from_api(...data.errors);
            } else {
                throw new Error("Unexpected data in session response");
            }
        }
    }
    return { csrf_token, remember_token, session_id };
};

export const handle: Handle = async ({ event, resolve }) => {
    console.log("this-should-run");
    if (dev && event.url.pathname.startsWith(`${config.api.path_prefix}/`)) {
        console.log("this-should-not-run");
        return fetch(config.api.host, event.request);
    }

    errors.add_from_api({ source: "auth", id: "invalid-credentials" });
    /*
    const errors = new Set<string>();
    let sessionID = "";
    let csrfToken: string | null = null;
    let user: User | null = null;

    interface Cookies {
        csrfToken?: string;
        rememberToken?: string;
        session?: string;
    }
    const cookies: Cookies = {};
    const requestCookieString = event.request.headers.get("Cookie");
    if (requestCookieString !== null) {
        const requestCookies = parseCookies(requestCookieString);
        if (requestCookies.session)
            sessionID = requestCookies.session;
        else if (requestCookies.remember_token)
            await restore_session(requestCookies.remember_token);
    }

    const sessionResponse = await fetch(`${API_URL}/auth/restore-session`);
    */

    const language = get_preferred_language(
        null,
        null,
        event.params.lang ?? null,
    );

    return resolve(event, {
        transformPageChunk: ({ html }) =>
            html.replace(
                "%lang-6bfef8a0-8879-4682-a60a-cc1bb2153e92%",
                language,
            ),
    });
};
