import path from "path";

import nodeAdapter from "@sveltejs/adapter-node";
import preprocess from "svelte-preprocess";

export default {
  kit: {
    adapter: nodeAdapter(),
    prerender: {
      enabled: false,
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
