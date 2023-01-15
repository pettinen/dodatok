import { parse as parseCookies, serialize as serializeCookie } from "cookie";
import type { CookieSerializeOptions } from "cookie";
// eslint-disable-next-line @typescript-eslint/no-redeclare
import type { RequestInfo, RequestInit, Response } from "node-fetch";
import { parse as parseSetCookie } from "set-cookie-parser";
import type { Handle } from "@sveltejs/kit";

import { config } from "$lib/config";
import { convertAPIAlert } from "$lib/errors";
import type { APIAlert, Message } from "$lib/errors";
import { validateAPIErrors, validateCSRFTokenResponse, validateUserResponse } from "$lib/types";
import { convertAPIUser, log } from "$lib/utils";


// eslint-disable-next-line no-implicit-globals
declare function fetch(url: RequestInfo, init?: RequestInit): Promise<Response>;

const serializeSetCookie = (name: string, value: string): string => {
  const cookieOptions: CookieSerializeOptions = {
    httpOnly: true,
    path: config.cookies.options.path,
    secure: config.cookies.options.secure,
  };
  if (
    config.cookies.options.sameSite === "strict"
    || config.cookies.options.sameSite === "lax"
    || config.cookies.options.sameSite === "none"
  )
    cookieOptions.sameSite = config.cookies.options.sameSite;
  return serializeCookie(name, value, cookieOptions);
};

export const handle: Handle = async ({ event, resolve }) => {
  if (/^\/api($|\/)/u.test(event.url.pathname))
    return resolve(event);

  const errors = new Set<Message>();
  let sessionID = "";
  let csrfToken = null;
  let user = null;

  interface Cookies {
    csrfToken?: string;
    rememberToken?: string;
    session?: string;
  }
  const cookies: Cookies = {};

  const addCookiesFromResponse = (response: Response): void => {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    for (const cookie of response.headers.raw()["set-cookie"] ?? []) {
      const [parsedCookie] = parseSetCookie(cookie);
      if (parsedCookie.name === config.cookies.session) {
        cookies.session = cookie;
        sessionID = parsedCookie.value;
      } else if (parsedCookie.name === config.cookies.rememberToken) {
        cookies.rememberToken = cookie;
      }
    }
  };

  const addCSRFToken = (token: string): void => {
    cookies.csrfToken = serializeSetCookie(config.cookies.csrfToken, token);
    csrfToken = token;
  };

  const addErrorsFromAPI = (apiErrors: APIAlert[]): void => {
    for (const error of apiErrors)
      errors.add(convertAPIAlert("error")(error));
  };

  const requestCookiesRaw = event.request.headers.get("Cookie");
  if (requestCookiesRaw) {
    const requestCookies = parseCookies(requestCookiesRaw);
    if (requestCookies.session) {
      sessionID = requestCookies.session;
    } else if (requestCookies.rememberToken) {
      let sessionResponse: Response | null = null;
      try {
        sessionResponse = await fetch(`${config.apiURL}/auth/restore-session`, {
          headers: { Cookie: requestCookiesRaw },
          method: "POST",
        });
      } catch (error) {
        log(error);
        errors.add("auth.errors.session-fetch-failed");
      }
      if (sessionResponse) {
        addCookiesFromResponse(sessionResponse);

        let data: unknown = null;
        try {
          data = await sessionResponse.json();
        } catch (error) {
          log(error);
          errors.add("auth.errors.session-fetch-failed");
        }

        if (data) {
          if (validateCSRFTokenResponse(data)) {
            addCSRFToken(data.csrf_token);
          } else if (validateAPIErrors(data)) {
            if (data.csrf_token)
              addCSRFToken(data.csrf_token);
            addErrorsFromAPI(data.errors);
          } else {
            throw new Error("Unexpected data in session response");
          }
        }
      }
    }
  }

  if (sessionID) {
    let userResponse: Response | null = null;
    try {
      userResponse = await fetch(`${config.apiURL}/users/me`, {
        headers: { Cookie: `${config.cookies.session}=${sessionID}` },
      });
    } catch (error) {
      log(error);
      errors.add("general.errors.could-not-connect-to-server");
    }
    if (userResponse) {
      addCookiesFromResponse(userResponse);

      let data: unknown = null;
      try {
        data = await userResponse.json();
      } catch (error) {
        log(error);
        errors.add("general.errors.user-data-fetch-failed");
      }

      if (data && validateUserResponse(data)) {
        user = convertAPIUser(data);
      } else if (validateAPIErrors(data)) {
        if (data.csrf_token)
          addCSRFToken(data.csrf_token);

        // Replace not-logged-in error with user-data-fetch-failed
        const isNotLoggedInError =
          (error: APIAlert): boolean => error.source === "auth" && error.id === "not-logged-in";

        if (data.errors.some(isNotLoggedInError)) {
          data.errors = data.errors.filter((error) => !isNotLoggedInError(error));
          data.errors.unshift({ source: "general", id: "user-data-fetch-failed" });
        }
        addErrorsFromAPI(data.errors);
      } else {
        errors.add("general.errors.user-data-fetch-failed");
      }
    }
  } else {
    let csrfResponse: Response | null = null;
    try {
      csrfResponse = await fetch(`${config.apiURL}/auth/csrf-token`);
    } catch (error) {
      log(error);
      errors.add("csrf.errors.fetch-failed");
    }
    if (csrfResponse) {
      addCookiesFromResponse(csrfResponse);

      let data: unknown = null;
      try {
        data = await csrfResponse.json();
      } catch (error) {
        log(error);
        errors.add("csrf.errors.fetch-failed");
      }
      if (data && validateCSRFTokenResponse(data)) {
        addCSRFToken(data.csrf_token);
      } else if (validateAPIErrors(data)) {
        if (data.csrf_token)
          addCSRFToken(data.csrf_token);
        addErrorsFromAPI(data.errors);
      } else {
        errors.add("csrf.errors.fetch-failed");
      }
    }
  }

  // eslint-disable-next-line require-atomic-updates
  event.locals = {
    csrfToken,
    errors,
    user,
  };

  const response = await resolve(event);
  if (cookies.csrfToken)
    response.headers.append("Set-Cookie", cookies.csrfToken);
  if (cookies.rememberToken)
    response.headers.append("Set-Cookie", cookies.rememberToken);
  if (cookies.session)
    response.headers.append("Set-Cookie", cookies.session);
  return response;
};
