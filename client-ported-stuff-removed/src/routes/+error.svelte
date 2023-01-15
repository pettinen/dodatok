<ErrorPage>{errorText}</ErrorPage>

<script context=module lang=ts>
  import type { Load } from "@sveltejs/kit";

  export const load: Load = ({ error, status }) => ({ props: { error, status } });
</script>

<script lang=ts>
  import { _ } from "svelte-i18n";

  import ErrorPage from "$lib/components/ErrorPage.svelte";


  export let error: Error;
  export let status: number;

  $: errorText = ((): string => {
    const translation = $_(error.message, { default: "" });
    if (translation)
      return translation;
    if (status === 404)
      return $_("general.errors.not-found");
    return $_("general.errors.unexpected");
  })();
</script>
