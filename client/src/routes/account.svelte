<svelte:window on:close-account-current-password={() => { currentSubmitFunction = null; }} />

{#if $session.user}
  {#if currentSubmitFunction}
    <div class=current-password>
      <div class=current-password-modal>
        {#if currentSubmitFunction === deleteAccount}
          <p>{$_("account.current-password-required-to-delete-account")}</p>
        {:else}
          <p>{$_("account.current-password-required")}</p>
        {/if}
        <IconPasswordInput
          bind:this={currentPasswordInput}
          bind:value={currentPassword}
          disabled={inputsDisabled}
          error={$gotErrors(...errorSources.currentPassword)}
          on:input={onCurrentPasswordInput}
          on:keydown={onCurrentPasswordKeydown}
        />
        {#if currentSubmitFunction === deleteAccount}
          <button
            class="button icon-text delete-account-button"
            disabled={inputsDisabled}
            on:click={submitWithCurrentPassword}
          >
            <span class=material-icons>delete_forever</span> {$_("account.delete-account")}
          </button>
        {:else}
          <button
            class="button icon-text current-password-submit"
            disabled={inputsDisabled}
            on:click={submitWithCurrentPassword}
          >
            <span class=material-icons>save</span> {$_("account.save-changes")}
          </button>
        {/if}
      </div>
      <div
        class=current-password-background
        on:click={() => { currentSubmitFunction = null; }}
      ></div>
    </div>
  {/if}

  <main>
    <h1
      bind:this={heading}
      class=main-heading
      class:avoid-navbar-intersection={moveHeadingDown}
    >
      {$_("account.your-account")}
    </h1>

    <div class=basic-info>
      {#if $userIcon}
        <div class=user-icon>
          <span class=username>{$session.user.username}</span>
          <label
            bind:this={iconLabel}
            class=user-icon-container
            on:dragenter={onIconDragenter}
            on:dragleave={() => { iconDragover = false; }}
            on:dragover={(event) => { event.preventDefault(); }}
            on:drop|preventDefault={onIconDrop}
          >
            <input
              class=hidden
              bind:files={iconFiles}
              accept={config.acceptedImageTypes.join(",")}
              disabled={inputsDisabled}
              type=file
            >
            <img class=user-icon-image src={$userIcon} alt={$_("account.user-icon")}>
            {#if submittingUserIcon}
              <div class="user-icon-overlay spinning">
                <span class=material-icons>autorenew</span>
              </div>
            {:else if iconDragover}
              <div class=user-icon-overlay>{$_("account.drop-file")}</div>
            {:else}
              <div class="user-icon-overlay hover">
                <span class=material-icons>file_upload</span>
              </div>
            {/if}
          </label>
          {#if $session.user.icon}
            <button class="remove-icon link" on:click={removeIcon}>
              {$_("account.remove-icon")}
            </button>
          {/if}
        </div>
      {/if}

      <div class=basic-info-text-fields>
        <IconTextInput
          bind:this={usernameInput}
          bind:value={username}
          disabled={inputsDisabled}
          error={$gotErrors(...errorSources.username) || usernameUnavailable}
          errorMessage={usernameUnavailable ? "account.username-unavailable" : ""}
          icon=person
          placeholder={$_("account.change-username")}
          on:input={onUsernameInput}
        />

        <IconPasswordInput
          bind:this={newPasswordInput}
          bind:value={newPassword}
          disabled={inputsDisabled}
          error={$gotErrors(...errorSources.newPassword)}
          placeholder={$_("account.change-password")}
          on:input={onNewPasswordInput}
        />

        <button
          class="button icon-text save-username-password"
          class:hidden={narrow && !textFieldsChanged}
          class:invisible={!narrow && !textFieldsChanged}
          disabled={inputsDisabled || usernameUnavailable}
          on:click={() => updateUsernamePassword().catch(catchError)}
        >
          <span class=material-icons>save</span>
          {$_(saveUsernamePasswordText)}
        </button>
      </div>
    </div>

    <div class=locale>
      <h2 class="subheading icon-heading">
        <span class=material-icons>language</span>
        {$_("account.locale")}
      </h2>
      <div class=locale-buttons>
        {#each Object.entries(config.locales) as [id, name]}
          <button
            class="button text locale-button"
            class:selected={selectedLocale === id}
            disabled={inputsDisabled}
            on:click={() => { updateLocale(id); }}
          >
            {name}
          </button>
        {/each}
      </div>
    </div>

    <div class=mfa class:mfa-open={totpKey}>
      {#if $session.user.totpEnabled}
      <div>
        <button
          class="button text disable-totp"
          disabled={inputsDisabled}
          on:click={() => { disableTOTP().catch(catchError); }}
        >
          {$_("account.disable-totp")}
        </button>
      </div>
      {:else if totpKey}
        <img class=totp-qr-code src={totpKey.qrCode} alt=account.totp-qr-code>
        <div class=totp-secret-container>
          <p class=totp-secret-label>Secret</p>
          <div class=totp-secret-row>
            <p class=totp-secret>{totpKey.key}</p>
            <button class="button copy-totp-secret" on:click={copyTOTPSecret}>
              <span class=material-icons>content_copy</span>
            </button>
          </div>
        </div>
        <p class=totp-setup-instructions>
          {$_("account.totp-setup-instructions", { values: { codeLength: config.totp.codeLength } })}
        </p>
        <div class=totp-input>
          <IconTextInput
            bind:this={totpVerificationInput}
            bind:value={totpVerification}
            disabled={inputsDisabled}
            error={$gotErrors(...errorSources.totp)}
            icon=key
            inputmode=numeric
            on:input={onTOTPVerificationInput}
            on:keydown={onTOTPVerificationKeydown}
          />
        </div>
        <div class=totp-setup-buttons>
          <button
            class="button text submit-totp-setup"
            disabled={inputsDisabled}
            on:click={() => { enableTOTP().catch(catchError); }}
          >
            {$_("account.complete-totp-setup")}
          </button>
          <button
            class="button text cancel-totp-setup"
            disabled={inputsDisabled}
            on:click={cancelTOTPSetup}
          >
            {$_("account.cancel-totp-setup")}
          </button>
        </div>
      {:else}
        <button
          class="button text enable-totp"
          disabled={inputsDisabled || fetchingTOTPSecret}
          on:click={getTOTPKey}
        >
          {$_("account.enable-totp")}
        </button>
      {/if}
    </div>

    <button
      class="button text logout-all-sessions"
      disabled={inputsDisabled}
      on:click={logoutAllSessions}
    >
      {$_("account.logout-all-sessions")}
    </button>

    <button
      class="button text delete-account"
      disabled={inputsDisabled}
      on:click={() => { deleteAccount(); }}
    >
      {$_("account.delete-account")}
    </button>
  </main>
{/if}

<script context=module lang=ts>
  export { requireAuth as load } from "$lib/auth";
</script>

<script lang=ts>
  import { onMount, tick } from "svelte";
  import { fade } from "svelte/transition";
  import { _, locale } from "svelte-i18n";

  import { browser } from "$app/env";
  import { goto } from "$app/navigation";
  import { assets } from "$app/paths";
  import { session } from "$app/stores";

  import { cache, cacheURL } from "$lib/cache";
  import { config } from "$lib/config";
  import { errors, gotErrors, infoMessages, warnings } from "$lib/errors";
  import { sudo, time } from "$lib/time";
  import type { JSONObject } from "$lib/types";
  import { apiFetch, catchError, modals, userIcon, userIconURL } from "$lib/utils";
  import { validators } from "$lib/validation";

  import Checkbox from "$lib/components/Checkbox.svelte";
  import FileInput from "$lib/components/FileInput.svelte";
  import IconTextInput from "$lib/components/IconTextInput.svelte";
  import IconPasswordInput from "$lib/components/IconPasswordInput.svelte";

  import "@fontsource/fira-mono/latin.css";


  interface PutDataBody {
    currentPassword?: string;
    locale?: string;
    newPassword?: string;
    totpVerifyEnable?: string;
    username?: string;
  }

  interface TOTPKey {
    expires: Date;
    key: string;
    qrCode: string;
  }

  let heading: HTMLHeadingElement;
  let moveHeadingDown = false;
  let narrow = false;

  onMount(() => {
    const narrowBreakpoint = "33rem";
    const narrowMediaQuery = matchMedia(`(max-width: ${narrowBreakpoint})`);
    narrow = narrowMediaQuery.matches;
    narrowMediaQuery.addEventListener("change", (event): void => {
      narrow = event.matches;
    });

    const button = document.querySelector("#navbar-home-button")
      ?? document.querySelector("#navbar-menu-button");
    if (!heading || !button)
      return;
    const avoidNavbarBreakpoint =
      heading.offsetLeft + heading.offsetWidth + 2 * button.offsetWidth;
    const avoidNavbarMediaQuery = matchMedia(`(max-width: ${avoidNavbarBreakpoint}px)`);
    moveHeadingDown = avoidNavbarMediaQuery.matches;
    avoidNavbarMediaQuery.addEventListener("change", (event): void => {
      moveHeadingDown = event.matches;
    });
  });

  const errorSources = {
    currentPassword: [
      /^validation\.errors\.current-password\./u,
      "account.errors.invalid-current-password",
    ],
    icon: [/^validation\.errors\.user-icon\./u],
    locale: [/^validation\.errors\.locale\./u],
    newPassword: [/^validation\.errors\.new-password\./u],
    totp: [
      /^validation\.errors\.totp\./u,
      "account.errors.no-totp-key-active",
      "account.errors.invalid-totp-verification",
    ],
    username: [/^validation\.errors\.new-username\./u],
  };

  let submitting = false;
  let submittingUserIcon = false;
  let currentSubmitFunction: (password?: string) => Promise<boolean> | null = null;

  let username = "";
  let usernameInput: IconTextInput;
  let newPassword = "";
  let newPasswordInput: IconPasswordInput;
  let currentPassword = "";
  let currentPasswordInput: IconPasswordInput;
  let saveUsernamePasswordText = "account.save-changes";
  let selectedLocale = $session.user?.locale ?? config.defaultLocale;
  let textFieldsChanged = false;
  let usernameTimeout: ReturnType<typeof setTimeout> | null = null;
  let usernameUnavailable = false;

  let totpVerificationInput: IconTextInput;
  let fetchingTOTPSecret = false;
  let totpKey: TOTPKey | null = null;
  let totpVerification = "";

  let iconFiles: FileList;
  let iconFile: File | null = null;
  let iconLabel: HTMLLabelElement;
  let iconDragover = false;

  const onIconDragenter = (event: DragEvent): void => {
    if (
      !inputsDisabled
      && event.dataTransfer
      && event.dataTransfer.items.length === 1
      && event.dataTransfer.items[0].kind === "file"
      && config.acceptedImageTypes.includes(event.dataTransfer.items[0].type)
    )
      iconDragover = true;
  };

  const onIconDrop = (event: DragEvent): void => {
    iconDragover = false;
    if (
      !inputsDisabled
      && event.dataTransfer
      && event.dataTransfer.items.length === 1
      && event.dataTransfer.items[0].kind === "file"
      && config.acceptedImageTypes.includes(event.dataTransfer.items[0].type)
    )
      iconFile = event.dataTransfer.items[0].getAsFile();
  };

  const onIconFilesChange = async (): void => {
    if (!iconFiles || iconFiles.length === 0) {
      iconFile = null;
    } else if (iconFiles.length > 1) {
      errors.add("validation.errors.user-icon.multiple-files");
    } else {
      await tick();
      iconFile = iconFiles[0];
    }
  };

  const onIconFileChange = async (): Promise<void> => {
    if (!iconFile || submitting || !$session.user)
      return;

    errors.clear(...errorSources.icon);
    const iconErrors = await validators.userIcon(iconFile);
    if (iconErrors.length > 0) {
      iconFile = null;
      iconFiles = null;
      errors.add(...iconErrors);
      return;
    }

    submitting = true;
    submittingUserIcon = true;
    const form = new FormData();
    form.append("icon", iconFile, "");
    let data: JSONObject;
    try {
      data = await apiFetch(`/users/${$session.user.id}/icon`, "PUT", form);
    } catch (error) {
      catchError(error);
      submitting = false;
      submittingUserIcon = false;
      return;
    }
    if (data.errors) {
      submitting = false;
      submittingUserIcon = false;
      return;
    }
    $session.user.icon = userIconURL(data.icon);
    document.querySelector("#navbar-home-button")?.focus();
    infoMessages.showTimed("account.icon-updated");
    submitting = false;
    submittingUserIcon = false;
  };

  const removeIcon = async (): Promise<void> => {
    if (!$session.user || submitting)
      return;

    submitting = true;
    let data: JSONObject;
    try {
      data = await apiFetch(`/users/${$session.user.id}/icon`, "PUT", { remove: true });
    } catch(error) {
      catchError(error);
      submitting = false;
      return;
    }
    if (data.errors) {
      submitting = false;
      return;
    }
    $session.user.icon = null;
    infoMessages.showTimed("account.icon-removed");
    submitting = false;
  };

  const updateUsernamePassword = async (password?: string): Promise<boolean> => {
    if (submitting || !$session.user)
      return true;

    username = username.trim();
    const newErrors = [];
    if (username)
      newErrors.push(...validators.username(username, "new-username", true));
    if (newPassword)
      newErrors.push(...validators.password(newPassword, "new-password", true));

    if (newErrors.length > 0) {
      errors.add(...newErrors);
      if ($gotErrors(...errorSources.username))
        usernameInput?.focus();
      else if ($gotErrors(...errorSources.newPassword))
        newPasswordInput?.focus();
      return true;
    }

    const body = {};
    if (username && username !== $session.user.username)
      body.username = username;
    if (newPassword)
      body.newPassword = newPassword;
    if (password) {
      body.currentPassword = password;
    } else if (!$sudo) {
      errors.clear();
      warnings.clear();
      infoMessages.clear();
      currentSubmitFunction = updateUsernamePassword;
      return true;
    }

    submitting = true;
    let data: JSONObject;
    try {
      data = await apiFetch(`/users/${$session.user.id}`, "PUT", body);
    } catch (error) {
      catchError(error);
      submitting = false;
      return true;
    }

    if (data.sudoUntil)
      $session.user.sudoUntil = data.sudoUntil;

    if (data.errors) {
      submitting = false;
      if (!currentSubmitFunction && $gotErrors("validation.errors.current-password.empty")) {
        errors.clear("validation.errors.current-password.empty");
        warnings.clear();
        currentSubmitFunction = updateUsernamePassword;
        return false;
      }
      if ($gotErrors(...errorSources.username))
        usernameInput?.focus();
      else if ($gotErrors(...errorSources.newPassword))
        newPasswordInput?.focus();
      return !$gotErrors(...errorSources.currentPassword);
    }

    if (data.username) {
      $session.user.username = data.username;
      username = "";
      infoMessages.showTimed("account.username-changed");
    }

    if (data.passwordUpdated) {
      newPassword = "";
      infoMessages.showTimed("account.password-changed");
    }

    if (data.passwordChangeReason !== undefined)
      $session.user.passwordChangeReason = data.passwordChangeReason;

    submitting = false;
    return true;
  };

  const cancelTOTPSetup = (): void => {
    errors.clear(...errorSources.totp);
    totpKey = null;
    totpVerification = "";
  };

  const enableTOTP = async (password?: string): Promise<boolean> => {
    if (submitting || !$session.user)
      return true;

    const totpErrors = validators.totp(totpVerification, false);
    if (totpErrors.length > 0) {
      errors.add(...totpErrors);
      submitting = false;
      totpVerificationInput?.focus();
      return true;
    }

    const body = { totp: totpVerification };
    if (password) {
      body.currentPassword = password;
    } else if (!$sudo) {
      errors.clear();
      warnings.clear();
      infoMessages.clear();
      currentSubmitFunction = enableTOTP;
      return true;
    }

    submitting = true;
    let data: JSONObject;
    try {
      data = await apiFetch(`/users/${$session.user.id}`, "PUT", body);
    } catch (error) {
      catchError(error);
      submitting = false;
      await tick();
      totpVerificationInput?.focus();
      return true;
    }

    if (data.sudoUntil)
      $session.user.sudoUntil = data.sudoUntil;

    if (data.errors) {
      submitting = false;
      if (!currentSubmitFunction && $gotErrors("validation.errors.current-password.empty")) {
        errors.clear("validation.errors.current-password.empty");
        warnings.clear();
        currentSubmitFunction = enableTOTP;
        return false;
      }
      await tick();
      totpVerificationInput?.focus();
      return !$gotErrors(...errorSources.currentPassword);
    }

    $session.user.totpEnabled = true;
    cancelTOTPSetup();
    infoMessages.showTimed("account.totp-enabled");
    submitting = false;
    return true;
  };

  const onTOTPVerificationInput = (): void => {
    errors.clear(...errorSources.totp);
    const chars = [...totpVerification];
    if (chars.length > config.totp.codeLength)
      totpVerification = chars.slice(0, config.totp.codeLength).join("");
  };

  const onTOTPVerificationKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Enter")
      enableTOTP().catch(catchError);
  };

  const getTOTPKey = async (): Promise<void> => {
    fetchingTOTPSecret = true;
    let data: JSONObject;
    try {
      data = await apiFetch("/account/totp-key");
    } catch {
      fetchingTOTPSecret = false;
      return;
    }
    if (data.errors) {
      fetchingTOTPSecret = false;
      return;
    }
    totpKey = {
      expires: new Date(data.expires),
      key: data.key,
      qrCode: data.qrCode,
    };
    fetchingTOTPSecret = false;
  };

  const copyTOTPSecret = async (): Promise<void> => {
    if (!totpKey?.key || !navigator.clipboard) {
      warnings.add("account.warnings.totp-secret-copy-failed");
      return;
    }
    try {
      await navigator.clipboard.writeText(totpKey.key);
    } catch (error) {
      warnings.add("account.warnings.totp-secret-copy-failed");
      return;
    }
    infoMessages.showTimed("account.totp-secret-copied");
  };

  const disableTOTP = async (password?: string): Promise<boolean> => {
    if (!$session.user?.totpEnabled || submitting)
      return true;

    const body = { totp: null };
    if (password) {
      body.currentPassword = password;
    } else if (!$sudo) {
      errors.clear();
      warnings.clear();
      infoMessages.clear();
      currentSubmitFunction = disableTOTP;
      return true;
    }

    submitting = true;
    let data: JSONObject;
    try {
      data = await apiFetch(`/users/${$session.user.id}`, "PUT", body);
    } catch (error) {
      catchError(error);
      submitting = false;
      return true;
    }

    if (data.sudoUntil)
      $session.user.sudoUntil = data.sudoUntil;

    if (data.errors) {
      submitting = false;
      if (!currentSubmitFunction && $gotErrors("validation.errors.current-password.empty")) {
        errors.clear("validation.errors.current-password.empty");
        warnings.clear();
        currentSubmitFunction = disableTOTP;
        return false;
      }
      return !$gotErrors(...errorSources.currentPassword);
    }

    $session.user.totpEnabled = false;
    infoMessages.showTimed("account.totp-disabled");
    submitting = false;
    return true;
  };

  const updateLocale = async (id: string): Promise<void> => {
    if (!$session.user || submitting || id === $session.user.locale)
      return;

    const localeErrors = validators.locale(id);
    if (localeErrors.length > 0) {
      errors.add(...localeErrors);
      return;
    }

    submitting = true;
    let data: JSONObject;
    try {
      data = await apiFetch(`/users/${$session.user.id}`, "PUT", { locale: id });
    } catch (error) {
      catchError(error);
      submitting = false;
      return;
    }

    if (data.errors || data.locale !== id) {
      submitting = false;
      return;
    }

    errors.clear(...errorSources.locale);
    submitting = false;
    $locale = id;
    $session.user.locale = id;
    selectedLocale = id;
    infoMessages.showTimed("account.locale-updated");
  };

  const logoutAllSessions = async (): Promise<void> => {
    if (submitting)
      return;

    submitting = true;
    let data: JSONObject;
    try {
      data = await apiFetch("/logout/all-sessions", "POST");
    } catch (error) {
      catchError(error);
      submitting = false;
      return;
    }

    if (data.errors) {
      submitting = false;
      return;
    }

    errors.clear();
    warnings.clear();
    infoMessages.clear();
    localStorage.setItem(config.csrfTokenField, data.csrfToken);

    goto("/").catch(catchError);
    $session.user = null;
    submitting = false;
  };

  const deleteAccount = async (password?: string): Promise<boolean> => {
    if (!$session.user || submitting)
      return true;

    if (!password) {
      errors.clear();
      warnings.clear();
      infoMessages.clear();
      currentSubmitFunction = deleteAccount;
      return true;
    }

    submitting = true;
    let data: JSONObject;
    try {
      data = await apiFetch(`/users/${$session.user.id}`, "DELETE", { password });
    } catch (error) {
      catchError(error);
      submitting = false;
      return true;
    }

    if (data.errors) {
      submitting = false;
      return !$gotErrors(...errorSources.currentPassword);
    }

    errors.clear();
    infoMessages.clear();
    localStorage.setItem(config.csrfTokenField, data.csrfToken);

    goto("/").catch(catchError);
    $session.user = null;
    infoMessages.showTimed("account.account-deleted");
    submitting = false;
    return true;
  };

  const onUsernameInput = (): void => {
    errors.clear(...errorSources.username);
    const chars = [...username];
    if (chars.length > config.validationRules.username.maxLength)
      username = chars.slice(0, config.validationRules.username.maxLength).join("");
    if (usernameTimeout)
      clearTimeout(usernameTimeout);
    usernameTimeout = setTimeout(async () => {
      if (
        username.trim() && username.trim().toLowerCase() !== $session.user.username.toLowerCase()
      ) {
        let data: JSONObject;
        try {
          data = await apiFetch(`/users/username-available/${encodeURIComponent(username.trim())}`);
        } catch (error) {
          catchError(error);
          usernameUnavailable = false;
          return;
        }
        usernameUnavailable = data.available === false;
      } else {
        usernameUnavailable = false;
      }
    }, config.debounceTime);
  };

  const onNewPasswordInput = (): void => {
    errors.clear(...errorSources.newPassword);
    const chars = [...newPassword];
    if (chars.length > config.validationRules.password.maxLength)
      newPassword = chars.slice(0, config.validationRules.password.maxLength).join("");
  };

  const submitWithCurrentPassword = async (): Promise<void> => {
    if (!currentSubmitFunction) {
      errors.add("general.errors.unexpected");
      return;
    }
    const passwordErrors = validators.password(currentPassword, "current-password", false);
    if (passwordErrors.length > 0) {
      errors.add(...passwordErrors);
      currentPasswordInput?.focus();
    } else if (await currentSubmitFunction(currentPassword)) {
      currentSubmitFunction = null;
      currentPassword = "";
    } else {
      currentPasswordInput?.focus();
    }
  };

  const onCurrentPasswordInput = (): void => {
    errors.clear(...errorSources.currentPassword);
    const chars = [...currentPassword];
    if (chars.length > config.validationRules.password.maxLength)
      currentPassword = chars.slice(0, config.validationRules.password.maxLength).join("");
  };

  const onCurrentPasswordKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Enter")
      submitWithCurrentPassword().catch(catchError);
  };

  /* eslint-disable @typescript-eslint/indent */
  $: inputsDisabled = submitting;
  $: iconFile, onIconFileChange().catch(catchError);
  $: iconFiles, onIconFilesChange().catch(catchError);
  $: textFieldsChanged =
       (username.trim() && username.trim() !== $session.user?.username) || newPassword;
  $: if (username.trim() && username.trim() !== $session.user?.username && newPassword)
       saveUsernamePasswordText = "account.change-username-and-password";
     else if (username.trim() && username.trim() !== $session.user?.username)
       saveUsernamePasswordText = "account.change-username";
     else if (newPassword)
       saveUsernamePasswordText = "account.change-password";
     else
       saveUsernamePasswordText = "account.save-changes";
  $: if (currentSubmitFunction) {
       modals.push("account-current-password");
       currentPasswordInput?.focus();
     } else {
       modals.remove("account-current-password");
     }
  $: totpKey && totpVerificationInput?.focus();
  $: if (totpKey && totpKey.expires < $time) {
       infoMessages.showTimed("account.totp-setup-expired");
       errors.clear(...errorSources.totp);
       totpKey = null;
       totpVerification = "";
     }
  /* eslint-enable @typescript-eslint/indent */
</script>

<style lang=scss>
  @use "sass:color";
  @use "globals.scss" as g;

  $narrow-breakpoint: 33rem;
  $body-padding: 1rem;
  $user-icon-size: 10rem;

  main {
    max-width: 40rem;
    margin: auto;
  }

  .current-password {
    position: fixed;
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 3;
  }

  .current-password-background {
    position: absolute;
    width: 100%;
    height: 100%;
    background-color: color.change(g.$black, $alpha: 0.9);
  }

  .current-password-modal {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 1rem;
    z-index: 4;
    background-color: g.$black;
    outline: g.$white solid 0.25rem;
  }

  .current-password-submit {
    background-color: g.$green;
  }

  .delete-account-button {
    background-color: g.$red;
  }

  .main-heading {
    height: g.$icon-button-size;
    display: inline-flex;
    align-items: end;
    font-size: 2rem;
    margin: 0 $body-padding;
  }

  .subheading {
    margin: 0.25rem 0;
    font-size: 1.25rem;
    font-weight: 500;
  }

  .icon-heading {
    display: flex;
    gap: 0.5rem;
  }

  .avoid-navbar-intersection {
    margin-top: g.$icon-button-size;
  }

  .basic-info {
    display: flex;
    flex-direction: column;
    align-items: center;

    @media(min-width: $narrow-breakpoint) {
      flex-direction: row;
      align-items: end;
    }
  }

  .basic-info-text-fields {
    width: calc(100% - 2 * $body-padding);
    margin: 1rem 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 1.5rem;

    @media(min-width: $narrow-breakpoint) {
      width: 20rem;
    }
  }

   .save-username-password {
     background-color: g.$green;
   }

   .username {
      font-weight: 700;
   }

  .user-icon {
    width: $user-icon-size;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    flex-shrink: 0;
    gap: 1rem;
    margin: 1rem 1rem 0 1rem;
  }

  .user-icon-container {
    height: $user-icon-size;
    width: $user-icon-size;
    clip-path: circle(closest-side);
    cursor: pointer;

    * {
      pointer-events: none;
    }

    .hover {
      display: none;
    }

    &:hover .hover {
      display: flex;
    }
  }

  .user-icon-image {
    position: absolute;
    width: $user-icon-size;
    height: $user-icon-size;
    image-rendering: high-quality;
  }

  .user-icon-overlay {
    position: absolute;
    width: $user-icon-size;
    height: $user-icon-size;
    display: flex;
    justify-content: center;
    align-items: center;
    background: rgba(255, 255, 255, 0.6);
    color: g.$black;
    font-weight: bold;

    .material-icons {
      font-size: 2rem;
    }
  }

  .mfa {
    margin: 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;

    @media(min-width: $narrow-breakpoint) {
      align-items: start;
    }
  }

  .mfa-open {
    padding: 1rem;
    background-color: g.$dark-grey;
  }

  .totp-qr-code {
    max-width: 100%;
    align-self: center;
    margin: 0 1rem;
    padding: 0.5rem;
    background-color: white;
  }

  .enable-totp {
    background-color: g.$green;
  }

  .disable-totp {
    background-color: g.$red;
  }

  $totp-secret-height: 4rem;
  $totp-secret-padding: 0.2rem;

  .totp-secret-container {
    display: flex;
    flex-direction: column;
    margin: 1rem 0;
    width: 100%;
  }

  .totp-secret-row {
    min-height: $totp-secret-height;
    display: flex;
  }

  .totp-secret {
    display: flex;
    justify-content: center;
    align-items: center;
    width: calc(100% - $totp-secret-height - $totp-secret-padding);
    height: $totp-secret-height;
    margin: 0;
    padding-left: $totp-secret-padding;
    background-color: #FFFFFF;
    color: g.$black;
    font-family: "Fira Mono";
    font-size: 0.8rem;  // should show on wide displays without linebreaks
    word-break: break-all;
  }

  .copy-totp-secret {
    width: $totp-secret-height;
    height: $totp-secret-height;
    background-color: g.$cyan;
  }

  .totp-secret-label {
    margin: 0;
    font-size: 0.9rem;
  }

  .totp-setup-instructions {
    margin-top: 0;
  }

  .totp-input {
    max-width: 20rem;
    align-self: center;
  }

  .totp-setup-buttons {
    width: 100%;
    display: flex;
    gap: 1rem;
    margin-top: 1rem;
  }

  $totp-setup-button-width: calc((100% - 1rem) / 2);

  .submit-totp-setup {
    width: $totp-setup-button-width;
    background-color: g.$green;
  }

  .cancel-totp-setup {
    width: $totp-setup-button-width;
    background-color: g.$red;
  }

  .locale {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .locale-buttons {
    display: flex;
    justify-content: center;
    align-items: start;
    gap: 0.5rem;
    margin: 0 1rem;

    @media(min-width: $narrow-breakpoint) {
      justify-content: start;
    }
  }

  .locale-button {
    background-color: g.$magenta;
  }

  .logout-all-sessions, .delete-account {
    margin: 1rem auto;
    background-color: g.$red;

    @media(min-width: $narrow-breakpoint) {
      margin: 1rem;
    }
  }
</style>
