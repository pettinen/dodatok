import { dev } from "$app/env";


export const config = {
  acceptedImageTypes: ["image/jpeg", "image/png", "image/webp"],
  apiURL: "http://kotori.lab:5000",
  baseURLs: {
    files: "http://kotori.lab:9000/files",
    userIcon: "http://kotori.lab:9000/user-icons",
  },
  cookies: {
    options: {
      path: "/",
      sameSite: "Lax",
      secure: !dev,
    },
    rememberToken: "rememberToken",
    session: "session",
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
    authRequired: ["/account"],
    noAuthRequired: ["/sign-up"],
  },
  socketio: {
    endpoint: "ws://kotori.lab:5000",
    path: "/socket/",
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
};
