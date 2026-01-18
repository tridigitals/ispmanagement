<script lang="ts">
    import { fade, fly } from "svelte/transition";
    import Icon from "./Icon.svelte";

    let { 
        title = "", 
        width = "420px", 
        show = false, 
        onclose,
        children,
        footer
    } = $props<{
        title?: string;
        width?: string;
        show: boolean;
        onclose?: () => void;
        children?: import('svelte').Snippet;
        footer?: import('svelte').Snippet;
    }>();

    function close() {
        if (onclose) onclose();
    }
</script>

{#if show}
    <div
        class="modal-backdrop"
        onclick={close}
        onkeydown={(e) => e.key === "Escape" && close()}
        role="button"
        tabindex="0"
        transition:fade={{ duration: 200 }}
    >
        <div
            class="modal-card"
            style="max-width: {width}"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            aria-label={title}
            transition:fly={{ y: 20, duration: 300 }}
        >
            <div class="modal-header">
                <h3>{title}</h3>
                <button class="close-btn" onclick={close}>
                    <Icon name="x" size={20} />
                </button>
            </div>
            <div class="modal-body">
                {#if children}
                    {@render children()}
                {/if}
            </div>
            {#if footer}
                <div class="modal-footer">
                    {@render footer()}
                </div>
            {/if}
        </div>
    </div>
{/if}

<style>
    .modal-backdrop {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.7);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
        padding: 1rem;
    }

    .modal-card {
        background: var(--bg-surface, #1e293b);
        width: 100%;
        border-radius: 16px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
        display: flex;
        flex-direction: column;
        max-height: 90vh;
    }

    .modal-header {
        padding: 1.5rem 1.5rem 1rem 1.5rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }

    .modal-header h3 {
        margin: 0;
        font-size: 1.25rem;
        color: var(--text-primary, white);
        font-weight: 600;
    }

    .close-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary, #94a3b8);
        cursor: pointer;
        padding: 0.5rem;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 8px;
        transition: all 0.2s;
    }

    .close-btn:hover {
        background: rgba(255, 255, 255, 0.1);
        color: var(--text-primary, #fff);
    }

    .modal-body {
        padding: 1.5rem;
        overflow-y: auto;
    }

    .modal-footer {
        padding: 1rem 1.5rem 1.5rem 1.5rem;
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
        border-top: 1px solid var(--border-color, rgba(255, 255, 255, 0.05));
        background: transparent; /* Explicitly transparent */
        border-bottom-left-radius: 16px;
        border-bottom-right-radius: 16px;
    }

    @media (max-width: 640px) {
        .modal-card {
            max-width: 100% !important;
            margin: 1rem;
        }

        .modal-backdrop {
            align-items: center;
            padding: 1rem;
        }

        .modal-body {
            max-height: 70vh;
        }
    }
</style>
