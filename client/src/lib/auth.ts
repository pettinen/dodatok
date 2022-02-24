import type { Load } from "@sveltejs/kit";

import { AppError } from "$lib/errors";


export const requireAuth: Load = ({ session }) => {
  if (!session.user) {
    return {
      error: new AppError("auth", "requires-login"),
      status: 400,
    };
  }
  return {};
};

export const requireNoAuth: Load = ({ session }) => {
  if (session.user) {
    return {
      error: new AppError("auth", "requires-no-login"),
      status: 400,
    };
  }
  return {};
};
