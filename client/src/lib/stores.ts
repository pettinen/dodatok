import { writable } from "svelte/store";
import type { Writable } from "svelte/store";

import type { User } from "helpers";

export const sudo_until: Writable<Date | null> = writable(null);

export type { User };

export const user: Writable<User | null> = writable(null);
