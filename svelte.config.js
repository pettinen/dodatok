import nodeAdapter from "@sveltejs/adapter-node";
import preprocess from "svelte-preprocess";


export default {
  kit: {
    adapter: nodeAdapter(),
  },
  preprocess: preprocess(),
};
