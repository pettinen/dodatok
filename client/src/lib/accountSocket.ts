import type { JTDSchemaType } from "ajv/dist/jtd.js";

import { config } from "$lib/config";
import { createSocketStore } from "$lib/socket";
import { validateWebSocketTokenResponse } from "$lib/types";
import { ajv, apiFetch, log, user } from "$lib/utils";


interface LogoutAllSessionsEvent {
  csrfToken: string;
}
const logoutAllSessionsSchema: JTDSchemaType<LogoutAllSessionsEvent> = {
  properties: {
    csrfToken: { type: "string" },
  },
};

interface UserUpdatedEvent {
  locale?: string;
}
const userUpdatedSchema: JTDSchemaType<UserUpdatedEvent> = {
  optionalProperties: {
    locale: { type: "string" },
  },
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const accountEvents: any = {
  logoutAllSessions: {
    handler: (data: LogoutAllSessionsEvent): void => {
      localStorage.setItem(config.csrfTokenStorageKey, data.csrfToken);
      user.set(null);
    },
    validator: ajv.compile(logoutAllSessionsSchema),
  },
  userUpdated: {
    handler: (data: UserUpdatedEvent): void => {
      console.log("user updated", data);
    },
    validator: ajv.compile(userUpdatedSchema),
  },
};

interface RawAccountEvent {
  event: keyof typeof accountEvents;
  data?: unknown;
}

/* eslint-disable
   @typescript-eslint/consistent-type-assertions,
   @typescript-eslint/no-explicit-any,
   @typescript-eslint/no-misused-promises,
   @typescript-eslint/no-unsafe-argument,
   @typescript-eslint/no-unsafe-assignment,
   @typescript-eslint/no-unsafe-call,
   @typescript-eslint/no-unsafe-member-access,
 */

const isAccountEvent = (data: unknown): data is RawAccountEvent =>
  Boolean(data)
  && typeof data === "object"
  && Object.hasOwn(accountEvents, (data as any).event);

const addAccountSocketHandlers = (socket: WebSocket): void => {
  socket.addEventListener("open", async () => {
    const { data } = await apiFetch(
      config.websocket.account.tokenEndpoint, validateWebSocketTokenResponse, "POST"
    );
    if (data) {
      socket.send(JSON.stringify({
        event: "authenticate",
        data: { token: data.token },
      }));
    } else {
      log("Could not fetch websocket token");
      socket.close();
    }
  });

  socket.addEventListener("message", (event: MessageEvent) => {
    let data: unknown;
    try {
      data = JSON.parse(event.data);
    } catch (error) {
      log("could not parse account event", event.data, error);
      return;
    }
    if (!isAccountEvent(data)) {
      log("invalid account event data", data);
      return;
    }

    const { handler, validator } = accountEvents[data.event];
    if (!validator(data.data)) {
      log("invalid account event data", data);
      return;
    }
    handler(data.data);
  });
};

export const accountSocket = createSocketStore(
  config.websocket.account.endpoint, addAccountSocketHandlers
);
