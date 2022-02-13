<svelte:window on:load={onWindowLoad} />

{#if $isLoading}
  <div class=loading out:fade>
    <div class=spinner>
      <div></div>
      <div></div>
      <div></div>
      <div></div>
    </div>
  </div>
{:else}
  <Navbar />

  <slot />

  {#if $errors.length > 0}
    <section class=errors role=alert>
      <ul class:single-item={$errors.length === 1}>
        {#each $errors as { text }}
          <li>{text}</li>
        {/each}
      </ul>
      <div class=close-container>
        <button
          class="button close icon material-icons"
          on:click={clearErrors}
        >
          close
        </button>
      </div>
    </section>
  {/if}
{/if}

<script context=module lang=ts>
  import type { Load } from "@sveltejs/kit";
  import { fade } from "svelte/transition";

  import { assets } from "$app/paths";

  import { cache } from "$lib/cache";
  import { AppError } from "$lib/errors";

  import "normalize.css";
  import "@fontsource/fira-sans/latin.css";
  import "material-icons/iconfont/material-icons.css";
  import "$lib/styles/main.scss";

  export const load: Load = ({ fetch, session, url }) => {
    const authPages = [
      "/account",
    ];
    const noAuthPages = [
      "/sign-up",
    ];

    if (!session.user && authPages.includes(url.pathname)) {
      return {
        error: "absolutely fuck",//new AppError("auth", "requires-login"),
        status: 307,
      };
    }

    if (session.user && noAuthPages.includes(url.pathname)) {
      return {
        error: new AppError("auth", "requires-no-login"),
        status: 403,
      };
    }

    const preloadedImages = [`${assets}/default-avatar.png`];
    preloadedImages.map(async (url) => cache.load(url, fetch));
    return {};
  };
</script>

<script lang=ts>
  import "$lib";

  import { isLoading } from "svelte-i18n";

  import { page } from "$app/stores";

  import Navbar from "$lib/components/Navbar.svelte";
  import { errors } from "$lib/errors";

  const clearErrors = (): void => {
    errors.clear();
  };

  const onWindowLoad = (): void => {
    document.body.classList.remove("preload");
  };
</script>

<style lang=scss>
  @use "globals.scss" as g;

  .loading {
    align-items: center;
    background: g.$background-color;
    display: flex;
    height: 100vh;
    justify-content: center;
    position: absolute;
    width: 100vw;

    .spinner {
      $spinner-base-size: max(1.5vw, 1.5vh);

      display: inline-block;
      position: relative;
      width: calc(10 * $spinner-base-size);
      height: calc(10 * $spinner-base-size);

      div {
        @keyframes spinner {
          0% {
            transform: rotate(0deg);
          }
          100% {
            transform: rotate(360deg);
          }
        }

        box-sizing: border-box;
        display: block;
        position: absolute;
        width: calc(8 * $spinner-base-size);
        height: calc(8 * $spinner-base-size);
        margin: $spinner-base-size;
        border: $spinner-base-size solid g.$white;
        border-radius: 50%;
        animation: spinner 1.2s cubic-bezier(0.5, 0, 0.5, 1) infinite;
        border-color: g.$white transparent transparent transparent;

        &:nth-child(1) {
          animation-delay: -0.45s;
        }
        &:nth-child(2) {
          animation-delay: -0.3s;
        }
        &:nth-child(3) {
          animation-delay: -0.15s;
        }
      }
    }
  }

  .errors {
    background-color: g.$red;
    bottom: 0;
    color: g.$text-color-dark;
    display: flex;
    justify-content: space-between;
    max-width: 49rem;
    position: fixed;
    right: 0;
    width: 100%;

    ul {
      list-style-type: "\2015\A0";  // <HORIZONTAL BAR><NO-BREAK SPACE>

      &.single-item {
        list-style-type: none;
      }

      li {
        line-height: 1.25em;
      }
    }

    div {
      display: flex;
      flex-direction: column;
      width: g.$icon-button-size;
    }

    button {
      background-color: g.$red;
    }
  }
</style>
