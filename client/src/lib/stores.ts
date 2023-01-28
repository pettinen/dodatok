import type { DateTime } from "luxon";
import { writable } from "svelte/store";
import type { Writable } from "svelte/store";

import type { User } from "helpers";

export const sudo_until: Writable<DateTime | null> = writable(null);

export type { User };

export const user: Writable<User | null> = writable(null);
