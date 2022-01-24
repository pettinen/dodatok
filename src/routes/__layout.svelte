<svelte:window on:load={onWindowLoad} />

{#if $isLoading}
  chotto matte
{:else}
  <nav>
    {#if loginOpen}
      <div
        class=login-modal
        in:reveal={{ direction: narrow ? "bottom" : "left", duration: 200 }}
        out:reveal={{ direction: narrow ? "bottom" : "left", duration: 200 }}
      >
        {#if !narrow}
          <button
            class="close pure-button"
            on:click={closeLogin}
            title={$_("login.close")}
          >
            <span class=material-icons>chevron_right</span>
          </button>
        {/if}
        <div class=login-inputs>
          <div class=login-input>
            <IconTextInput
              bind:this={usernameInput}
              bind:value={username}
              icon=person
              placeholder={$_("login.username")}
              on:input={handleUsernameInput}
              on:keydown={handleUsernameKeydown}
            />
          </div>
          <div class=login-input>
            <IconPasswordInput
              bind:this={passwordInput}
              bind:value={password}
              icon=password
              placeholder={$_("login.password")}
              on:keydown={handlePasswordInput}
              on:keydown={handlePasswordKeydown}
            />
          </div>
        </div>
        {#if narrow}
          <button
            class="close narrow pure-button"
            tabindex=1
            title={$_("login.close")}
            on:click={closeLogin}
          >
            <span class=material-icons>expand_less</span>
          </button>
        {/if}
      </div>
    {/if}
    <button
      class="pure-button open-or-login-button"
      class:login={loginOpen}
      class:tall={narrow && loginOpen}
      class:open={!loginOpen}
      on:click={loginOrOpenLogin}
    >
      {#key loginOpen}
        <span class=open-or-login-icon in:reveal={{ direction: "left" }} out:reveal={{ direction: "right" }}>
          <span class=material-icons>{loginOpen ? "login" : "person"}</span>
        </span>
      {/key}
    </button>
  </nav>

  <!--
  {#each ["magenta", "yellow", "green", "blue", "cyan", "red"] as color}
  <div class={color} style="height: 6rem; width: 100vw;">{color}</div>
  {/each}
  -->
  <p>{$_("test test")} narrow: {narrow}</p>

  <slot />

  <p>Errors: {errors.join(",  ")}</p>
{/if}

<script lang="ts">
  import "$lib";
  import { tick, onMount } from "svelte";
  import { _, isLoading } from "svelte-i18n";

  import { browser } from "$app/env";

  import IconTextInput from "$lib/components/IconTextInput.svelte";
  import IconPasswordInput from "$lib/components/IconPasswordInput.svelte";
  import { reveal } from "$lib/transitions";
  import { passwordRules, usernameRules, validate } from "$lib/validation";

  import "$lib/styles/main.scss";
  import "@fontsource/fira-sans/latin.css";
  import "purecss";
  import "material-icons/iconfont/material-icons.css";

  let errors: string[] = [];
  let loginOpen: boolean = false;
  let narrow = false;
  let username = "";
  let usernameInput: IconTextInput;
  let password = "";
  let passwordInput: IconPasswordInput;

  if (browser) {
    const mediaQuery = matchMedia(`(min-width: 49rem)`);
    narrow = !mediaQuery.matches;
    mediaQuery.addEventListener("change", (event) => {
      narrow = !event.matches;
    });
  }

  const handlePasswordInput = (event: InputEvent): void => {
    const chars = [...password];
    if (chars.length > passwordRules.maxLength)
      password = chars.slice(0, passwordRules.maxLength).join("");
  };

  const handlePasswordKeydown = (event: KeyboardEvent): void => {
    console.log("keydown");
    if (event.key === "Enter")
      login();
  };

  const handleUsernameInput = (event: InputEvent): void => {
    const chars = [...username];
    if (chars.length > usernameRules.maxLength)
      username = chars.slice(0, usernameRules.maxLength).join("");
  };

  const handleUsernameKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Enter" && passwordInput)
      passwordInput.focus();
  };

  const closeLogin = (): void => {
    loginOpen = false;
  };

  const login = async () => {
    validateLoginInputs();
    console.log(errors);
    if (errors.length)
      return;
    alert("lol");
  };

  const loginOrOpenLogin = async (): Promise<void> => {
    if (!loginOpen) {
      loginOpen = true;
      await tick();
      if (usernameInput)
        usernameInput.focus();
    } else {
      login();
    }
  };

  const onWindowLoad = () => {
    document.body.classList.remove("preload");
  };

  const validateLoginInputs = (): boolean => {
    errors = [];
    username = username.trim();
    errors.push(...validate(username, "login.username", usernameRules));
    errors.push(...validate(password, "login.password", passwordRules));
  }

</script>

<style lang="scss">
  @use "globals.scss" as g;

  nav {
    align-items: start;
    display: flex;
    height: g.$navbar-height;
    justify-content: flex-end;
    position: absolute;
    width: 100vw;

    button {
      align-items: center;
      border-radius: 0;
      display: flex;
      height: g.$navbar-height;
      justify-content: center;
      line-height: 0;
      padding: 0;
      width: g.$navbar-height;
      z-index: 10;

      &.open-or-login-button {
        transition: background-color 400ms, height 200ms;

        &.tall {
          height: calc(2 * g.$navbar-height);
        }

        .open-or-login-icon {
          overflow: hidden;
          padding: 0.1rem;
          position: absolute;
          transform: rotate(-45deg);

          span {
            font-size: g.$login-icon-size;
            transform: rotate(45deg);
          }
        }
      }

      &.close {
        background-color: g.$red;
      }

      &.open {
        background-color: g.$cyan;
      }

      &.login {
        background-color: g.$green;
      }
    }

    .login-modal {
      background-color: g.$black;
      display: flex;
      flex-direction: column;
      gap: g.$navbar-gap;
      height: calc(2.5 * g.$navbar-height);
      justify-content: start;
      left: 0;
      padding-right: g.$navbar-gap;
      position: absolute;
      right: g.$navbar-height;
      top: 0;
      width: 100%;

      button.close {
        height: calc(0.5 * g.$navbar-height);

        &.narrow {
          width: 100%;
        }
      }

      .login-inputs {
        display: flex;
        flex-direction: column;
        gap: g.$navbar-gap;
        height: calc(2 * g.$navbar-height);
        justify-content: end;

        .login-input {
          padding: 0 g.$navbar-gap;
          width: calc(100vw - g.$navbar-height - 2 * g.$navbar-gap);
        }
      }
    }

    @media (min-width: 49rem) {
      .login-modal {
        flex-direction: row;
        height: g.$navbar-height;
        left: unset;
        width: unset;

        button.close {
          height: g.$navbar-height;
        }

        .login-inputs {
          align-items: center;
          flex-direction: row;
          height: g.$navbar-height;

          .login-input {
            padding: 0;
            width: 20rem;
          }
        }
      }
    }
  }

  .red { background-color: g.$red; }
  .green { background-color: g.$green; }
  .yellow { background-color: g.$yellow; }
  .blue { background-color: g.$blue; }
  .magenta { background-color: g.$magenta; }
  .cyan { background-color: g.$cyan; }
</style>
