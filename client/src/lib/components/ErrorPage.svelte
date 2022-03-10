<main>
  <Heading>{$_("general.errors.oops")}</Heading>
  <p><slot /></p>
</main>

<script lang=ts>
  import { _ } from "svelte-i18n";

  import Heading from "$lib/components/Heading.svelte";
</script>
