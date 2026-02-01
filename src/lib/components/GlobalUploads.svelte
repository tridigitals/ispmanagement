<script lang="ts">
    import { uploadStore } from "$lib/stores/upload";
    import Icon from "$lib/components/Icon.svelte";
    import { fly, slide } from "svelte/transition";
    import { t } from "svelte-i18n";

    let uploads = $derived($uploadStore);
    let activeCount = $derived(uploads.length);
    let isExpanded = $state(true);

    function toggle() {
        isExpanded = !isExpanded;
    }
</script>

{#if activeCount > 0}
    <div class="upload-container" transition:fly={{ y: 50, duration: 300 }}>
        <!-- Header -->
        <div
            class="header"
            onclick={toggle}
            role="button"
            tabindex="0"
            onkeydown={(e) => (e.key === "Enter" || e.key === " ") && toggle()}
        >
            <div class="title">
                <span class="count">{activeCount}</span>
                <span>
                    {$t("components.global_uploads.title") || "Uploads in progress"}
                </span>
            </div>
            <button class="toggle-btn" type="button">
                <Icon
                    name={isExpanded ? "chevron-down" : "chevron-up"}
                    size={18}
                />
            </button>
        </div>

        <!-- List -->
        {#if isExpanded}
            <div class="list" transition:slide>
                {#each uploads as item (item.id)}
                    <div class="item">
                        <div class="item-icon">
                            <Icon name="file" size={20} />
                        </div>
                        <div class="item-content">
                            <div class="item-top">
                                <span class="name">{item.file.name}</span>
                                {#if item.status === "error"}
                                    <span class="status error">
                                        {$t("components.global_uploads.status.failed") ||
                                            "Failed"}
                                    </span>
                                {:else if item.status === "success"}
                                    <span class="status success">
                                        {$t("components.global_uploads.status.done") ||
                                            "Done"}
                                    </span>
                                {:else}
                                    <span class="status">{item.progress}%</span>
                                {/if}
                            </div>

                            <div class="progress-track">
                                <div
                                    class="progress-bar {item.status}"
                                    style="width: {item.progress}%"
                                ></div>
                            </div>
                        </div>

                        {#if item.status === "uploading"}
                            <button
                                class="cancel-btn"
                                type="button"
                                title={$t("components.global_uploads.actions.cancel") ||
                                    "Cancel upload"}
                                aria-label={$t("components.global_uploads.actions.cancel") ||
                                    "Cancel upload"}
                                onclick={() => uploadStore.cancel(item.id)}
                            >
                                <Icon name="x" size={14} />
                            </button>
                        {/if}
                    </div>
                {/each}
            </div>
        {/if}
    </div>
{/if}

<style>
    .upload-container {
        position: fixed;
        bottom: 24px;
        right: 24px;
        width: 360px;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
        z-index: 10000;
        overflow: hidden;
        font-family: var(--font-family);
    }

    .header {
        background: var(--bg-app);
        padding: 12px 16px;
        display: flex;
        justify-content: space-between;
        align-items: center;
        cursor: pointer;
        border-bottom: 1px solid var(--border-color);
    }

    .title {
        display: flex;
        align-items: center;
        gap: 10px;
        font-weight: 600;
        font-size: 0.9rem;
        color: var(--text-primary);
    }

    .count {
        background: var(--color-primary);
        color: white;
        padding: 2px 8px;
        border-radius: 10px;
        font-size: 0.75rem;
    }

    .toggle-btn {
        background: none;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
    }

    .list {
        max-height: 300px;
        overflow-y: auto;
        background: var(--bg-surface);
    }

    .item {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 12px 16px;
        border-bottom: 1px solid var(--border-subtle);
    }

    .item:last-child {
        border-bottom: none;
    }

    .item-icon {
        color: var(--text-secondary);
        flex-shrink: 0;
    }

    .item-content {
        flex: 1;
        min-width: 0;
    }

    .item-top {
        display: flex;
        justify-content: space-between;
        margin-bottom: 6px;
        font-size: 0.85rem;
    }

    .name {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        color: var(--text-primary);
        font-weight: 500;
    }

    .status {
        font-size: 0.75rem;
        color: var(--text-secondary);
        font-variant-numeric: tabular-nums;
    }

    .status.success {
        color: var(--color-success);
    }
    .status.error {
        color: var(--color-danger);
    }

    .progress-track {
        height: 4px;
        background: var(--bg-app);
        border-radius: 2px;
        overflow: hidden;
    }

    .progress-bar {
        height: 100%;
        background: var(--color-primary);
        transition: width 0.2s ease;
    }

    .progress-bar.success {
        background: var(--color-success);
    }
    .progress-bar.error {
        background: var(--color-danger);
    }

    .cancel-btn {
        background: none;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 4px;
    }

    .cancel-btn:hover {
        color: var(--color-danger);
    }
</style>
