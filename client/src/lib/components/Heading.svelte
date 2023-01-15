<h1 bind:this={heading} class:avoid-navbar-intersection={avoidNavbarIntersection}>
  <slot />
</h1>

<script lang=ts>
  import { onMount, tick } from "svelte";

  import { page } from "$app/stores";

  import { config } from "$lib/config";
  import { noop } from "$lib/utils";

  let heading: HTMLHeadingElement;
  let avoidNavbarIntersection = false;

  onMount(async (): Promise<() => void> => {
    await tick();
    const menuButton = document.querySelector<HTMLElement>("#navbar-menu-button");
    if (!heading || !menuButton)
      return noop;

    const widths = [`${heading.offsetWidth}px`, `${menuButton.offsetWidth}px`];
    if (heading.parentNode instanceof HTMLElement)
      widths.push(getComputedStyle(heading.parentNode).paddingLeft);
    if ($page.url.pathname !== config.pages.index) {
      const homeButton = document.querySelector<HTMLElement>("#navbar-home-button");
      if (homeButton)
        widths.push(`${homeButton.offsetWidth}px`);
    }
    const breakpoint = `calc(${widths.join(" + ")})`;
    const avoidNavbarMediaQuery = matchMedia(`(max-width: ${breakpoint})`);
    avoidNavbarIntersection = avoidNavbarMediaQuery.matches;

    const update = (event: MediaQueryListEvent): void => {
      avoidNavbarIntersection = event.matches;
    };
    avoidNavbarMediaQuery.addEventListener("change", update);

    return (): void => {
      avoidNavbarMediaQuery.removeEventListener("change", update);
    };
  });
</script>

<style lang=scss>
  @use "$lib/styles/globals.scss" as g;

  h1 {
    height: g.$icon-button-size;
    display: inline-flex;
    align-items: end;
    margin: 0;
  }

  .avoid-navbar-intersection {
    margin-top: g.$icon-button-size;
  }
</style>
