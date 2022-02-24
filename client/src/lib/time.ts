import { derived, readable } from "svelte/store";

import { session } from "$app/stores";


export const time = readable(new Date(), (set) => {
  const interval = setInterval(() => {
    set(new Date());
  }, 10_000);

  return (): void => {
    clearInterval(interval);
  };
});

export const sudo = derived([session, time], ([$session, $time]) => {
  if (!$session.user.sudoUntil)
    return false;
  return $time < new Date($session.user.sudoUntil);
});
