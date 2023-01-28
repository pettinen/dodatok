<script lang="ts">
    import { set_language, type Language } from "$i18n";

    const change_language = (lang: Language) => () => set_language(lang);
</script>

<nav>
    <button type="button" on:click={change_language("fi")}>fi</button>
    <button type="button" on:click={change_language("en")}>en</button>
</nav>

<style lang="scss">
    @use "$lib/styles/globals.scss" as g;
</style>
