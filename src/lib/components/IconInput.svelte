<div {style}>
  <input bind:this={input} class={classes} {placeholder} {type}>
  <span class=material-icons>{icon}</span>
</div>

<script lang="ts">
  let classes = "";
  export { classes as class };

  export let icon: string;
  export let placeholder = "";
  export let style = "";
  export let type = "text";

  let input: HTMLInputElement;

  export function focus(): void {
    if (input)
      input.focus();
  }
</script>

<style lang="scss">
  $input-placeholder-color: #888888 !default;
  $input-icon-color: $input-placeholder-color !default;
  $input-icon-focus-color: black !default;

  div {
    align-items: center;
    display: flex;
    position: relative;
  }

  input {
    height: 100%;
    padding-left: 2em;

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
    position: absolute;
  }
</style>
