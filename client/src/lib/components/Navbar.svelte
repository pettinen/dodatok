<script lang="ts">
    import { set_language } from "$i18n";
</script>

<nav>
    <button type="button" on:click={() => set_language("fi")}>fi</button>
    <button type="button" on:click={() => set_language("en")}>en</button>
</nav>

<style lang="scss">
    @use "$lib/styles/globals.scss" as g;
</style>
