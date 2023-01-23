<script lang="ts">
    import { tl } from "$i18n";

    import Heading from "$lib/components/Heading.svelte";
</script>

<main>
    <Heading>{$tl("general.errors.oops")}</Heading>
    <p><slot /></p>
</main>
