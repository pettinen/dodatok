<div class=content>
  <h1>{$_("general.errors.oops")}</h1>
  <p>{errorText}</p>
</div>

<script context=module lang=ts>
  import type { ErrorLoad } from "@sveltejs/kit";

  export const load: ErrorLoad = ({ error, status }) => ({ props: { error, status } });
</script>

<script lang=ts>
  import { _ } from "svelte-i18n";


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
