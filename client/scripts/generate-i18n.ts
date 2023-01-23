import { basename, dirname } from "path";

import { globby } from "globby";

const extract_language = (path: string): string => basename(dirname(path));

const [language_paths, default_language_paths] = await Promise.all([
    globby("src/i18n/*/messages.po"),
    globby("src/i18n/*/default-language"),
]);

if (default_language_paths.length !== 1)
    throw new Error("specify one default language");

const default_language = extract_language(default_language_paths[0]);
const languages = language_paths.map(extract_language);

if (!languages.includes(default_language))
    throw new Error("invalid default language");

const languages_quoted = languages.map((lang) => JSON.stringify(lang));

const file_contents = [
    "// This file is auto-generated.",
    `export type Language = ${languages_quoted.join(" | ")};`,
    `export const languages: Language[] = [${languages_quoted.join(", ")}];`,
    `export const default_language: Language = "${default_language}";`,
].join("\n");

console.log(file_contents);
