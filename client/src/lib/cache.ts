import { derived, writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { browser } from "$app/environment";

import { unexpected } from "$lib/utils";


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
  load: (url: string, reload?: boolean) => void;
}

const createCache = (): CacheStore => {
  const { subscribe, update } = writable(new Map());

  const doFetch = async (url: string): Promise<void> => {
    let response: Response;
    try {
      response = await fetch(url);
      if (!response.ok)
        throw new Error("Cache fetch failed");
    } catch {
      update((current) => current.set(url, { state: "failed" }));
      return;
    }

    const blob = await response.blob();
    update((current) => current.set(url, new CacheEntryLoaded(blob)));
  };

  return {
    load: (url: string, reload = false): void => {
      if (!browser)
        return;

      update((current) => {
        if (reload || !current.has(url)) {
          current.set(url, { state: "loading" });
          doFetch(url).catch(unexpected);
        }
        return current;
      });
    },
    subscribe,
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
