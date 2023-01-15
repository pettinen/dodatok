/*
import path from "path";

import preprocess from "svelte-preprocess";
import nodeAdapter from "@sveltejs/adapter-node";


export default {
  kit: {
    adapter: nodeAdapter(),
    prerender: {
      enabled: false,
    },
    version: {
      pollInterval: 600_000,
    },
    vite: {
      resolve: {
        alias: {
          $lib: path.resolve("src/lib"),
        }
      },
    },
  },
  preprocess: preprocess({
    scss: { includePaths: ["src/lib/styles"] },
  }),
};
*/

import nodeAdapter from "@sveltejs/adapter-node";
import { vitePreprocess } from "@sveltejs/kit/vite";

/** @type {import('@sveltejs/kit').Config} */
const config = {
    // Consult https://kit.svelte.dev/docs/integrations#preprocessors
    // for more information about preprocessors
    preprocess: vitePreprocess({
      scss: { includePaths: ["src/lib/styles"] },
    }),

    kit: {
        adapter: nodeAdapter(),
    },
};

export default config;
