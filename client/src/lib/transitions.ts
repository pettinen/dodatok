import { linear } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";


enum Direction {
  top = 0,
  right = 1,
  bottom = 2,
  left = 3,
}

export interface RevealParams {
  direction: keyof typeof Direction;
  delay?: number;
  duration?: number;
  easing?: (t: number) => number;
  reverse?: boolean;
}

const REVEAL_DEFAULTS = {
  delay: 0,
  duration: 400,
  easing: linear,
  reverse: false,
};

export const reveal = (
  node: Readonly<HTMLElement>,
  {
    direction,
    delay = REVEAL_DEFAULTS.delay,
    duration = REVEAL_DEFAULTS.duration,
    easing = REVEAL_DEFAULTS.easing,
    reverse = REVEAL_DEFAULTS.reverse,
  }: Readonly<RevealParams>,
): TransitionConfig => {
  const dir = Direction[direction];
  let dim: number;
  if (dir === Direction.top || dir === Direction.bottom)
    dim = node.offsetHeight;
  else
    dim = node.offsetWidth;
  const css = (t: number, u: number): string => {
    const insetParams = ["0", "0", "0"];
    const progress = reverse ? t : u;
    insetParams.splice(dir, 0, `${progress * dim}px`);
    return `clip-path: inset(${insetParams.join(" ")});`;
  };
  return { css, delay, duration, easing };
};
