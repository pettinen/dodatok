import { get, writable } from "svelte/store";
import { browser } from "$app/env";

import type { Fetch, Readable } from "$lib/types";

class CacheEntryLoaded {
  public readonly state = "loaded";
  private _url: string | null = null;

  public constructor(private readonly blob: Blob) {}

  public get url(): string | null {
    if (!browser)
      return null;
    if (this._url === null)
      this._url = URL.createObjectURL(this.blob);
    return this._url;
  }
}

interface CacheEntryNotLoaded {
  state: "failed" | "loading";
}

export type CacheEntry = CacheEntryLoaded | CacheEntryNotLoaded;

interface CacheStore extends Readable<Map<string, CacheEntry>> {
  load: (url: string, fetchFn?: Fetch) => Promise<void>;
  subscribe: (...args: unknown[]) => unknown;
}

const doFetch = async (url: string, fetchFn: Fetch): Promise<CacheEntry> => {
  let response: Response;
  try {
    response = await fetchFn(url);
  } catch {
    return { state: "failed" };
  }

  if (response.ok)
    return new CacheEntryLoaded(await response.blob());
  return { state: "failed" };
};

const createCache = (): CacheStore => {
  const store = writable(new Map<string, CacheEntry>());

  return {
    async load(url: string, fetchFn: Fetch = fetch): Promise<void> {
      if (get(store).has(url))
        return;
      store.update((map) => map.set(url, { state: "loading" }));
      const loadResult = await doFetch(url, fetchFn);
      store.update((map) => map.set(url, loadResult));
    },
    subscribe: store.subscribe,
  };
};

export const cache = createCache();
