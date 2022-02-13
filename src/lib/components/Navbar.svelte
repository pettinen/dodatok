<nav class:logged-in={loggedIn}>
  {#if $page.url.pathname !== `${base}/`}
    <a
      class="button home icon"
      href=/
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
        {#if loggedIn}
          <div class=account-info>
            <div class=logged-in-as>
              {#if $session.user.icon}
                <img
                  class=user-icon
                  src={userIcon}
                  alt={$_("menu.user-icon")}
                >
              {/if}
              <span>{@html loggedInAs}</span>
            </div>
            <a class="button manage-account" href=/account>
              {$_("menu.manage-account")}
            </a>
          </div>
        {:else}
          <div class=login-inputs>
            <div class=login-input class:disabled={loggingIn}>
              <IconTextInput
                bind:this={usernameInput}
                bind:value={username}
                disabled={loggingIn}
                error={gotErrors("login.username")}
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
                error={gotErrors("login.password")}
                placeholder={$_("login.password")}
                on:input={onPasswordInput}
                on:keydown={onPasswordKeydown}
              />
            </div>
          </div>
          <div
            class=logged-out-buttons
          >
            <button class="button change-language" on:click={changeLanguage}>
              <span class="icon-before-text material-icons">public</span>
              {$_("menu.change-language")}
            </button>
            {#if $page.url.pathname !== `${base}/sign-up`}
              <a href=/sign-up class="button signup" title={$_("signup.sign-up")}>
                <span class="icon-before-text material-icons">person_add</span>
                {$_("signup.sign-up")}
              </a>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <div class=account-multi-button-container class:tall={accountOpen}>
    <button
      bind:this={multiButton}
      class="account-multi-button button icon"
      class:login={accountOpen && !loggedIn}
      class:logout={accountOpen && loggedIn}
      class:open={!accountOpen}
      disabled={loggingIn || loggingOut}
      title={multiButtonTitle}
      on:click={onMultiButtonClick}
    >
      {#if loadingUserIcon || loggingIn || loggingOut}
        <span
          class="account-multi-icon spinning"
          in:reveal|local={{ direction: "left" }}
          out:reveal|local={{ direction: "right" }}
        >
          <span class="account-multi-icon-inner material-icons">
            autorenew
          </span>
        </span>
      {:else if loggedIn && accountOpen}
        <span
          class=account-multi-icon
          in:reveal|local={{ direction: "left" }}
          out:reveal|local={{ direction: "right" }}
        >
          <span class="account-multi-icon-inner material-icons">logout</span>
        </span>
      {:else if loggedIn}
        <span
          class=account-multi-icon
          in:reveal|local={{ direction: "left" }}
          out:reveal|local={{ direction: "right" }}
        >
          <img
            class="account-multi-icon-inner user-icon"
            src={userIcon}
            alt={$_("menu.user-icon")}
          >
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
  import escapeHTML from "escape-html";
  import { tick } from "svelte";
  import { cubicIn, cubicOut } from "svelte/easing";
  import { _, locale } from "svelte-i18n";

  import { browser } from "$app/env";
  import { assets, base } from "$app/paths";
  import { page, navigating, session } from "$app/stores";

  import { cache } from "$lib/cache";
  import IconTextInput from "$lib/components/IconTextInput.svelte";
  import IconPasswordInput from "$lib/components/IconPasswordInput.svelte";
  import { errors, LoginError, UnexpectedError } from "$lib/errors";
  import { reveal } from "$lib/transitions";
  import { passwordRules, usernameRules, validate } from "$lib/validation";

  // from login form: buttons: 2 * 3rem + inputs 2 * 20rem + gaps 3 * 1rem
  const NARROW_BREAKPOINT = "49rem";

  let accountOutroEndHooks: Array<() => void> = [];
  let accountModal: HTMLDivElement;
  let accountOpen = false;
  let loggingIn = false;
  let loggingOut = false;
  let multiButton: HTMLButtonElement;
  let narrow = false;
  let password = "";
  let passwordInput: IconPasswordInput;
  let username = "";
  let usernameInput: IconTextInput;

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
    accountOpen = false;
  };

  const login = async (): Promise<void> => {
    if (loggingIn)
      return;
    loggingIn = true;

    username = username.trim();
    $errors = [
      ...validate("login", "login.username", username, usernameRules),
      ...validate("login", "login.password", password, passwordRules),
    ];
    if ($errors.length > 0) {
      loggingIn = false;
      return;
    }

    try {
      await new Promise((resolve): void => {
        const LOGIN_WAIT = 2000;
        setTimeout(resolve, LOGIN_WAIT);
      });
      if (username.toLowerCase() !== "a" || password !== "b")
        throw new LoginError("not-found");
      closeAccount();
      username = "";
      password = "";
      await accountOutroFinished();
      $session = {
        ...$session,
        user: {
          icon: "https://reqres.in/img/faces/8-image.jpg",
          id: "1ac91374-cfd0-44c3-a19f-03933313b669",
          username: "aho",
        },
      };
      void cache.load($session.user.icon);
    } catch (error: unknown) {
      const shownError = error instanceof LoginError
        ? error
        : new UnexpectedError("login");
      $errors = [{ source: "login", text: shownError.text }];
    }
    loggingIn = false;
  };

  const logout = async (): Promise<void> => {
    if (loggingOut)
      return;
    loggingOut = true;
    try {
      await new Promise((resolve): void => {
        const LOGOUT_WAIT = 500;
        setTimeout(resolve, LOGOUT_WAIT);
      });
      closeAccount();
      await accountOutroFinished();
      $session = {
        ...$session,
        user: null,
      };
    } catch {
      $errors = [{ source: "logout", text: $_("logout.fail") }];
    }
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
      if (!loggedIn) {
        await tick();
        usernameInput?.focus();
      }
    } else if (loggedIn) {
      void logout();
    } else {
      void login();
    }
  };

  const onPasswordInput = (): void => {
    errors.clear("login.password");
    const chars = [...password];
    if (chars.length > passwordRules.maxLength)
      password = chars.slice(0, passwordRules.maxLength).join("");
  };

  const onPasswordKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Enter")
      void login();
  };

  const onUsernameInput = (): void => {
    errors.clear("login.username");
    const chars = [...username];
    if (chars.length > usernameRules.maxLength)
      username = chars.slice(0, usernameRules.maxLength).join("");
  };

  const onUsernameKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Enter") {
      const validationErrors = validate("login", "login.username", username, usernameRules);
      if (validationErrors.length)
        $errors = validationErrors;
      else
        passwordInput?.focus();
    }
  };

  $: cacheURL = (url: string): string | null => $cache.get(url)?.url || null;
  $: gotErrors = (containsSource: string): boolean =>
    $errors.some(({ source }) => containsSource === source);
  $: loadingUserIcon = $cache.get($session.user?.icon)?.state === "loading";
  $: loggedIn = Boolean($session.user);
  $: loggedInAs = $session.user?.username && $_(
    "menu.logged-in-as",
    { values: { username: `<strong>${escapeHTML($session.user.username)}</strong>` } },
  );
  $: multiButtonTitle = ((): string => {
    if (!loggedIn)
      return $_("login.log-in");
    if (accountOpen)
      return $_("menu.log-out");
    return $_("menu.open-account-info");
  })();
  $: $navigating && closeAccount();
  $: userIcon = cacheURL($session.user?.icon) || cacheURL(`${assets}/default-avatar.png`);
</script>

<style lang=scss>
  @use "globals.scss" as g;

  $wide-input-width: 20rem;
  $breakpoint: 2 * $wide-input-width + 2 * g.$icon-button-size + 3 * g.$navbar-gap;

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

      .account-info, .login-inputs {
        height: 2 * g.$navbar-height - 2 * g.$navbar-gap;
        width: calc(100% - g.$icon-button-size - 2* g.$navbar-gap);
      }

      .account-info {
        display: flex;
        flex-direction: column;
        gap: g.$navbar-gap;
        justify-content: center;
        padding: g.$navbar-gap;

        .button {
          background-color: g.$magenta;
          height: 0.6 * g.$icon-button-size;
        }

        .user-icon {
          width: 1.2rem;
          height: 1.2rem;
        }
      }

      .login-inputs {
        display: flex;
        flex-direction: column;
        gap: g.$navbar-gap;
        justify-content: space-between;
        padding: g.$navbar-gap 0;

        .login-input {
          padding-left: g.$navbar-gap;
          transition: opacity 400ms;

          &.disabled {
            opacity: 0.5;
          }
        }
      }

      .logged-out-buttons {
        display: flex;
        height: g.$navbar-height;

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

      button {
        height: 100%;
        min-height: g.$navbar-height;
        max-height: calc(2 * g.$icon-button-size);
        transition: background-color 400ms;
        z-index: 3;

        .account-multi-icon {
          overflow: hidden;
          padding: 0.1rem;
          position: absolute;
          transform: rotate(-45deg);

          .account-multi-icon-inner {
            font-size: g.$icon-button-font-size;
            height: g.$icon-button-font-size;
            transform: rotate(45deg);
            width: g.$icon-button-font-size;
          }

          &.spinning span {
            @keyframes spin {
              from {
                transform:rotate(0deg);
              }
              to {
                transform:rotate(360deg);
              }
            }

            animation-name: spin;
            animation-duration: 2s;
            animation-iteration-count: infinite;
            animation-timing-function: linear;
          }
        }
      }
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

    @media (min-width: $breakpoint) {
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
          max-width: $breakpoint - 2 * g.$icon-button-size - 2 * g.$navbar-gap;
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
    }
  }
</style>
