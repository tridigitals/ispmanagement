<script lang="ts">
    import Modal from "$lib/components/ui/Modal.svelte";
    import { t } from "svelte-i18n";
    import type { User } from "$lib/api/client";

    let {
        show = $bindable(false),
        user,
        getTenantName,
    } = $props<{
        show: boolean;
        user: User | null;
        getTenantName: (u: any) => string;
    }>();

    function formatDateMaybe(value: any) {
        if (!value) return "-";
        const d = new Date(value);
        if (Number.isNaN(d.getTime())) return "-";
        return d.toLocaleString();
    }
</script>

<Modal
    bind:show
    title={user
        ? $t("superadmin.users.details.title_with_name", {
              values: { name: user.name },
          }) || `User Details — ${user.name}`
        : $t("superadmin.users.details.title") || "User Details"}
    width="640px"
    onclose={() => {
        show = false;
        // Parent should handle clearing 'user' if needed, or we just close
    }}
>
    {#if user}
        <div class="details-grid">
            <div class="detail-card">
                <div class="detail-title">
                    {$t("superadmin.users.details.sections.account") ||
                        "Account"}
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.name") || "Name"}
                    </span>
                    <span class="detail-val">{user.name}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.email") || "Email"}
                    </span>
                    <span class="detail-val">{user.email}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.role") || "Role"}
                    </span>
                    <span class="detail-val">
                        {#if user.is_super_admin}
                            <span class="role-pill superadmin"
                                >{$t("sidebar.super_admin") ||
                                    "Super Admin"}</span
                            >
                        {:else}
                            <span class="role-pill {user.role}"
                                >{user.role}</span
                            >
                        {/if}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.status") ||
                            "Status"}
                    </span>
                    <span class="detail-val">
                        {#if user.is_active}
                            <span class="status-pill active">
                                <span class="dot"></span>
                                {$t("common.active") || "Active"}
                            </span>
                        {:else}
                            <span class="status-pill inactive">
                                <span class="dot"></span>
                                {$t("common.inactive") || "Inactive"}
                            </span>
                        {/if}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.created") ||
                            "Created"}
                    </span>
                    <span class="detail-val"
                        >{formatDateMaybe(user.created_at)}</span
                    >
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.last_login") ||
                            "Last Login"}
                    </span>
                    <span class="detail-val">
                        {formatDateMaybe(
                            (user as any).last_login_at ||
                                (user as any).last_login ||
                                (user as any).last_login_date,
                        )}
                    </span>
                </div>
            </div>

            <div class="detail-card">
                <div class="detail-title">
                    {$t("superadmin.users.details.sections.tenant") || "Tenant"}
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.tenant") ||
                            "Tenant"}
                    </span>
                    <span class="detail-val">
                        {#if getTenantName(user as any)}
                            {getTenantName(user as any)}
                        {:else}
                            -
                        {/if}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.slug") || "Slug"}
                    </span>
                    <span class="detail-val text-mono">
                        {(user as any).tenant_slug || "-"}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.tenant_role") ||
                            "Tenant Role"}
                    </span>
                    <span class="detail-val">
                        {(user as any).tenant_role || "-"}
                    </span>
                </div>
            </div>

            <div class="detail-card">
                <div class="detail-title">
                    {$t("superadmin.users.details.sections.security") ||
                        "Security"}
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.twofa_enabled") ||
                            "2FA Enabled"}
                    </span>
                    <span class="detail-val">
                        {(user as any).two_factor_enabled
                            ? $t("common.yes") || "Yes"
                            : $t("common.no") || "No"}
                    </span>
                </div>
                <div class="detail-row">
                    <span class="detail-key">
                        {$t("superadmin.users.details.labels.preferred_2fa") ||
                            "Preferred 2FA"}
                    </span>
                    <span class="detail-val">
                        {(user as any).preferred_2fa_method || "-"}
                    </span>
                </div>
            </div>
        </div>
    {/if}
</Modal>

<style>
    .details-grid {
        display: grid;
        grid-template-columns: 1fr;
        gap: 0.9rem;
    }

    @media (min-width: 720px) {
        .details-grid {
            grid-template-columns: 1fr 1fr;
        }

        .details-grid :global(.detail-card:nth-child(3)) {
            grid-column: 1 / -1;
        }
    }

    .detail-card {
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 14px;
        padding: 1rem;
    }

    :global([data-theme="light"]) .detail-card {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .detail-title {
        font-weight: 800;
        color: var(--text-primary);
        margin-bottom: 0.75rem;
    }

    .detail-row {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 1rem;
        padding: 0.5rem 0;
        border-top: 1px solid rgba(255, 255, 255, 0.06);
    }

    :global([data-theme="light"]) .detail-row {
        border-top-color: rgba(0, 0, 0, 0.06);
    }

    .detail-row:first-of-type {
        border-top: none;
        padding-top: 0;
    }

    .detail-key {
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .role-pill {
        padding: 0.3rem 0.8rem;
        border-radius: 20px;
        font-size: 0.8rem;
        font-weight: 600;
        text-transform: capitalize;
    }

    .role-pill.admin {
        background: rgba(99, 102, 241, 0.2);
        color: #818cf8;
    }

    .role-pill.superadmin {
        background: rgba(139, 92, 246, 0.2);
        color: #a78bfa;
        border: 1px solid rgba(139, 92, 246, 0.3);
    }

    .role-pill.user {
        background: rgba(16, 185, 129, 0.2);
        color: #34d399;
    }

    .status-pill {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.35rem 0.8rem;
        border-radius: 20px;
        font-size: 0.8rem;
        font-weight: 600;
    }

    .status-pill.active {
        background: rgba(16, 185, 129, 0.15);
        color: var(--color-success);
        border: 1px solid rgba(16, 185, 129, 0.2);
    }

    .status-pill.inactive {
        background: rgba(239, 68, 68, 0.15);
        color: var(--color-danger);
        border: 1px solid rgba(239, 68, 68, 0.2);
    }

    .dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: currentColor;
    }

    .text-mono {
        font-family: monospace;
        font-size: 0.85rem;
        color: var(--text-secondary);
    }
</style>
