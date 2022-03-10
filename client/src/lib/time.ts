import { derived, readable } from "svelte/store";

import { user } from "$lib/utils";


export const time = readable(new Date(), (set) => {
  const interval = setInterval(() => {
    set(new Date());
  }, 10_000);

  return (): void => {
    clearInterval(interval);
  };
});

export const sudo = derived([user, time], ([$user, $time]) => {
  if (!$user?.sudoUntil)
    return false;
  return $time < new Date($user.sudoUntil);
});
