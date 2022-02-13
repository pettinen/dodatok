<div>
  {#if visible}
    <input bind:this={input} bind:value {...attrs} on:input on:keydown>
  {:else}
    <input
      bind:this={input}
      bind:value
      type=password
      {...attrs}
      on:input
      on:keydown
    >
  {/if}
  <span class="material-icons password-icon" class:error>{icon}</span>
  <span
    role="button"
    class="material-icons visibility-icon"
    on:click={toggleVisible}
  >
    {visible ? "visibility_off" : "visibility"}
  </span>
</div>

<script lang=ts>
  import { tick } from "svelte";

  export let disabled = false;
  export let error = false;
  export let icon = "password";
  export let placeholder = "";
  export let value = "";

  let input: HTMLInputElement;
  let visible = false;

  $: attrs = {
    disabled,
    placeholder,
  };

  export const focus = (): void => {
    if (input)
      input.focus();
  };

  const toggleVisible = async (): Promise<void> => {
    let selectionStart: number | null = null;
    let selectionEnd: number | null = null;
    if (input)
      ({ selectionStart, selectionEnd } = input);
    visible = !visible;
    if (input) {
      await tick();
      input.setSelectionRange(selectionStart, selectionEnd);
      input.focus();
    }
  };
</script>

<style lang=scss>
  @use "globals.scss" as g;

  $input-color: g.$white;
  $input-line-color: g.$yellow;
  $input-placeholder-color: g.$yellow;
  $input-icon-color: $input-placeholder-color;
  $input-icon-focus-color: $input-color;
  $input-icon-padding: 2.5em;

  div {
    position: relative;
  }

  input {
    background-color: transparent;
    border: none;
    border-bottom: 1px solid $input-line-color;
    color: $input-color;
    height: 100%;
    outline: 0;
    padding-bottom: 0.25em;
    padding-left: $input-icon-padding;
    padding-right: $input-icon-padding;
    width: calc(100% - 2 * $input-icon-padding);

    &::placeholder {
      color: $input-placeholder-color;
    }

    &:focus + span {
      color: $input-icon-focus-color;
    }
  }

  .material-icons {
    color: $input-icon-color;
    position: absolute;

    &.password-icon {
      bottom: 0.1em;
      font-size: 1.5em;
      left: 0.25em;

      &.error {
        color: g.$red !important;
      }
    }

    &.visibility-icon {
      bottom: 0.15em;
      cursor: pointer;
      font-size: 1em;
      padding: 0.25em;
      right: 0;
    }
  }
</style>
