import { deepEqual } from "fast-equals";
import { _ } from "svelte-i18n";
import { derived, get, writable } from "svelte/store";
import type { Readable, Writable } from "svelte/store";

import { dev } from "$app/env";

import { config } from "$lib/config";
import type { JSONObject } from "$lib/types";


export class AppError extends Error {
  public constructor(public readonly source: string, public readonly id: string) {
    super(`${source}.errors.${id}`);
  }
}

type APIAlertType = "error" | "warning";

export interface APIAlert {
  details?: string;
  id: string;
  source: string;
  values?: JSONObject;
}

export type Message = string | [string, JSONObject];
export type Alert = Message;

export const format = derived(_, ($_) => (message: Message): string => {
  if (Array.isArray(message)) {
    const [id, values] = message;
    const rv = $_(id, { default: "", values });
    return dev ? rv || "(missing translation)" : rv;
  }
  const rv = $_(message, { default: "" });
  return dev ? rv || "(missing translation)" : rv;
});

export const formatError = derived(_, ($_) => (error: Alert): string => {
  if (Array.isArray(error)) {
    const [id, values] = error;
    const rv = $_(id, { default: "", values }) || $_("general.errors.unexpected");
    return dev ? `${id}: ${rv}` : rv;
  }
  const rv = $_(error, { default: "" }) || $_("general.errors.unexpected");
  return dev ? `${error}: ${rv}` : rv;
});

export const formatWarning = derived(_, ($_) => (warning: Alert): string => {
  if (Array.isArray(warning)) {
    const [id, values] = warning;
    const rv = $_(id, { default: "", values });
    if (dev)
      return `${id}: ${rv || "(missing translation)"}`;
    return rv;
  }
  const rv = $_(warning, { default: "" });
  if (dev)
    return `${warning}: ${rv || "(missing translation)"}`;
  return rv;
});

interface AlertStore extends Writable<Alert[]> {1
  add: (...alerts: Alert[]) => void;
  addFromAPI: (alerts: APIAlert[]) => void;
  clear: (...clearSources: Array<string | RegExp>) => void;
}

export const convertAPIAlert = (type: APIAlertType) => (error: APIAlert): Alert => {
  const alertID = `${error.source}.${type}s.${error.id}`;
  if ("values" in error)
    return [alertID, error.values];
  return alertID;
};

const alertFilter = (searches: Array<string | RegExp>, revert = false) =>
  (alert: Alert): boolean => {
    const alertID = Array.isArray(alert) ? alert[0] : alert;
    for (const search of searches) {
      if (typeof search === "string") {
        if (search === alertID)
          return !revert;
      } else if (search.test(alertID)) {
        return !revert;
      }
    }
    return revert;
  };

const createAlertStore = (type: APIAlertType): AlertStore => {
  const alertStore: Writable<Alert[]> = writable([]);

  return {
    add: (...alerts: Alert[]): void => {
      const current = get(alertStore);
      const newAlerts = alerts.filter((alert) =>
        !current.some((existing) => deepEqual(alert, existing)));
      alertStore.update((store) => [...newAlerts, ...store]);
    },
    addFromAPI(alerts: APIAlert[]): void {
      this.add(...alerts.map(convertAPIAlert(type)));
    },
    clear: (...searches: Array<string | RegExp>): void => {
      if (searches.length > 0)
        alertStore.update((store) => store.filter(alertFilter(searches, true)));
      else
        alertStore.set([]);
    },
    set: alertStore.set,
    subscribe: alertStore.subscribe,
    update: alertStore.update,
  };
};

export const errors: AlertStore = createAlertStore("error");
export const warnings: AlertStore = createAlertStore("warning");

export const gotErrors: Readable<(...searches: Array<string | RegExp>) => boolean> = derived(
  errors,
  ($errors) => (...searches: Array<string | RegExp>) => $errors.some(alertFilter(searches)),
);


export interface InfoMessage {
  message: Message;
  id: symbol;
  timeout?: ReturnType<typeof setTimeout>;
}

interface InfoMessageStore extends Writable<InfoMessage[]> {
  clear: () => void;
  show: (...messages: Message[]) => void;
  showTimed: (...messages: Message[]) => void;
}

const createInfoMessageStore = (): InfoMessageStore => {
  const messageStore: Writable<InfoMessage[]> = writable([]);

  const add = (time: number | null, messages: Message[]): void => {
    const current = get(messageStore);
    const newMessages: InfoMessage[] = [];

    for (const newMessage of messages) {
      const existing = current.find(({ message }) => deepEqual(message, newMessage));
      if (existing) {
        if (existing.timeout)
          clearTimeout(existing.timeout);
        if (time) {
          existing.timeout = setTimeout(() => {
            messageStore.update((store) =>
              store.filter((messageData) => messageData.id !== existing.id));
          }, time);
        }
      } else {
        const id = Symbol("InfoMessage");
        let timeout: ReturnType<typeof setTimeout> | null;
        const newInfoMessage: InfoMessage = { id, message: newMessage };
        if (time) {
          newInfoMessage.timeout = setTimeout(() => {
            messageStore.update((store) => store.filter((message) => message.id !== id));
          }, time);
        }
        newMessages.push(newInfoMessage);
      }
    }
    if (newMessages.length > 0)
      messageStore.update((store) => [...newMessages, ...store]);
  };

  return {
    clear: (): void => {
      messageStore.set([]);
    },
    show: (...messages: Message[]): void => {
      add(null, messages);
    },
    showTimed: (...messages: Message[]): void => {
      add(config.defaultMessageTimeout, messages);
    },
    subscribe: messageStore.subscribe,
  };
};

export const formatInfoMessage = derived(_, ($_) => ({ message }: InfoMessage): string => {
  const specialCases = new Set(["general.update-available"]);
  if (specialCases.has(message))
    return message;

  if (Array.isArray(message)) {
    const [id, values] = message;
    const rv = $_(id, { default: "", values });
    return dev ? `${id}: ${rv || "(missing translation)"}` : rv;
  }
  const rv = $_(message, { default: "" });
  return dev ? `${message}: ${rv || "(missing translation)"}` : rv;
});

export const infoMessages = createInfoMessageStore();
