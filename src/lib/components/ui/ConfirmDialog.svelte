<script lang="ts">
    import Modal from "./Modal.svelte";
    import Icon from "./Icon.svelte";
    import { t } from "svelte-i18n";

    let { 
        show = $bindable(false), 
        title = "Confirm Action", 
        message = "Are you sure you want to proceed?", 
        confirmText = "Confirm", 
        cancelText = "Cancel", 
        type = "danger", 
        confirmationKeyword = "", 
        loading = false,
        onconfirm,
        oncancel
    } = $props<{
        show?: boolean;
        title?: string;
        message?: string;
        confirmText?: string;
        cancelText?: string;
        type?: "danger" | "warning" | "info";
        confirmationKeyword?: string;
        loading?: boolean;
        onconfirm?: () => void;
        oncancel?: () => void;
    }>();

    let inputValue = $state("");

    let canConfirm = $derived(
        !loading &&
        (!confirmationKeyword || inputValue === confirmationKeyword)
    );

    function handleConfirm() {
        if (!canConfirm) return;
        if (onconfirm) onconfirm();
        inputValue = ""; // Reset after confirm
    }

    function handleCancel() {
        show = false;
        if (oncancel) oncancel();
        inputValue = ""; // Reset after cancel
    }
</script>

<Modal {show} {title} width="400px" onclose={handleCancel}>
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

        {#if confirmationKeyword}
            <div class="confirmation-input">
                <p class="instruction">
                    {$t("components.confirm_dialog.instruction_prefix") || "Type"}
                    <strong>{confirmationKeyword}</strong>
                    {$t("components.confirm_dialog.instruction_suffix") ||
                        "to confirm."}
                </p>
                <input
                    type="text"
                    bind:value={inputValue}
                    placeholder={$t("components.confirm_dialog.placeholder", {
                        values: { keyword: confirmationKeyword },
                    }) || `Type ${confirmationKeyword} here`}
                    class="confirm-input"
                />
            </div>
        {/if}
    </div>

    {#snippet footer()}
        <div class="actions">
            <button class="btn btn-ghost" onclick={handleCancel} disabled={loading}>
                {cancelText}
            </button>
            <button
                class="btn btn-{type}"
                onclick={handleConfirm}
                disabled={!canConfirm}
            >
                {#if loading}
                    <span class="spinner"></span>
                {/if}
                {confirmText}
            </button>
        </div>
    {/snippet}
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

    .confirmation-input {
        margin-top: 1.5rem;
        width: 100%;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .instruction {
        font-size: 0.9rem;
        color: var(--text-secondary);
        margin: 0;
    }

    .instruction strong {
        color: var(--text-primary);
        user-select: all;
    }

    .confirm-input {
        width: 100%;
        padding: 0.75rem;
        border: 1px solid var(--border-color, rgba(255, 255, 255, 0.1));
        background: var(--bg-primary, rgba(0, 0, 0, 0.2));
        color: var(--text-primary, white);
        border-radius: 6px;
        text-align: center;
        font-size: 1rem;
    }

    .confirm-input:focus {
        outline: none;
        border-color: var(--color-primary, #6366f1);
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
        opacity: 0.5;
        cursor: not-allowed;
        filter: grayscale(0.5);
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
