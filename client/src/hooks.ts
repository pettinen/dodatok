import { parse as parseCookies, serialize as serializeCookie } from "cookie";
import { parse as parseSetCookie } from "set-cookie-parser";
import type { GetSession, Handle } from "@sveltejs/kit";

import { config } from "$lib/config";
import { convertAPIAlert } from "$lib/errors";
import type { APIAlert } from "$lib/errors";
import { convertAPIUser } from "$lib/utils";


export const getSession: GetSession = async ({ locals }) => ({
  ...locals,
  user: convertAPIUser(locals.user),
});

export const handle: Handle = async ({ event, resolve }) => {
  let sessionID = "";
  event.locals = {
    csrfToken: null,
    errors: new Set(),
    user: null,
  };

  interface AuthCookies {
    rememberToken?: string;
    session?: string;
  }
  const authCookies: AuthCookies = {};

  const addErrorsFromAPI = (errors: APIAlert[]): void => {
    for (const error of errors)
      event.locals.errors.add(convertAPIAlert("error")(error));
  };

  const requestCookiesRaw = event.request.headers.get("Cookie");
  if (requestCookiesRaw) {
    const requestCookies = parseCookies(requestCookiesRaw);
    if (requestCookies.session) {
      sessionID = requestCookies.session;
    } else if (requestCookies.rememberToken) {
      let sessionResponse: Response;
      try {
        sessionResponse = await fetch(`${config.apiURL}/get-session`, {
          headers: event.request.headers,
          method: "POST",
        });
        for (const cookie of sessionResponse.headers.raw()["set-cookie"]) {
          const [parsedCookie] = parseSetCookie(cookie);
          if (parsedCookie.name === config.cookies.session) {
            authCookies.session = cookie;
            sessionID = parsedCookie.value;
          } else if (parsedCookie.name === config.cookies.rememberToken) {
            authCookies.rememberToken = cookie;
          }
        }
        try {
          const data = await sessionResponse.json();
          if (data.errors)
            addErrorsFromAPI(data.errors);
          else
            event.locals.csrfToken = data.csrfToken;
        } catch {
          event.locals.errors.add("general.errors.could-not-connect-to-server");
        }
      } catch {
        event.locals.errors.add("general.errors.could-not-connect-to-server");
      }
    }
  }

  if (sessionID) {
    let userResponse: Response | null = null;
    try {
      userResponse = await fetch(`${config.apiURL}/users/me`, {
        headers: { "Cookie": `${config.cookies.session}=${sessionID}` },
      });
    } catch {
      event.locals.errors.add("general.errors.could-not-connect-to-server");
    }
    if (userResponse) {
      if (userResponse.ok) {
        try {
          event.locals.user = (await userResponse.json()).user;
        } catch {
          event.locals.errors.add("general.errors.user-data-fetch-failed");
        }
      } else {
        try {
          const data = (await userResponse.json());
          if (data.errors) {
            const clearSessionCookie = () => {
              authCookies.session = serializeCookie(config.cookies.session, "", {
                httpOnly: true,
                maxAge: 0,
                path: config.cookies.options.path,
                sameSite: config.cookies.options.sameSite,
                secure: config.cookies.options.secure
              });
            };
            // Replace not-logged-in error with user-data-fetch-failed
            if (
              data.errors.some((error: APIAlert) =>
                error.source === "auth" && error.id === "not-logged-in")
            ) {
              clearSessionCookie();
              data.errors.unshift({ source: "general", id: "user-data-fetch-failed" });
              data.errors = data.errors.filter(
                (error: APIAlert) => !(error.source === "auth" || error.id === "not-logged-in")
              )
            }
            addErrorsFromAPI(data.errors);
            if (data.errors.some((error: APIAlert) => error.source === "auth"))
              clearSessionCookie();
          }
        } catch {
          event.locals.errors.add("general.errors.user-data-fetch-failed");
        }
      }
    }
  }
  const response = await resolve(event);
  for (const cookie of Object.values(authCookies))
    response.headers.append("Set-Cookie", cookie);
  return response;
};
