<svelte:window on:keydown={onWindowKeydown} />

{#if !$isLoading}
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

<script lang=ts>
  import { goto } from "$app/navigation";
  import { page, updated } from "$app/stores";

  import { accountSocket } from "$lib/accountSocket";
  import { cache } from "$lib/cache";

  import Message from "$lib/components/Message.svelte";
  import Navbar from "$lib/components/Navbar.svelte";

  export let data;

  user.set($page.data.user);

  if (browser) {
    if ($session.csrfToken)
      localStorage.setItem(config.csrfTokenStorageKey, $session.csrfToken);

    cache.load(config.defaultUserIcon);
    if ($user)
      cache.load($user.icon);
  }

  const onWindowKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Escape") {
      const modal = modals.pop();
      if (modal)
        window.dispatchEvent(new CustomEvent(`close-${modal}`));
    }
  };

  /* eslint-disable @typescript-eslint/indent */
  $: browser && $user && accountSocket.initialize().catch(unexpected);
  $: if (browser && !$user) {
       accountSocket.destroy();
       if (config.pages.authRequired.includes($page.url.pathname))
         goto(config.pages.index).catch(unexpected);
     }
  $: browser && $locale && localStorage.setItem("locale", $locale);
  $: $user?.icon && cache.load($user.icon);
  $: $updated && !$isLoading && infoMessages.show("general.update-available");
  $: gotErrorMessages =
       !$isLoading && $errors.map($formatError).filter((msg) => msg).length > 0;
  $: gotWarningMessages =
       !$isLoading && $warnings.map($formatWarning).filter((msg) => msg).length > 0;
  $: gotInfoMessages =
       !$isLoading && $infoMessages.map($formatInfoMessage).filter((msg) => msg).length > 0;
  /* eslint-enable @typescript-eslint/indent */
</script>

<style lang=scss>
  @use "$lib/styles/globals.scss" as g;

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
