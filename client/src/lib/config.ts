import { dev } from "$app/env";
import { base } from "$app/paths";


export const config = {
  acceptedImageTypes: ["image/jpeg", "image/png", "image/webp"],
  apiURL: "http://api.kotori.lab:5000",
  baseURLs: {
    files: "http://s3.kotori.lab:9000/files",
    userIcon: "http://s3.kotori.lab:9000/user-icons",
  },
  cookies: {
    csrfToken: "csrfToken",
    rememberToken: "rememberToken",
    session: "session",
    options: {
      path: `${base}/`,
      sameSite: "Lax",
      secure: !dev,
    },
  },
  csrfTokenField: "csrfToken",
  debounceTime: 500,
  defaultMessageTimeout: 10_000,
  defaultLocale: "en-US",
  defaultUserIcon: "http://kotori.lab:9000/files/default-user-icon.png",
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
      endpoint: "ws://api.kotori.lab:5000/account/socket",
      tokenEndpoint: "/account/socket/token",
    }
  }
};
