{#if $user}
  <ErrorPage>{$_("auth.requires-no-login")}</ErrorPage>
{:else}
  <main>
    <Heading>{$_("signup.sign-up")}</Heading>
  </main>
{/if}
<script lang=ts>
  import { _ } from "svelte-i18n";

  import { user } from "$lib/utils";

  import ErrorPage from "$lib/components/ErrorPage.svelte";
  import Heading from "$lib/components/Heading.svelte";
</script>

<style lang=scss>
</style>
