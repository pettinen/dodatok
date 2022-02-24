import type { RequestHandler } from "@sveltejs/kit";

import { config } from "$lib/config";


export const handle: RequestHandler = async ({ params, request }) => {
  let response: Response;
  try {
    response = await fetch(`${config.apiURL}/${params.path}`, request);
  } catch {
    return {
      body: { errors: [{ id: "could-not-connect-to-server", source: "general" }] },
      headers: { "Content-Type": "application/json" },
      status: 502,
    };
  }
  return response;
};

export {
  handle as del,
  handle as get,
  handle as post,
  handle as put,
};
