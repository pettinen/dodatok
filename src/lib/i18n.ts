import { register, init, getLocaleFromNavigator } from "svelte-i18n";

register("en-US", () => import("$lib/translations/en-US.json"));

init({
  fallbackLocale: "en-US",
  initialLocale: getLocaleFromNavigator(),
});
