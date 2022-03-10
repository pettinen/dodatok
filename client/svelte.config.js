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
