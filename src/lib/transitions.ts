interface RevealParams {
  direction: "top" | "right" | "bottom" | "left";
  delay?: number;
  duration?: number;
  reverse?: boolean;
}

export const reveal = function(
  node: HTMLElement,
  { direction, delay = 0, duration = 400, reverse = false }: RevealParams
) {
  const rect = node.getBoundingClientRect();
  let directionIndex = {
    top: 0,
    right: 1,
    bottom: 2,
    left: 3,
  }[direction];
  const dim = rect[directionIndex % 2 === 0 ? "height" : "width"];
  const css = (t: number, u: number) => {
    const rectParams = ["auto", "auto", "auto"];
    let progress: number;
    if (directionIndex === 1 || directionIndex === 2)
      progress = reverse ? u : t;
    else
      progress = reverse ? t : u;
    rectParams.splice(directionIndex, 0, `${progress * dim}px`);
    return `clip: rect(${rectParams.join(", ")});`;
  }
  return { css, delay, duration };
};
