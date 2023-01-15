import { getLocaleFromNavigator, init, isLoading, locale, register } from "svelte-i18n";
import type { LayoutLoad } from './$types';

import { browser } from "$app/environment";

import { config } from "$lib/config";
import { errors } from "$lib/errors";

import "normalize.css";
import "@fontsource/fira-sans/latin.css";
import "material-icons/iconfont/material-icons.css";
import "$lib/styles/main.scss";


export const load: LayoutLoad = async ({ data }) => {
for (const localeID of Object.keys(config.locales))
    register(localeID, async () => import(`../translations/${localeID}.json`));

await init({
    fallbackLocale: config.defaultLocale,
    initialLocale: data.user?.locale
    ?? (browser ? localStorage.getItem("locale") : null)
    ?? getLocaleFromNavigator(),
});

errors.add(...data.errors);

return {};
};
