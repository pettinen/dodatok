<svelte:window on:close-navbar={closeAccount} />

<nav class:logged-in={$user}>
  {#if $page.url.pathname !== config.pages.index}
    <a
      class="button home icon"
      href={config.pages.index}
      id=navbar-home-button
      title={$_("menu.home")}
      in:reveal|local={{ direction: "left", easing: cubicOut }}
      out:reveal|local={{ direction: "left", easing: cubicIn }}
    >
      <span class=material-icons>home</span>
    </a>
  {/if}

  {#if accountOpen}
    <div
      bind:this={accountModal}
      class=account-modal
      in:reveal={{ direction: narrow ? "bottom" : "left", easing: cubicOut }}
      out:reveal={{ direction: narrow ? "bottom" : "left", easing: cubicIn }}
      on:outroend={onAccountOutroEnd}
    >
      {#if !narrow}
        <button
          class="button close icon"
          title={$_("login.close")}
          on:click={closeAccount}
        >
          <span class=material-icons>chevron_right</span>
        </button>
      {/if}
      <div class=account-modal-inner>
        {#if $user}
          <div class=account-info>
            <div class=logged-in-as>
              {#if userIconState === "loaded"}
                <img
                  class=user-icon
                  src={$cacheURL($user.icon)}
                  alt={$_("account.user-icon")}
                >
              {/if}
              <span>{@html loggedInAs}</span>
            </div>

            {#if $page.url.pathname !== `${base}/account`}
              <a class="button manage-account" href={`${base}/account`}>
                {$_("menu.manage-account")}
              </a>
            {/if}
          </div>
        {:else}
          <div class=login-inputs>
            <div class=login-input class:disabled={loggingIn}>
              <IconTextInput
                bind:this={usernameInput}
                bind:value={username}
                disabled={loggingIn}
                error={$gotErrors(...errorSources.username)}
                icon=person
                placeholder={$_("login.username")}
                on:input={onUsernameInput}
                on:keydown={onUsernameKeydown}
              />
            </div>
            <div class=login-input class:disabled={loggingIn}>
              <IconPasswordInput
                bind:this={passwordInput}
                bind:value={password}
                disabled={loggingIn}
                error={$gotErrors(...errorSources.password)}
                placeholder={$_("login.password")}
                on:input={onPasswordInput}
                on:keydown={submitLogin}
              />
            </div>
          </div>

          {#if username.trim()}
            <div
              class=login-secondary-inputs
              in:reveal|local={{ direction: "bottom", easing: cubicOut }}
              out:reveal|local={{ direction: "bottom", easing: cubicIn }}
            >
              <div class="login-input login-secondary-input remember-me" class:disabled={loggingIn}>
                <Checkbox bind:checked={remember} disabled={loggingIn} label=login.remember-me />
              </div>
              <div class="login-input login-secondary-input" class:disabled={loggingIn}>
                <IconTextInput
                  bind:this={totpInput}
                  bind:value={totp}
                  disabled={loggingIn}
                  error={$gotErrors(...errorSources.totp)}
                  icon=key
                  inputmode=numeric
                  placeholder={$_("login.totp-placeholder")}
                  on:input={onTOTPInput}
                  on:keydown={submitLogin}
                />
              </div>
            </div>
          {:else}
            <div
              class=logged-out-buttons
              in:reveal|local={{ direction: "top", easing: cubicIn }}
              out:reveal|local={{ direction: "top", easing: cubicOut }}
            >
              <button class="button change-language" on:click={changeLanguage}>
                <span class="icon-before-text material-icons">public</span>
                {$_("menu.change-language")}
              </button>
              {#if $page.url.pathname !== `${base}/sign-up`}
                <a href={`${base}/sign-up`} class="button signup" title={$_("signup.sign-up")}>
                  <span class="icon-before-text material-icons">person_add</span>
                  {$_("signup.sign-up")}
                </a>
              {/if}
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}

  <div class=account-multi-button-container class:tall={accountOpen} id=navbar-menu-button>
    <button
      bind:this={multiButton}
      class="account-multi-button button icon"
      class:login={accountOpen && !$user}
      class:logout={accountOpen && $user}
      class:open={!accountOpen}
      disabled={loggingIn || loggingOut}
      title={multiButtonTitle}
      on:click={onMultiButtonClick}
    >
      {#if loggingIn || loggingOut || (userIconState === "loading" && userIconChanged)}
        <span
          class="account-multi-icon spinning"
          in:reveal|local={{ direction: "left" }}
          out:reveal|local={{ direction: "right" }}
        >
          <span class="account-multi-icon-inner material-icons">
            autorenew
          </span>
        </span>
      {:else if $user && accountOpen}
        <span
          class=account-multi-icon
          in:reveal|local={{ direction: "left" }}
          out:reveal|local={{ direction: "right" }}
        >
          <span class="account-multi-icon-inner material-icons">logout</span>
        </span>
      {:else if $user}
        <span
          class=account-multi-icon
          in:reveal|local={{ direction: "left" }}
          out:reveal|local={{ direction: "right" }}
        >
          {#if userIconState === "loaded"}
          <img
            class="account-multi-icon-inner user-icon"
            src={$cacheURL($user.icon)}
            alt={$_("account.user-icon")}
          >
          {:else}
            <span class="account-multi-icon-inner material-icons">person</span>
          {/if}
        </span>
      {:else if accountOpen}
        <span
          class=account-multi-icon
          in:reveal|local={{ direction: "left" }}
          out:reveal|local={{ direction: "right" }}
        >
          <span class="account-multi-icon-inner material-icons">login</span>
        </span>
      {:else}
        <span
          class=account-multi-icon
          in:reveal|local={{ direction: "left" }}
          out:reveal|local={{ direction: "right" }}
        >
          <span class="account-multi-icon-inner material-icons">menu</span>
        </span>
      {/if}
    </button>
  </div>

  {#if accountOpen}
  <div
    class=narrow-close-button-container
    class:hidden={!narrow}
    in:reveal|local={{ direction: "bottom", easing: cubicOut }}
    out:reveal|local={{ direction: "bottom", easing: cubicIn }}
  >
    <button
      class="button close icon narrow"
      title={$_("login.close")}
      on:click={closeAccount}
    >
      <span class=material-icons>expand_less</span>
    </button>
  </div>
  {/if}
</nav>

<script lang=ts>
  import { htmlEscape } from "escape-goat";
  import { tick } from "svelte";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { _, locale } from "svelte-i18n";

  import { browser } from "$app/env";
  import { goto } from "$app/navigation";
  import { base } from "$app/paths";
  import { page, navigating } from "$app/stores";

  import { cache, cacheURL } from "$lib/cache";
  import { config } from "$lib/config";
  import { errors, gotErrors, infoMessages, warnings } from "$lib/errors";
  import { accountSocket } from "$lib/sockets";
  import { reveal } from "$lib/transitions";
  import { validateCSRFTokenResponse, validateLoginResponse } from "$lib/types";
  import type { JSONObject } from "$lib/types";
  import { apiFetch, convertAPIUser, modals, unexpected, user, userIconURL } from "$lib/utils";
  import { validators } from "$lib/validation";

  import Checkbox from "$lib/components/Checkbox.svelte";
  import IconTextInput from "$lib/components/IconTextInput.svelte";
  import IconPasswordInput from "$lib/components/IconPasswordInput.svelte";


  // from login form: buttons: 2 * 3rem + inputs 2 * 20rem + gaps 3 * 1rem
  const NARROW_BREAKPOINT = "49rem";
  let narrow = false;

  const errorSources = {
    username: [/^validation\.errors\.login-username\./u, "auth.errors.invalid-credentials"],
    password: [/^validation\.errors\.login-password\./u, "auth.errors.invalid-credentials"],
    totp: [
      /^validation\.errors\.totp\./u,
      "auth.errors.invalid-totp",
      "auth.errors.totp-already-used",
      "auth.errors.totp-required",
    ],
  };

  let accountOutroEndHooks: Array<() => void> = [];
  let accountOpen = false;
  let loggingIn = false;
  let loggingOut = false;
  let multiButtonTitle = "";

  let username = "";
  let password = "";
  let totp = "";
  let remember = (browser && JSON.parse(localStorage.getItem("rememberMe") ?? "false")) || false;

  const initialUserIcon = $user?.icon;
  let userIconChanged = false;

  let accountModal: HTMLDivElement;
  let multiButton: HTMLButtonElement;
  let usernameInput: IconTextInput;
  let passwordInput: IconPasswordInput;
  let totpInput: IconTextInput;

  if (browser) {
    const mediaQuery = matchMedia(`(min-width: ${NARROW_BREAKPOINT})`);
    narrow = !mediaQuery.matches;
    mediaQuery.addEventListener("change", (event): void => {
      narrow = !event.matches;
    });
  }

  const accountOutroFinished = async (): Promise<void> => {
    await tick();
    if (!accountModal || !accountModal.style.animation)
      return Promise.resolve();
    return new Promise((resolve) => {
      accountOutroEndHooks.push(resolve);
    });
  };

  const changeLanguage = (): void => {
    if ($locale === "en-US")
      $locale = "fi-FI";
    else
      $locale = "en-US";
  };

  const closeAccount = (): void => {
    modals.remove("navbar");
    accountOpen = false;
  };

  const login = async (): Promise<void> => {
    if (loggingIn)
      return;

    username = username.trim();
    totp = totp.trim();
    $errors = [
      ...validators.username(username, "login-username", false),
      ...validators.password(password, "login-password", false),
      ...validators.totp(totp, true),
    ];
    if ($errors.length > 0)
      return;

    loggingIn = true;
    errors.clear();
    warnings.clear();

    const body: JSONObject = { username, password, remember };
    if (totp)
      body.totp = totp;

    const { data } = await apiFetch("/auth/login", validateLoginResponse, "POST", body);

    if (!data) {
      loggingIn = false;
      if ($gotErrors(...errorSources.password, /^csrf\.errors\./u)) {
        await tick();
        passwordInput.focus();
      } else if ($gotErrors(...errorSources.username)) {
        await tick();
        usernameInput.focus();
      } else if ($gotErrors(...errorSources.totp)) {
        await tick();
        totpInput.focus();
      }
      return;
    }

    if (data.user.icon)
      cache.load(userIconURL(data.user.icon));
    closeAccount();
    localStorage.setItem(config.csrfTokenField, data.csrfToken);

    if (config.pages.noAuthRequired.includes($page.url.pathname))
      goto(config.pages.index).catch(unexpected);

    username = "";
    password = "";
    totp = "";

    await accountOutroFinished();
    $user = convertAPIUser(data.user);
    loggingIn = false;
  };

  const logout = async (): Promise<void> => {
    if (loggingOut)
      return;
    loggingOut = true;

    const { data } = await apiFetch("/auth/logout", validateCSRFTokenResponse, "POST");

    if (!data) {
      loggingOut = false;
      return;
    }

    closeAccount();
    errors.clear();
    warnings.clear();
    infoMessages.clear();
    localStorage.setItem(config.csrfTokenField, data.csrfToken);
    accountSocket.destroy();

    if (config.pages.authRequired.includes($page.url.pathname))
      goto(config.pages.index).catch(unexpected);

    await accountOutroFinished();
    $user = null;
    loggingOut = false;
  };

  const onAccountOutroEnd = (): void => {
    for (const hook of accountOutroEndHooks)
      hook();
    accountOutroEndHooks = [];
  };

  const onMultiButtonClick = async (): Promise<void> => {
    if (multiButton)
      multiButton.blur();
    if (!accountOpen) {
      accountOpen = true;
      modals.push("navbar");
      if (!$user) {
        await tick();
        usernameInput?.focus();
      }
    } else if ($user) {
      logout().catch(unexpected);
    } else {
      login().catch(unexpected);
    }
  };

  const onUsernameInput = (): void => {
    errors.clear(...errorSources.username, ...errorSources.totp);
    const chars = [...username];
    if (chars.length > config.validationRules.username.maxLength)
      username = chars.slice(0, config.validationRules.username.maxLength).join("");
  };

  const onPasswordInput = (): void => {
    errors.clear(...errorSources.password, ...errorSources.totp);
    const chars = [...password];
    if (chars.length > config.validationRules.password.maxLength)
      password = chars.slice(0, config.validationRules.password.maxLength).join("");
  };

  const onUsernameKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Enter") {
      const validationErrors = validators.username(username, "login-username", false);
      if (validationErrors.length)
        $errors = validationErrors;
      else
        passwordInput?.focus();
    }
  };

  const onTOTPInput = (): void => {
    errors.clear(...errorSources.totp);
    const chars = [...totp];
    if (chars.length > config.totp.codeLength)
      totp = chars.slice(0, config.totp.codeLength).join("");
  };

  const submitLogin = (event: KeyboardEvent): void => {
    if (event.key === "Enter")
      login().catch(unexpected);
  };

  /* eslint-disable @typescript-eslint/indent */
  $: userIconState = ($user && $cache.get($user.icon)?.state) || "not-loaded";
  $: if ($user?.icon !== initialUserIcon)
       userIconChanged = true;
  $: loggedInAs = $user?.username && $_(
       "menu.logged-in-as",
       { values: { username: `<strong>${htmlEscape($user.username)}</strong>` } },
     );
  $:
    if (!$user)
      multiButtonTitle = $_("login.log-in");
    else if (accountOpen)
      multiButtonTitle = $_("menu.log-out");
    else
      multiButtonTitle = $_("menu.open-account-info");
  $: $navigating && closeAccount();
  $: browser && localStorage.setItem("rememberMe", JSON.stringify(remember));
  /* eslint-enable @typescript-eslint/indent */
</script>

<style lang=scss>
  @use "globals.scss" as g;

  $wide-input-width: 20rem;
  $narrow-breakpoint: 2 * $wide-input-width + 2 * g.$icon-button-size + 3 * g.$navbar-gap;

  .button {
    &.close {
      background-color: g.$red;
      height: 2 * g.$icon-button-size;

      &.narrow {
        height: calc(0.6 * g.$icon-button-size);
        pointer-events: auto;
        width: 100%;
      }
    }

    &.home {
      background-color: g.$green;
      position: absolute;
      right: g.$icon-button-size;
    }

    &.login {
      background-color: g.$green;
    }

    &.logout {
      background-color: g.$yellow;
    }

    &.open {
      background-color: g.$cyan;
    }
  }

  nav {
    align-items: start;
    display: flex;
    height: g.$navbar-height;
    justify-content: flex-end;
    left: 0;
    pointer-events: none;
    position: absolute;
    right: 0;

    * {
      pointer-events: auto;
    }

    &.logged-in {
      .account-modal {
        height: 2 * g.$navbar-height + 0.6 * g.$icon-button-size;
      }

      .narrow-close-button-container {
        height: 2 * g.$navbar-height + 0.6 * g.$icon-button-size;
      }

      .account-multi-button-container.tall {
        height: 2 * g.$navbar-height + 0.6 * g.$icon-button-size;
      }
    }

    .account-modal {
      display: flex;
      flex-wrap: wrap;
      gap: g.$navbar-gap;
      height: calc(3 * g.$navbar-height + 0.6 * g.$icon-button-size);
      justify-content: start;
      position: absolute;
      right: 0;
      top: 0;
      width: 100vw;
      z-index: 1;
    }

    .account-info {
      display: flex;
      flex-direction: column;
      gap: g.$navbar-gap;
      height: 2 * g.$navbar-height - 2 * g.$navbar-gap;
      justify-content: center;
      padding: g.$navbar-gap;
      width: calc(100% - g.$icon-button-size - 2 * g.$navbar-gap);

      .button {
        background-color: g.$magenta;
        height: 0.6 * g.$icon-button-size;
      }

      .user-icon {
        width: 1.2rem;
        height: 1.2rem;
      }
    }

    .login-input {
      padding-left: g.$navbar-gap;
      transition: opacity 400ms;

      &.disabled {
        opacity: 0.5;
      }
    }

    .login-inputs {
      display: flex;
      flex-direction: column;
      gap: g.$navbar-gap;
      height: 2 * g.$navbar-height - 2 * g.$navbar-gap;
      justify-content: space-between;
      padding: g.$navbar-gap 0;
      width: calc(100% - g.$icon-button-size - g.$navbar-gap);
    }

    .logged-out-buttons {
      display: flex;
      height: g.$navbar-height;
      position: absolute;
      width: 100%;

      .button {
        flex: 1;
      }

      .change-language {
        background-color: g.$yellow;
      }

      .signup {
        background-color: g.$magenta;
      }
    }

    .login-secondary-input {
      display: flex;
      align-items: center;
      padding: 0 !important;
      width: calc(50% - 0.5 * g.$navbar-gap);
    }

    .login-secondary-inputs {
      display: flex;
      gap: g.$navbar-gap;
      height: g.$navbar-height;
      padding: 0 g.$navbar-gap;
      position: absolute;
      width: calc(100% - 2 * g.$navbar-gap);
    }

    .remember-me {
      justify-content: center;
    }

    .account-modal-inner {
      background: g.$account-background-color;
      width: 100%;
    }

    .account-multi-button-container {
      display: flex;
      flex-direction: column;
      height: 0;
      position: absolute;
      transition: height 400ms cubic-bezier(0.32, 0, 0.67, 0);

      &.tall {
        height: calc(3 * g.$navbar-height + 0.6 * g.$icon-button-size);
        transition: height 400ms cubic-bezier(0.33, 1, 0.68, 1);
      }
    }

    .account-multi-button {
      height: 100%;
      min-height: g.$navbar-height;
      max-height: calc(2 * g.$icon-button-size);
      transition: background-color 400ms;
      z-index: 3;
    }

    .account-multi-icon {
      overflow: hidden;
      padding: 0.1rem;
      position: absolute;
      transform: rotate(-45deg);
    }

    .account-multi-icon-inner {
      font-size: g.$icon-button-font-size;
      height: g.$icon-button-font-size;
      transform: rotate(45deg);
      width: g.$icon-button-font-size;
    }

    .logged-in-as {
      align-items: center;
      display: flex;
      gap: 0.5 * g.$navbar-gap;
    }

    .narrow-close-button-container {
      display: flex;
      flex-direction: column;
      height: 3 * g.$navbar-height + 0.6 * g.$icon-button-size;
      justify-content: end;
      pointer-events: none;
      width: 100vw;
      z-index: 1;
    }

    .user-icon {
      border-radius: 100%;
    }

    @media (min-width: $narrow-breakpoint) {
      .account-modal {
        flex-wrap: nowrap;
        gap: 0;
        height: 2 * g.$navbar-height !important;
        width: unset;

        button.close {
          width: g.$icon-button-size;
        }

        .account-info {
          height: 2 * g.$navbar-height;
          margin-right: g.$icon-button-size;
          max-width: $narrow-breakpoint - 2 * g.$icon-button-size - 2 * g.$navbar-gap;
          min-width: $wide-input-width;
          padding-bottom: 0;
          padding-top: 0;
        }

        .login-inputs {
          align-items: center;
          flex-direction: row;
          height: g.$navbar-height;
          justify-content: start;
          margin-right: g.$icon-button-size;
          padding: 0 g.$navbar-gap;
          width: calc(100% - g.$navbar-gap);

          .login-input {
            padding: 0;
            width: 20rem;
          }
        }
      }

      .account-modal-inner {
        width: calc(100% - 2 * g.$icon-button-size);
      }

      .logged-out-buttons {
        position: absolute;
        width: calc(100% - 2 * g.$icon-button-size);
      }

      .login-secondary-inputs {
        position: absolute;
        width: calc(100% - 2 * g.$icon-button-size);
        padding: 0 1rem;
      }

      .login-secondary-input {
        width: calc((100% - g.$icon-button-size) / 2);
      }
    }
  }
</style>
