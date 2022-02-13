<div>
  <input
    bind:this={input}
    bind:value
    {...attrs}
    on:input
    on:keydown
  >
  <span class=material-icons class:error>{icon}</span>
</div>

<script lang=ts>
  export let disabled = false;
  export let error = false;
  export let icon: string;
  export let placeholder = "";
  export let value = "";

  let input: HTMLInputElement;

  $: attrs = {
    disabled,
    placeholder,
  };

  export const focus = (): void => {
    if (input)
      input.focus();
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
    width: calc(100% - $input-icon-padding);

    &::placeholder {
      color: $input-placeholder-color;
    }

    &:focus + span {
      color: $input-icon-focus-color;
    }
  }

  span {
    color: $input-icon-color;
    font-size: 1.5em;
    left: 0.25em;
    bottom: 0.1em;
    position: absolute;

    &.error {
      color: g.$red !important;
    }
  }
</style>
