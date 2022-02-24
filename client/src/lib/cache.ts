import { derived, get, writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { browser } from "$app/env";


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
  load: (url: string, reload?: boolean) => Promise<void>;
}

const createCache = (): CacheStore => {
  const store = writable(new Map<string, CacheEntry>());

  return {
    load: async (url: string, reload = false): Promise<void> => {
      if (!browser)
        return;

      if (get(store).has(url) && !reload)
        return;

      store.update((map) => map.set(url, { state: "loading" }));

      let response: Response;
      try {
        response = await fetch(url);
        if (!response.ok)
          throw new Error("Cache fetch failed");
      } catch {
        store.update((map) => map.set(url, { state: "failed" }));
        return;
      }

      const blob = await response.blob();
      store.update((map) => map.set(url, new CacheEntryLoaded(blob)));
    },
    subscribe: store.subscribe,
  };
};

export const cache = createCache();

export const cacheURL: Readable<(url: string) => string | null> = derived(
  cache,
  ($cache: Map<string, CacheEntry>) => (url: string) => {
    const entry = $cache.get(url);
    if (entry?.state === "loaded")
      return entry.url;
    return null;
  },
);
