import { derived, readable } from "svelte/store";

import { sudo_until } from "$lib/stores";

export const time = readable(new Date(), (set) => {
    const interval = setInterval(() => {
        set(new Date());
    }, 10_000);

    return (): void => {
        clearInterval(interval);
    };
});

export const sudo = derived([sudo_until, time], ([$sudo_until, $time]) => {
    if (!$sudo_until) return false;
    return $time < new Date($sudo_until);
});
