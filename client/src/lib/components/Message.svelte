<script lang="ts">
    import { tl } from "$i18n";
    import type { InfoMessage } from "$lib/messages";

    export let message: InfoMessage;
    $: ({ message: messageString } = message);

    const reload = () => {
        location.reload();
    };
</script>

{#if messageString === "general.update_available"}
    <li>
        <button class="link" type="button" on:click={reload}>
            {$tl("general.update_available")}
        </button>
    </li>
{:else}
    <li>{messageString}</li>
{/if}
