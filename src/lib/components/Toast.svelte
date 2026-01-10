<script lang="ts">
    import { toast } from "$lib/stores/toast";
    import Icon from "./Icon.svelte";
    import { fly, fade } from "svelte/transition";
    import { flip } from "svelte/animate";

    function getIcon(type: string) {
        switch (type) {
            case "success":
                return "check-circle";
            case "error":
                return "alert-circle";
            case "warning":
                return "alert-triangle";
            default:
                return "info";
        }
    }

    function getColor(type: string) {
        switch (type) {
            case "success":
                return "var(--color-success, #10b981)";
            case "error":
                return "var(--color-danger, #ef4444)";
            case "warning":
                return "var(--color-warning, #f59e0b)";
            default:
                return "var(--color-info, #3b82f6)";
        }
    }
</script>

<div class="toast-container">
    {#each $toast as t (t.id)}
        <div
            class="toast"
            class:success={t.type === "success"}
            class:error={t.type === "error"}
            class:warning={t.type === "warning"}
            class:info={t.type === "info"}
            transition:fly={{ y: 20, duration: 300 }}
            animate:flip
        >
            <div class="icon" style="color: {getColor(t.type)}">
                <Icon name={getIcon(t.type)} size={20} />
            </div>
            <div class="message">{t.message}</div>
            <button class="close-btn" on:click={() => toast.remove(t.id)}>
                <Icon name="x" size={16} />
            </button>
        </div>
    {/each}
</div>

<style>
    .toast-container {
        position: fixed;
        bottom: 2rem;
        right: 2rem;
        z-index: 9999;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        pointer-events: none; /* Let clicks pass through container */
    }

    .toast {
        min-width: 300px;
        max-width: 400px;
        background: var(--bg-surface, #1e293b);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 12px;
        padding: 1rem;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
        pointer-events: auto; /* Re-enable clicks on toast */
        border-left: 4px solid transparent;
    }

    .toast.success {
        border-left-color: var(--color-success, #10b981);
    }
    .toast.error {
        border-left-color: var(--color-danger, #ef4444);
    }
    .toast.warning {
        border-left-color: var(--color-warning, #f59e0b);
    }
    .toast.info {
        border-left-color: var(--color-info, #3b82f6);
    }

    .message {
        flex: 1;
        font-size: 0.95rem;
        color: var(--text-primary, #fff);
        line-height: 1.4;
    }

    .close-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary, #94a3b8);
        cursor: pointer;
        padding: 0.25rem;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 4px;
        transition: all 0.2s;
    }

    .close-btn:hover {
        background: rgba(255, 255, 255, 0.1);
        color: var(--text-primary, #fff);
    }

    @media (max-width: 640px) {
        .toast-container {
            right: 0;
            left: 0;
            bottom: 0;
            padding: 1rem;
            align-items: stretch; /* Full width */
        }

        .toast {
            width: 100%;
            max-width: 100%;
            min-width: 0;
            border-radius: 8px;
        }
    }
</style>
