{#if message.message === "general.update-available"}
  <li>
    {$_("general.update-available-prefix")}
    <button class=link on:click={reload}>{$_("general.update-available-refresh")}</button>
    {$_("general.update-available-suffix")}
  </li>
{:else if formatted}
  <li>{formatted}</li>
{/if}

<script lang=ts>
  import { _ } from "svelte-i18n";

  import { formatInfoMessage } from "$lib/errors";
  import type { InfoMessage } from "$lib/errors";


  export let message: InfoMessage;

  const reload = (): void => {
    location.reload();
  };

  $: formatted = $formatInfoMessage(message);
</script>
