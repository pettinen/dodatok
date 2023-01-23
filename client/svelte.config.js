import nodeAdapter from "@sveltejs/adapter-node";
import { vitePreprocess } from "@sveltejs/kit/vite";

/** @type {import('@sveltejs/kit').Config} */
export default {
    preprocess: vitePreprocess(),
    kit: {
        adapter: nodeAdapter(),
        alias: {
            $i18n: "src/i18n",
        },
    },
};
