<script lang="ts">
    import { language, loaded, tl } from "$i18n";
</script>

{#if $loaded($language)}
    <h1>({$tl("testan")}) {$tl("general.errors.fuck")}</h1>
{:else}
    <h1>Loading language {$language}...</h1>
{/if}

<a href="/">back</a>
