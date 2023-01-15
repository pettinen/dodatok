import { Mutex } from "async-mutex";
import { get, writable } from "svelte/store";
import type { Readable, Writable } from "svelte/store";


interface SocketStore extends Readable<WebSocket | null> {
  destroy: () => void;
  initialize: () => Promise<void>;
}

export const createSocketStore =
  (endpoint: string, registerHandlers: (socket: WebSocket) => void): SocketStore => {
    const socketStore: Writable<WebSocket | null> = writable(null);
    const initMutex = new Mutex();

    return {
      destroy: (): void => {
        socketStore.update((socket: WebSocket | null) => {
          socket?.close();
          return null;
        });
      },
      initialize: async (): Promise<void> => {
        await initMutex.runExclusive(() => {
          if (get(socketStore))
            return;
          const socket = new WebSocket(endpoint);
          registerHandlers(socket);
          socketStore.set(socket);
        });
      },
      subscribe: socketStore.subscribe,
    };
  };
