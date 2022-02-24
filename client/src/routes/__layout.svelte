<svelte:window on:load={onWindowLoad} on:keydown={onWindowKeydown} />

{#if $isLoading}
  <div class=loading transition:fade>
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

  {#if gotErrorMessages || gotWarningMessages || gotInfoMessages}
    <section class=messages role=alert>
      <div class=message-lists>
        {#if gotInfoMessages}
          <div class="message-list-container info-messages">
            <span class=material-icons>info</span>
            <ul class=message-list>
              {#each $infoMessages as message}
                <Message {message} />
              {/each}
            </ul>
          </div>
        {/if}
        {#if gotErrorMessages}
          <div class="message-list-container error-messages">
            <span class=material-icons>report</span>
            <ul class=message-list>
              {#each $errors as error}
                {@const formattedError = $formatError(error)}
                {#if formattedError}
                  <li>{formattedError}</li>
                {/if}
              {/each}
            </ul>
          </div>
        {/if}
        {#if gotWarningMessages}
          <div class="message-list-container warning-messages">
          <span class=material-icons>report_problem</span>
            <ul class=message-list>
              {#each $warnings as warning}
                {@const formattedWarning = $formatWarning(warning)}
                {#if formattedWarning}
                  <li>{formattedWarning}</li>
                {/if}
              {/each}
            </ul>
          </div>
        {/if}
      </div>
      <div class=close-message-container>
        <button
          class="button icon close-message-button material-icons"
          on:click={clearMessages}
        >
          close
        </button>
      </div>
    </section>
  {/if}
{/if}

<script context=module lang=ts>
  import { getLocaleFromNavigator, init, isLoading, locale, register } from "svelte-i18n";
  import type { Load } from "@sveltejs/kit";

  import { browser } from "$app/env";
  import { assets } from "$app/paths";
  import { updated } from "$app/stores";

  import { config } from "$lib/config";
  import { apiFetch, catchError } from "$lib/utils";

  import "normalize.css";
  import "@fontsource/fira-sans/latin.css";
  import "material-icons/iconfont/material-icons.css";
  import "$lib/styles/main.scss";


  export const load: Load = async ({ session }) => {
    for (const localeID of Object.keys(config.locales))
      register(localeID, () => import(`../translations/${localeID}.json`));

    const i18nInit = init({
      fallbackLocale: config.defaultLocale,
      initialLocale: session.user?.locale
        ?? (browser ? localStorage.getItem("locale") : null)
        ?? getLocaleFromNavigator(),
    });
    if (i18nInit instanceof Promise)
      i18nInit.catch(catchError);

    if (browser) {
      if (session.csrfToken) {
        localStorage.setItem(config.csrfTokenField, session.csrfToken);
      } else if (!session.user) {
        try {
          await apiFetch("/csrf-token");
        } catch (error) {
          catchError(error);
          session.errors.add("auth.errors.unauthenticated-csrf-token-fetch-failed");
        }
      }
    }

    return {};
  };
</script>

<script lang=ts>
  import { io } from "socket.io-client";
  import type { Socket } from "socket.io-client";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";

  import { session } from "$app/stores";

  import { cache } from "$lib/cache";
  import {
    errors,
    formatError,
    formatInfoMessage,
    formatWarning,
    infoMessages,
    warnings,
  } from "$lib/errors";
  import type { JSONObject } from "$lib/types";
  import { modals } from "$lib/utils";

  import Message from "$lib/components/Message.svelte";
  import Navbar from "$lib/components/Navbar.svelte";


  for (const url of [config.defaultUserIcon])
    cache.load(url).catch(catchError);


  let socket: Socket | null = null;
  let socketStatus = "not-created";

  const createSocket = async (): Promise<void> => {
    if (socketStatus !== "not-created")
      return;

    socketStatus = "creating";
    let data: JSONObject;
    try {
      data = await apiFetch("/account/websocket-token");
    } catch (error) {
      catchError(error);
      socketStatus = "failed";
      return;
    }

    if (data.errors) {
      socketStatus = "failed";
      return;
    }

    socket = io(config.socketio.endpoint, { path: config.socketio.path });
    socket.emit("authenticate", data.token);
    socketStatus = "created";
  }

  const destroySocket = (): void => {
    socketStatus = "not-created";
    if (!socket)
      return;
    console.log("disconnected");
    socket.disconnect();
    socket = null;
  }

  const clearMessages = (): void => {
    errors.clear();
    warnings.clear();
    infoMessages.clear();
  };

  const onWindowLoad = (): void => {
    document.body.classList.remove("preload");
  };

  const onWindowKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Escape") {
      const modal = modals.pop();
      if (modal)
        window.dispatchEvent(new CustomEvent(`close-${modal}`));
    }
  };

  $: browser && $session.user && void createSocket().catch(catchError);
  $: browser && !$session.user && destroySocket();
  $: $session.user?.icon && cache.load($session.user.icon).catch(catchError);
  $: $session.user?.icon && cache.load($session.user.icon).catch(catchError);
  $: browser && $locale && localStorage.setItem("locale", $locale);
  $: $updated && !$isLoading && infoMessages.show("general.update-available");
  $: !$isLoading && ($errors = [...$session.errors]);
  $: gotErrorMessages = $errors.map($formatError).filter((msg) => msg).length > 0;
  $: gotWarningMessages = $warnings.map($formatWarning).filter((msg) => msg).length > 0;
  $: gotInfoMessages = $infoMessages.map($formatInfoMessage).filter((msg) => msg).length > 0;
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

  .messages {
    position: fixed;
    right: 0;
    bottom: 0;
    width: 100%;
    max-width: 49rem;
    display: flex;
    justify-content: space-between;
    background-color: g.$dark-grey;
    color: g.$text-color-dark;
    z-index: 3;
  }

  .message-lists {
    width: calc(100% - g.$icon-button-size);
    display: flex;
    flex-direction: column;
  }

  .message-list-container {
    flex: 1;
    display: flex;
    align-items: center;
    padding: 0 0.25rem;

    .material-icons {
      height: g.$icon-button-size;
      align-self: start;
      display: flex;
      align-items: center;
    }
  }

  .message-list {
    list-style-type: none;
    margin: 0;
    padding: 0.5rem;

    li {
      line-height: 1.25em;
    }
  }

  .close-message-container {
    display: flex;
    flex-direction: column;
    width: g.$icon-button-size;
  }

  .close-message-button {
    background-color: g.$dark-grey;
    color: g.$white;
  }

  .info-messages {
    background-color: g.$dark-grey;
    color: g.$white;
  }

  .error-messages {
    background-color: g.$red;
  }

  .warning-messages {
    background-color: g.$yellow;
  }
</style>
