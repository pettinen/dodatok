import { dev } from "$app/environment";
import { base } from "$app/paths";

const API_HOST = "http://localhost:5000";

export const config = {
    accepted_image_types: ["image/jpeg", "image/png", "image/webp"],
    api: {
        path_prefix: "/api",
        host: API_HOST,
    },
    base_urls: {
        files: "http://s3.kotori.lab:9000/files",
        user_icon: "http://s3.kotori.lab:9000/user-icons",
    },
    cookies: {
        csrf_token: "csrf_token",
        remember_token: "remember_token",
        session: "session",
        options: {
            path: `${base}/`,
            same_site: "Lax",
            secure: !dev,
        },
    },
    csrf_token_header: "CSRF-Token",
    csrf_token_storage_key: "csrf_token",
    debounce_time: 500,
    default_message_timeout: 10_000,
    default_user_icon: "http://s3.kotori.lab:9000/files/default-user-icon.png",
    pages: {
        auth_required: [`${base}/account`],
        no_auth_required: [`${base}/sign-up`],
        index: `${base}/`,
    },
    storage: {
        local: {
            language: {
                key: "language",
            },
        },
    },
    totp: {
        code_length: 6,
    },
    validation_rules: {
        password: {
            max_length: 1000,
            min_length: 8,
        },
        username: {
            max_length: 20,
            min_length: 1,
        },
        user_icon: {
            max_dimension_ratio: 3,
            max_size_mb: 10,
            min_size_px: 20,
        },
    },
    websocket: {
        account: {
            endpoint: `ws://${API_HOST}/account/socket`,
            token_endpoint: "/account/socket/token",
        },
    },
};
