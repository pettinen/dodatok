import { dev } from "$app/env";
import { base } from "$app/paths";


const apiHost = "api.kotori.lab:5000";

export const config = {
  acceptedImageTypes: ["image/jpeg", "image/png", "image/webp"],
  apiURL: `http://${apiHost}`,
  baseURLs: {
    files: "http://s3.kotori.lab:9000/files",
    userIcon: "http://s3.kotori.lab:9000/user-icons",
  },
  cookies: {
    csrfToken: "csrf_token",
    rememberToken: "remember_token",
    session: "session",
    options: {
      path: `${base}/`,
      sameSite: "Lax",
      secure: !dev,
    },
  },
  csrfTokenHeader: "CSRF-Token",
  csrfTokenStorageKey: "csrf_token",
  debounceTime: 500,
  defaultMessageTimeout: 10_000,
  defaultLocale: "en-US",
  defaultUserIcon: "http://s3.kotori.lab:9000/files/default-user-icon.png",
  locales: {
    "en-US": "English (US)",
    "fi-FI": "Suomi",
  },
  pages: {
    authRequired: [`${base}/account`],
    noAuthRequired: [`${base}/sign-up`],
    index: `${base}/`,
  },
  totp: {
    codeLength: 6,
  },
  validationRules: {
    password: {
      maxLength: 1000,
      minLength: 8,
    },
    username: {
      maxLength: 20,
      minLength: 1,
    },
    userIcon: {
      maxDimensionRatio: 3,
      maxSizeMB: 10,
      minSize: 20,
    },
  },
  websocket: {
    account: {
      endpoint: `ws://${apiHost}/account/socket`,
      tokenEndpoint: "/account/socket/token",
    },
  },
};
