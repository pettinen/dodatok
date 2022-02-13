import { get, writable } from "svelte/store";
import { _ } from "svelte-i18n";
import type { MessageFormatter } from "svelte-i18n/types/runtime/types";

import type { Writable } from "$lib/types";

export class AppError extends Error {
  public constructor(public readonly source: string, public readonly id: string) {
    super(id);
  }

  public get text(): string {
    return get(_)(`${this.source}.errors.${this.id}`);
  }
}

type LoginErrorID = "account-disabled" | "not-found";

export class LoginError extends AppError {
  public constructor(id: LoginErrorID) {
    super("login", id);
  }
}

export class UnexpectedError extends AppError {
  public constructor(source: string) {
    super(source, "unexpected");
  }
}

interface ErrorMessage {
  source: string;
  text: string;
}

interface ErrorsStore extends Writable<ErrorMessage[]> {
  clear: (clearSource?: string) => void;
  contains: (containsSource: string) => boolean;
}

const createErrors = (): ErrorsStore => {
  const store: Writable<ErrorMessage[]> = writable([]);

  return {
    clear: (clearSource?: string): void => {
      if (clearSource !== undefined)
        store.update((s) => s.filter(({ source }) => clearSource !== source));
      else
        store.set([]);
    },
    set: store.set,
    subscribe: store.subscribe,
    update: store.update,
  };
};

export const errors = createErrors();
