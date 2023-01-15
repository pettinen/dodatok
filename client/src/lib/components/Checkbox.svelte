<label>
  <input bind:checked={checked} {disabled} on:keydown={onKeydown} type=checkbox>
  <div class=checkbox>
    {#if checked}
      <span class="check-mark material-icons">check</span>
    {/if}
  </div>
  {$format(label)}
</label>

<script lang=ts>
  import { format } from "$lib/errors";
  import type { Message } from "$lib/errors";


  export let checked = false;
  export let disabled = false;
  export let label: Message;

  const onKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Enter")
      checked = !checked;
  };
</script>

<style lang="scss">
  @use "$lib/styles/globals.scss" as g;

  $checkbox-size: 0.8rem;
  $checkbox-outline: 0.15rem;
  $check-mark-size: 1rem;
  $label-gap: 0.2rem;

  input {
    position: absolute;
    opacity: 0;

    &:focus + div {
      background-color: g.$blue;
    }
  }

  label {
    display: inline-flex;
    gap: $label-gap;
    align-items: start;
  }

  .checkbox {
    height: $checkbox-size;
    width: $checkbox-size;
    margin: $checkbox-outline;
    display: flex;
    justify-content: center;
    align-items: center;
    background-color: g.$black;
    outline: $checkbox-outline solid g.$white;
  }

  .check-mark {
    position: absolute;
    top: auto; bottom: auto;
    font-size: $check-mark-size;
  }
</style>
