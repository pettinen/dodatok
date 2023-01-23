<script lang="ts">
    import { is_language, tl } from "$i18n";
    import { errors, warnings } from "$lib/alerts";
    import { info_messages } from "$lib/messages";

    import Message from "$lib/components/Message.svelte";
    import Navbar from "$lib/components/Navbar.svelte";

    import "@fontsource/fira-sans/latin.css";
    import "material-icons/iconfont/material-icons.css";
    import "$lib/styles/main.scss";

    // $: console.log("+layout.svelte $page.data:", $page.data);
    // $: ({ language } = $page.data);
    // $: language && set_language(language).catch(console.error); // TODO: error handling

    const isLoading = false;

    const clearMessages = (): void => {
        errors.clear();
        warnings.clear();
        info_messages.clear();
    };

    $: got_errors = $errors.length > 0;
    $: got_warnings = $warnings.length > 0;
    $: got_info_messages = $info_messages.length > 0;
</script>

{#if !isLoading}
    <Navbar />
    {is_language("it")}
    {is_language("fi")}
    <slot />

    {#if got_errors || got_warnings || got_info_messages}
        <section class="messages" role="alert">
            <div class="message-lists">
                {#if got_info_messages}
                    <div class="message-list-container info-messages">
                        <span class="material-icons">info</span>
                        <ul class="message-list">
                            {#each $info_messages as message}
                                <Message {message} />
                            {/each}
                        </ul>
                    </div>
                {/if}
                {#if got_errors}
                    <div class="message-list-container error-messages">
                        <span class="material-icons">report</span>
                        <ul class="message-list">
                            {#each $errors as error}
                                {@const message = $tl(...error.tl)}
                                {#if message}
                                    <li>{message}</li>
                                {/if}
                            {/each}
                        </ul>
                    </div>
                {/if}
                {#if got_warnings}
                    <div class="message-list-container warning-messages">
                        <span class="material-icons">report_problem</span>
                        <ul class="message-list">
                            {#each $warnings as warning}
                                {@const message = $tl(...warning.tl)}
                                {#if message}
                                    <li>{message}</li>
                                {/if}
                            {/each}
                        </ul>
                    </div>
                {/if}
            </div>
            <div class="close-message-container">
                <button
                    class="button icon close-message-button material-icons"
                    type="button"
                    on:click={clearMessages}
                >
                    close
                </button>
            </div>
        </section>
    {/if}
{/if}

<style lang="scss">
    @use "$lib/styles/globals.scss" as g;

    .messages {
        position: fixed;
        right: 0;
        bottom: 0;
        width: 100%;
        max-width: 49rem;
        display: flex;
        justify-content: space-between;
        background-color: g.$dark-grey;
        color: g.$text-color-dark;
        z-index: 3;
    }

    .message-lists {
        width: calc(100% - g.$icon-button-size);
        display: flex;
        flex-direction: column;
    }

    .message-list-container {
        flex: 1;
        display: flex;
        align-items: center;
        padding: 0 0.25rem;

        .material-icons {
            height: g.$icon-button-size;
            align-self: start;
            display: flex;
            align-items: center;
        }
    }

    .message-list {
        list-style-type: none;
        margin: 0;
        padding: 0.5rem;

        li {
            line-height: 1.25em;
        }
    }

    .close-message-container {
        display: flex;
        flex-direction: column;
        width: g.$icon-button-size;
    }

    .close-message-button {
        background-color: g.$dark-grey;
        color: g.$white;
    }

    .info-messages {
        background-color: g.$dark-grey;
        color: g.$white;
    }

    .error-messages {
        background-color: g.$red;
    }

    .warning-messages {
        background-color: g.$yellow;
    }
</style>
