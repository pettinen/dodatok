import { DateTime } from "luxon";
import { derived, readable } from "svelte/store";

import { sudo_until } from "$lib/stores";

export const time = readable(DateTime.utc(), (set) => {
    const interval = setInterval(() => {
        set(DateTime.utc());
    }, 10_000);
    return (): void => {
        clearInterval(interval);
    };
});

export const sudo = derived([sudo_until, time], ([$sudo_until, $time]) => {
    if (!$sudo_until) return false;
    return $time < $sudo_until;
});
