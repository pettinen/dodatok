import { Mutex } from "async-mutex";
import { io } from "socket.io-client";
import type { Socket } from "socket.io-client";
import { get, writable } from "svelte/store";
import type { Readable, Writable } from "svelte/store";

import { config } from "$lib/config";
import { validateWebSocketTokenResponse } from "$lib/types";
import { apiFetch, log, updateUser, user } from "$lib/utils";


interface SocketStore extends Readable<Socket | null> {
  destroy: () => void;
  initialize: () => Promise<void>;
}

const createSocketStore = (endpoint: string, register_handlers: (socket: Socket) => void): SocketStore => {
  const socketStore: Writable<Socket | null> = writable(null);
  const initMutex = new Mutex();

  return {
    destroy: (): void => {
      socketStore.update((socket: Socket | null) => {
        if (socket?.connected)
          socket.disconnect();
        return null;
      });
    },
    initialize: async (): Promise<void> => {
      await initMutex.runExclusive(async (): Promise<void> => {
        if (get(socketStore))
          return;
        const socket = new WebSocket(endpoint);
        register_handlers(socket);
        socketStore.set(socket);
      });
    },
    subscribe: socketStore.subscribe,
  };
};

const addAccountSocketHandlers = (socket: WebSocket): void => {
  /*socket.addEventListener("logout_all_sessions", (csrfToken: string) => {
    localStorage.setItem(config.csrfTokenField, csrfToken);
    user.set(null);
  });*/
  socket.addEventListener("open", async () => {
    const { data } = await apiFetch(
      config.websocket.account.tokenEndpoint, validateWebSocketTokenResponse, "POST"
    );
    if (data) {
      socket.send(JSON.stringify({ event: "authenticate", data: { token: data.token } }));
    } else {
      log("Could not fetch websocket token");
      socket.close();
    }
  });

  socket.addEventListener("message", (event) => {
    let data: unknown = null;
    try {
      data = JSON.parse(event.data);
    } catch (error) {
      log(error);
    }
    if (data)
      console.log(data);
  });
};

export const accountSocket = createSocketStore(
  config.websocket.account.endpoint, addAccountSocketHandlers
);
