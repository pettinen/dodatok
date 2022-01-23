<nav>
  {#if loginOpen}
    <div bind:this={loginForm} class="login pure-form" transition:flyFullWidth>
      <input placeholder="Username">
      <input placeholder="Password">
    </div>
  {/if}
  <button bind:this={loginButton} class="pure-button pure-button-primary" on:click={toggleLogin}>
    {#key loginOpen}
      <span class="icon" in:in_={loginButtonKey} out:out={loginButtonKey}>
        <span class=material-icons>{loginOpen ? "close" : "login"}</span>
      </span>
    {/key}
</nav>

<slot />

<script lang="ts">
  import "$lib/styles/globals.sass";
  import "material-icons/iconfont/material-icons.css";
  import "purecss";

  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";

  //import Menu from "$lib/Menu.svelte";

  let loginButton: HTMLButtonElement;
  let loginForm: HTMLDivElement;

  let loginOpen: boolean = false;
  const toggleLogin = () => { loginOpen = !loginOpen; };
  const loginButtonKey = Symbol();

  let loginFormFlyDistance = 0;

  onMount(() => {
    const loginButtonRect = loginButton.getBoundingClientRect();
    const loginFormRect = loginForm.getBoundingClientRect();
    loginFormFlyDistance = loginButtonRect.width + loginFormRect.width;
  });

  const [in_, out] = function revealFromRight() {
    const sending = new Map<string | symbol, DOMRect>();
    const receiving = new Map<string | symbol, DOMRect>();

    function reveal(node: HTMLElement, in_: boolean) {
      const rect = node.getBoundingClientRect();
      let css: (number, number) => string;
      if (in_)
        css = (t, u) => `clip: rect(auto, auto, auto, ${u * rect.width}px);`;
      else {
        css = (t, u) => `clip: rect(auto, ${t * rect.width}px, auto, auto);`;
      }
      return {
        css,
        delay: 0,
        duration: 500,
      };
    }

    function transition(items: any, counterparts: any, in_: boolean) {
      return (node: HTMLElement) => {
        return reveal(node, in_);
      };
    }

    return [transition(sending, receiving, true), transition(sending, receiving, false)];
  }();

  const flyFullWidth = function(node, params) {
    const style = getComputedStyle(node);
    console.log(style, params);
    return {
      css: (t, u) => `transform: translateX(${100 * u}%);`,
      delay: 0,
      duration: 200,
    };
  }

</script>

<style lang="sass">
  $navbar-height: 3rem

  :global
    body, html
      height: 90%
      margin: 0
      padding: 0
      width: 90%

  nav
    align-items: center
    display: flex
    height: $navbar-height
    justify-content: flex-end
    position: absolute
    top: 0
    right: 0
    padding: 0
    margin: 0
  button
    margin: 0
    padding: 0
    align-items: center
    border-radius: 0
    display: flex
    height: $navbar-height
    justify-content: center
    width: $navbar-height
    z-index: 10
  .icon
    position: absolute
    transform: rotate(-45deg)
  .login
    align-items: center
    display: flex
    background-color: #444444
    height: $navbar-height
    padding: 0 15px 0 10px

    input
      height: 80%
  .material-icons
    font-size: calc(0.6 * $navbar-height)
    transform: rotate(45deg)
</style>
