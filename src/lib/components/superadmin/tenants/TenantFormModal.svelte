<script lang="ts">
    import Modal from "$lib/components/ui/Modal.svelte";
    import Input from "$lib/components/ui/Input.svelte";
    import Select from "$lib/components/ui/Select.svelte";
    import Icon from "$lib/components/ui/Icon.svelte";
    import { t } from "svelte-i18n";

    let {
        show = $bindable(false),
        isEditing = false,
        newTenant = $bindable(),
        plans = [],
        loading = false,
        onSubmit,
        onGenerateSlug,
    } = $props<{
        show: boolean;
        isEditing: boolean;
        newTenant: {
            name: string;
            slug: string;
            customDomain: string;
            ownerEmail: string;
            ownerPassword: string;
            isActive: boolean;
            planId: string;
        };
        plans: { label: string; value: string }[];
        loading: boolean;
        onSubmit: () => void;
        onGenerateSlug: () => void;
    }>();

    let showPassword = $state(false);
</script>

<Modal
    bind:show
    title={isEditing
        ? $t("superadmin.tenants.modal.edit_title") || "Edit Tenant"
        : $t("superadmin.tenants.modal.create_title") || "Create New Tenant"}
>
    <div class="modal-form">
        <Input
            label={$t("superadmin.tenants.modal.labels.name") || "Tenant Name"}
            bind:value={newTenant.name}
            oninput={onGenerateSlug}
            placeholder={$t("superadmin.tenants.modal.placeholders.name") ||
                "e.g. Acme Corp"}
        />

        <Input
            label={$t("superadmin.tenants.modal.labels.slug") || "Slug (URL)"}
            bind:value={newTenant.slug}
            placeholder={$t("superadmin.tenants.modal.placeholders.slug") ||
                "e.g. acme-corp"}
            disabled={isEditing}
        />

        <Input
            label={$t("superadmin.tenants.modal.labels.custom_domain") ||
                "Custom Domain (Optional)"}
            bind:value={newTenant.customDomain}
            placeholder={$t(
                "superadmin.tenants.modal.placeholders.custom_domain",
            ) || "e.g. app.acme.com"}
        />

        {#if !isEditing}
            <div class="divider">
                <span>
                    {$t(
                        "superadmin.tenants.modal.sections.initial_subscription",
                    ) || "Initial Subscription"}
                </span>
            </div>

            <Select
                label={$t("superadmin.tenants.modal.labels.plan") ||
                    "Subscription Plan"}
                options={plans}
                bind:value={newTenant.planId}
                placeholder={$t("superadmin.tenants.modal.placeholders.plan") ||
                    "Select a plan"}
            />

            <div class="divider">
                <span>
                    {$t("superadmin.tenants.modal.sections.initial_admin") ||
                        "Initial Admin User"}
                </span>
            </div>

            <Input
                label={$t("superadmin.tenants.modal.labels.owner_email") ||
                    "Owner Email"}
                type="email"
                bind:value={newTenant.ownerEmail}
                placeholder={$t(
                    "superadmin.tenants.modal.placeholders.owner_email",
                ) || "admin@acme.com"}
            />

            <div class="password-group">
                <Input
                    label={$t(
                        "superadmin.tenants.modal.labels.owner_password",
                    ) || "Owner Password"}
                    type={showPassword ? "text" : "password"}
                    bind:value={newTenant.ownerPassword}
                    placeholder={$t(
                        "superadmin.tenants.modal.placeholders.owner_password",
                    ) || "Strong password"}
                />
                <button
                    class="toggle-password"
                    onclick={() => (showPassword = !showPassword)}
                    type="button"
                >
                    <Icon name={showPassword ? "eye-off" : "eye"} size={18} />
                </button>
            </div>
        {/if}

        <div class="form-group toggle-row">
            <label>
                <input type="checkbox" bind:checked={newTenant.isActive} />
                <span class="toggle-label">
                    {$t("superadmin.tenants.modal.labels.active_status") ||
                        "Active Status"}
                </span>
            </label>
        </div>

        <div class="modal-actions">
            <button
                class="btn btn-secondary"
                onclick={() => (show = false)}
                disabled={loading}
            >
                {$t("common.cancel") || "Cancel"}
            </button>
            <button
                class="btn btn-primary"
                onclick={onSubmit}
                disabled={loading}
            >
                {#if loading}
                    <div class="spinner-sm"></div>
                {/if}
                {isEditing
                    ? $t("superadmin.tenants.modal.actions.update") ||
                      "Update Tenant"
                    : $t("superadmin.tenants.modal.actions.create") ||
                      "Create Tenant"}
            </button>
        </div>
    </div>
</Modal>

<style>
    /* Modal Form Styles */
    .modal-form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .divider {
        display: flex;
        align-items: center;
        gap: 1rem;
        margin: 1rem 0;
        color: var(--text-secondary);
        font-size: 0.8rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .divider::before,
    .divider::after {
        content: "";
        flex: 1;
        height: 1px;
        background: var(--border-color);
    }

    .password-group {
        position: relative;
    }

    .toggle-password {
        position: absolute;
        right: 0;
        top: 28px; /* Approximate alignment with input content */
        background: transparent;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0.5rem;
    }

    .toggle-row {
        margin-top: 0.5rem;
    }

    .toggle-row label {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        cursor: pointer;
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.75rem;
        margin-top: 1.5rem;
    }

    .spinner-sm {
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

    .btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.25rem;
        border-radius: var(--radius-md);
        font-weight: 600;
        font-size: 0.9rem;
        border: none;
        cursor: pointer;
        transition: all 0.2s;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover {
        background: var(--color-primary-hover);
    }

    .btn-secondary {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .btn-secondary:hover {
        background: var(--bg-hover);
    }
</style>
