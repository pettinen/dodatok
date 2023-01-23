<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { tl } from "$i18n";

    import { noop } from "$lib/utils";

    export let accept: string[] = [];
    export let disabled = false;
    export let hidden = false;
    export let files: FileList | null = null;

    const dispatch = createEventDispatcher();
    let dragover = false;
    let inputFiles: FileList;
    let text = "";

    $: attrs = { accept: accept.join(","), disabled };
    $: files = inputFiles;
    $: if (
        !files ||
        files.length === 0 ||
        (files.length === 1 && !files[0].name)
    )
        text = $tl("file_upload.select_or_drop_file");
    else if (files.length === 1) text = files[0].name;
    else text = `${files.length} files selected`;

    const clear = (): void => {
        files = null;
        dispatch("clear");
    };

    const showDragover = (): void => {
        dragover = true;
    };

    const hideDragover = (): void => {
        dragover = false;
    };

    const onDrop = (event: DragEvent): void => {
        dragover = false;
        if (
            !disabled &&
            event.dataTransfer &&
            event.dataTransfer.files.length > 0
        )
            ({ files } = event.dataTransfer);
    };
</script>

<div class="upload" class:dragover class:hidden>
    <label
        class="button text"
        class:disabled
        on:dragenter={showDragover}
        on:dragleave={hideDragover}
        on:dragover|preventDefault={noop}
        on:drop|preventDefault={onDrop}
    >
        <input class="hidden" bind:files={inputFiles} {...attrs} type="file" />
        <span class="material-icons">file_upload</span>
        {text}
    </label>
    {#if files && files.length > 0}
        <button
            class="button icon clear"
            title={$tl("file_upload.clear")}
            type="button"
            on:click={clear}
        >
            <span class="material-icons">clear</span>
        </button>
    {/if}
</div>

<style lang="scss">
    @use "$lib/styles/globals.scss" as g;

    $dragover-outline-width: 0.15rem;

    label {
        display: flex;
        gap: 0.5rem;
        cursor: pointer;
    }

    .upload {
        width: max-content;
        display: flex;
        background-color: g.$magenta;
    }

    .clear {
        background-color: g.$red;
    }

    .dragover {
        outline: $dragover-outline-width dashed g.$white;
    }
</style>
