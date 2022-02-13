<div class=content>
  <h1>{$_("errors.welp")}</h1>
  <p>{errorText}</p>
</div>
<!--
<script context=module lang=ts>
  export const load = ({ error, status }) => {
    return {
      props: { error, status },
    };
  };
</script>
-->

<script lang=ts>
  import { _ } from "svelte-i18n";

  import { page } from "$app/stores";

  import { AppError } from "$lib/errors";

  $: errorText = (() => {
    if ($page.error instanceof AppError)
      return $page.error.text;
    if ($page.status === 404)
      return $_("errors.not-found");
    return $_("errors.unexpected");
  })();
</script>
