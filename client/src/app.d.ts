import type { Language } from "$i18n";
import type { Alert } from "$lib/alerts";

declare global {
    namespace App {
        interface Locals {
            csrf_token?: string;
            errors?: Alert[];
            warnings?: Alert[];
        }

        interface PageData {
            language: Language;
        }
    }
}
