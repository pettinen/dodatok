<nav>
  {#if loginOpen}
    <div class="login" transition:reveal={{ direction: "left", duration: 200 }}>
      <IconInput bind:this={usernameInput} icon=person style="height: 50%;" placeholder={$_("login.username")} />
      <IconInput type=password icon=password style="height: 50%;" placeholder={$_("login.password")} />
    </div>
  {/if}
  <button class="pure-button pure-button-primary" on:click={toggleLogin}>
    {#key loginOpen}
      <span class="login-icon" in:in_ out:out>
        <span class=material-icons>{loginOpen ? "close" : "login"}</span>
      </span>
    {/key}
</nav>

<slot />

<script lang="ts">
  import "$lib/i18n";
  import { tick } from "svelte";
  import { _ } from "svelte-i18n";

  import IconInput from "$lib/components/IconInput.svelte";

  import "@fontsource/fira-sans/latin.css";
  import "purecss";
  import "material-icons/iconfont/material-icons.css";

  //import Menu from "$lib/Menu.svelte";

  let loginOpen: boolean = false;
  let usernameInput: IconInput;
  const toggleLogin = async (): void => {
    loginOpen = !loginOpen;
    if (loginOpen) {
      await tick();
      if (usernameInput)
        usernameInput.focus();
    }
  };

  interface RevealParams {
    direction: "top" | "right" | "bottom" | "left";
    delay?: number;
    duration?: number;
    reverse?: boolean;
  }

  const reveal = function(
    node: HTMLElement,
    { direction, delay = 0, duration = 300, reverse = false }: RevealParams
  ) {
    const width = node.getBoundingClientRect().width;
    let directionIndex: number;
    switch (direction) {
      case "top":
        directionIndex = 0;
        break;
      case "right":
        directionIndex = 1;
        break;
      case "bottom":
        directionIndex = 2;
        break;
      case "left":
        directionIndex = 3;
        break;
      default:
        throw new Error(`invalid direction: ${direction}`)
    }
    const css = (t: number, u: number) => {
      const rectParams = ["auto", "auto", "auto"];
      rectParams.splice(directionIndex, 0, `${(reverse ? t : u) * width}px`);
      return `clip: rect(${rectParams.join(", ")})`;
    }
    return { css, delay, duration };
  };

  const [in_, out] = function crossReveal() {
    const transition = (in_: boolean) => {
      return (node: HTMLElement, _params: {}) => (
        reveal(node, { direction: in_ ? "left": "right", reverse: !in_ })
      );
    }
    return [transition(true), transition(false)];
  }();
</script>

<style lang="scss">
  $navbar-height: 5rem;
  $login-icon-size: calc(0.6 * $navbar-height);

  :global {
    :root {
      font-size: 62.5%;
    }
    body {
      font-family: "Fira Sans";
      font-size: 1.6rem;
      font-weight: 300;
    }
  }

  nav {
    align-items: center;
    display: flex;
    height: $navbar-height;
    justify-content: flex-end;
    position: absolute;
    top: 0;
    right: 0;

    button {
      align-items: center;
      border-radius: 0;
      display: flex;
      height: $navbar-height;
      justify-content: center;
      width: $navbar-height;
      z-index: 10;

      .login-icon {
        overflow: hidden;
        position: absolute;
        transform: rotate(-45deg);

        span {
          font-size: $login-icon-size;
          transform: rotate(45deg);
        }
      }
    }

    .login {
      align-items: center;
      display: flex;
      background-color: #444444;
      height: $navbar-height;
      padding: 0 1rem;
      position: absolute;
      right: $navbar-height;
    }
  }
</style>
