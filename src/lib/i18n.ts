import { register, init, getLocaleFromNavigator } from "svelte-i18n";

interface Translations {
  [id: string]: Translations | string;
}

for (const locale of ["en-US", "fi-FI"]) {
  register(
    locale,
    async (): Promise<Translations> => import(`./translations/${locale}.json`),
  );
}

void init({
  fallbackLocale: "en-US",
  initialLocale: getLocaleFromNavigator(),
});
