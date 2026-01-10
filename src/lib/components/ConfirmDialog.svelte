<script lang="ts">
    import Modal from "./Modal.svelte";
    import Icon from "./Icon.svelte";
    import { createEventDispatcher } from "svelte";

    export let show = false;
    export let title = "Confirm Action";
    export let message = "Are you sure you want to proceed?";
    export let confirmText = "Confirm";
    export let cancelText = "Cancel";
    export let type: "danger" | "warning" | "info" = "danger";
    export let loading = false;

    const dispatch = createEventDispatcher();

    function onConfirm() {
        dispatch("confirm");
    }

    function onCancel() {
        show = false;
        dispatch("cancel");
    }
</script>

<Modal {show} {title} width="400px" on:close={onCancel}>
    <div class="confirm-content">
        <div class="icon-wrapper {type}">
            <Icon
                name={type === "danger"
                    ? "alert-circle"
                    : type === "warning"
                      ? "alert-triangle"
                      : "info"}
                size={32}
            />
        </div>
        <p class="message">{message}</p>
    </div>

    <div slot="footer" class="actions">
        <button class="btn btn-ghost" on:click={onCancel} disabled={loading}>
            {cancelText}
        </button>
        <button class="btn btn-{type}" on:click={onConfirm} disabled={loading}>
            {#if loading}
                <span class="spinner"></span>
            {/if}
            {confirmText}
        </button>
    </div>
</Modal>

<style>
    .confirm-content {
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
        padding: 0.5rem 0;
    }

    .icon-wrapper {
        width: 64px;
        height: 64px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-bottom: 1rem;
    }

    .icon-wrapper.danger {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
    }

    .icon-wrapper.warning {
        background: rgba(245, 158, 11, 0.1);
        color: #f59e0b;
    }

    .icon-wrapper.info {
        background: rgba(59, 130, 246, 0.1);
        color: #3b82f6;
    }

    .message {
        color: var(--text-secondary, #94a3b8);
        line-height: 1.5;
        margin: 0;
    }

    .actions {
        display: flex;
        gap: 0.75rem;
        width: 100%;
        justify-content: center;
    }

    .btn {
        padding: 0.6rem 1.25rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        border: none;
        transition: all 0.2s;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .btn-ghost {
        background: transparent;
        color: var(--text-secondary, #94a3b8);
        border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .btn-ghost:hover {
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-primary, #fff);
    }

    .btn-danger {
        background: #ef4444;
        color: white;
    }
    .btn-danger:hover {
        background: #dc2626;
    }

    .btn-warning {
        background: #f59e0b;
        color: white;
    }
    .btn-warning:hover {
        background: #d97706;
    }

    .btn-info {
        background: #3b82f6;
        color: white;
    }
    .btn-info:hover {
        background: #2563eb;
    }

    .btn:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    .spinner {
        width: 16px;
        height: 16px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }
</style>
