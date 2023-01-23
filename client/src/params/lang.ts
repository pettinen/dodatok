import type { ParamMatcher } from "@sveltejs/kit";

import { is_language } from "$i18n";
import type { Language } from "$i18n";

export const match: ParamMatcher = (param): param is Language =>
    is_language(param);
