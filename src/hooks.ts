import type { GetSession } from "@sveltejs/kit";

import { cache } from "$lib/cache";
import type { Locals, Session } from "$lib/types";

export const getSession: GetSession<Locals, Session> = async () => {
  const session: Session = {
    user: null,
  };
  if (session.user?.icon)
    await cache.load(session.user.icon);
  return session;
};
