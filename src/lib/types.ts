export type Fetch = (info: RequestInfo, init?: RequestInit) => Promise<Response>;

export type Locals = Record<string, never>;

interface User {
  icon: string | null;
  id: string;
  username: string;
}

export interface Session {
  user: User | null;
}

export interface Readable<T> {
  subscribe: (run: (value: T) => void, invalidate?: (value?: T) => void) => () => void;
}

export interface Writable<T> extends Readable<T> {
  set: (value: T) => void;
  update: (updater: (value: T) => T) => void;
}
