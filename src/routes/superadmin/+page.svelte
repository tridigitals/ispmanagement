<script lang="ts">
    import { api } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import { systemHealthCache } from "$lib/stores/systemHealth";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { t } from "svelte-i18n";

    let tenantTotal = $state(0);
    let tenantActive = $state(0);
    let userTotal = $state(0);
    let healthStatus = $state<"ok" | "down" | "unknown">("unknown");
    let loading = $state(true);

    onMount(async () => {
        await loadStats();
    });

    async function loadStats() {
        loading = true;
        try {
            const [tenantsRes, usersRes, healthRes] = await Promise.all([
                api.superadmin.listTenants(),
                api.users.list(1, 1),
                api.superadmin.getSystemHealth().catch(() => null),
            ]);

            const tenants = tenantsRes.data || [];

            tenantTotal = tenantsRes.total ?? tenants.length;
            tenantActive = tenants.filter((t: any) => t.is_active).length;

            userTotal = usersRes?.total ?? 0;
            if (healthRes) {
                systemHealthCache.set({ health: healthRes, fetchedAt: Date.now() });
            }
            healthStatus = healthRes?.database?.is_connected ? "ok" : "down";
        } catch (e) {
            console.error("Failed to load stats", e);
            healthStatus = "unknown";
        } finally {
            loading = false;
        }
    }
</script>

<div class="superadmin-content fade-in">
    {#if loading}
        <div class="loading">
            <div class="spinner"></div>
            <p>
                {$t("superadmin.dashboard.loading") ||
                    "Loading system data..."}
            </p>
        </div>
    {:else}
        <div
            class="stats-grid"
            aria-label={$t("superadmin.dashboard.stats_aria") || "Superadmin stats"}
        >
            <button
                class="stat-card cyan"
                onclick={() => goto("/superadmin/tenants")}
            >
                <div class="stat-icon">
                    <Icon name="database" size={32} />
                </div>
                <div class="stat-content">
                    <span class="stat-value">{tenantTotal}</span>
                    <span class="stat-label">
                        {$t("superadmin.dashboard.stats.tenants") || "Tenants"}
                    </span>
                </div>
            </button>

            <button
                class="stat-card emerald"
                onclick={() => goto("/superadmin/tenants")}
            >
                <div class="stat-icon">
                    <Icon name="check-circle" size={32} />
                </div>
                <div class="stat-content">
                    <span class="stat-value">{tenantActive}</span>
                    <span class="stat-label">
                        {$t("superadmin.dashboard.stats.active_tenants") ||
                            "Active Tenants"}
                    </span>
                </div>
            </button>

            <button
                class="stat-card indigo"
                onclick={() => goto("/superadmin/users")}
            >
                <div class="stat-icon">
                    <Icon name="users" size={32} />
                </div>
                <div class="stat-content">
                    <span class="stat-value">{userTotal}</span>
                    <span class="stat-label">
                        {$t("superadmin.dashboard.stats.users") || "Users"}
                    </span>
                </div>
            </button>

            <button
                class="stat-card amber"
                onclick={() => goto("/superadmin/system")}
            >
                <div class="stat-icon">
                    <Icon name="activity" size={32} />
                </div>
                <div class="stat-content">
                    <span class="stat-value">
                        {#if healthStatus === "ok"}
                            {$t("superadmin.dashboard.stats.health_ok") ||
                                "OK"}
                        {:else if healthStatus === "down"}
                            {$t("superadmin.dashboard.stats.health_down") ||
                                "DB Down"}
                        {:else}
                            {$t("common.loading") || "—"}
                        {/if}
                    </span>
                    <span class="stat-label">
                        {$t("superadmin.dashboard.stats.system_health") ||
                            "System Health"}
                    </span>
                </div>
            </button>
        </div>

        <div class="section-header">
            <h2>
                {$t("superadmin.dashboard.quick_actions.title") ||
                    "Quick Actions"}
            </h2>
        </div>

        <div class="actions-grid">
            <button
                class="action-card"
                onclick={() => goto("/superadmin/tenants")}
            >
                <div class="action-icon accent-cyan">
                    <Icon name="database" size={18} />
                </div>
                <h3>
                    {$t("superadmin.dashboard.quick_actions.tenants.title") ||
                        "Manage Tenants"}
                </h3>
                <p>
                    {$t("superadmin.dashboard.quick_actions.tenants.desc") ||
                        "Create, edit, and maintain organizations."}
                </p>
            </button>

            <button
                class="action-card"
                onclick={() => goto("/superadmin/users")}
            >
                <div class="action-icon accent-indigo">
                    <Icon name="users" size={18} />
                </div>
                <h3>
                    {$t("superadmin.dashboard.quick_actions.users.title") ||
                        "Manage Users"}
                </h3>
                <p>
                    {$t("superadmin.dashboard.quick_actions.users.desc") ||
                        "View global users, roles, and access."}
                </p>
            </button>

            <button
                class="action-card"
                onclick={() => goto("/superadmin/audit-logs")}
            >
                <div class="action-icon accent-emerald">
                    <Icon name="activity" size={18} />
                </div>
                <h3>
                    {$t("superadmin.dashboard.quick_actions.audit.title") ||
                        "Audit Logs"}
                </h3>
                <p>
                    {$t("superadmin.dashboard.quick_actions.audit.desc") ||
                        "Track activity and security events."}
                </p>
            </button>

            <button
                class="action-card"
                onclick={() => goto("/superadmin/settings")}
            >
                <div class="action-icon accent-amber">
                    <Icon name="settings" size={18} />
                </div>
                <h3>
                    {$t("superadmin.dashboard.quick_actions.settings.title") ||
                        "Platform Settings"}
                </h3>
                <p>
                    {$t("superadmin.dashboard.quick_actions.settings.desc") ||
                        "Configure policies and system defaults."}
                </p>
            </button>
        </div>
    {/if}
</div>

<style>
    .superadmin-content {
        padding: clamp(16px, 3vw, 32px);
        max-width: 1400px;
        margin: 0 auto;
        color: var(--text-primary);
        --accent-emerald: #10b981;
        --accent-cyan: #22d3ee;
        --accent-indigo: #6366f1;
        --accent-amber: #f59e0b;
        --glass: rgba(255, 255, 255, 0.04);
        --glass-border: rgba(255, 255, 255, 0.08);
    }

    .loading {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 2.5rem 1rem;
        gap: 0.75rem;
        color: var(--text-secondary);
    }

    .spinner {
        width: 28px;
        height: 28px;
        border: 2px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
        gap: 1.25rem;
        margin-bottom: 2rem;
    }

    .stat-card {
        background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
        border: 1px solid var(--glass-border);
        border-radius: 16px;
        padding: 1.35rem;
        display: flex;
        align-items: center;
        gap: 1.1rem;
        cursor: pointer;
        transition: all 0.2s;
        color: var(--text-primary);
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
        text-align: left;
    }

    .stat-card:hover {
        border-color: var(--color-primary);
        transform: translateY(-2px);
    }

    .stat-icon {
        width: 52px;
        height: 52px;
        border-radius: 14px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid rgba(255, 255, 255, 0.06);
    }

    .stat-content {
        display: flex;
        flex-direction: column;
        gap: 0.2rem;
        min-width: 0;
    }

    .stat-value {
        font-size: 1.55rem;
        font-weight: 800;
        letter-spacing: -0.02em;
    }

    .stat-label {
        color: var(--text-secondary);
        font-size: 0.92rem;
    }

    .stat-card.emerald {
        background: radial-gradient(circle at 20% 20%, rgba(16, 185, 129, 0.18), transparent 55%), #0c1411;
        border-color: rgba(16, 185, 129, 0.25);
    }

    .stat-card.cyan {
        background: radial-gradient(circle at 20% 20%, rgba(34, 211, 238, 0.18), transparent 55%), #081216;
        border-color: rgba(34, 211, 238, 0.25);
    }

    .stat-card.indigo {
        background: radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.18), transparent 55%), #0b0c17;
        border-color: rgba(99, 102, 241, 0.25);
    }

    .stat-card.amber {
        background: radial-gradient(circle at 20% 20%, rgba(245, 158, 11, 0.18), transparent 55%), #141009;
        border-color: rgba(245, 158, 11, 0.25);
    }

    .section-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin: 0 0 1rem 0;
    }

    .section-header h2 {
        margin: 0;
        font-size: 1.2rem;
        font-weight: 800;
        letter-spacing: -0.01em;
        color: var(--text-primary);
    }

    .actions-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 1rem;
    }

    .action-card {
        background: var(--glass);
        border: 1px solid var(--glass-border);
        border-radius: 16px;
        padding: 1.25rem;
        text-align: left;
        cursor: pointer;
        transition: all 0.2s;
        color: var(--text-primary);
        box-shadow: 0 12px 30px rgba(0, 0, 0, 0.25);
    }

    .action-card:hover {
        transform: translateY(-2px);
        border-color: rgba(99, 102, 241, 0.35);
    }

    .action-icon {
        width: 40px;
        height: 40px;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: 1px solid rgba(255, 255, 255, 0.08);
        margin-bottom: 0.85rem;
        background: rgba(255, 255, 255, 0.03);
    }

    .accent-emerald {
        color: var(--accent-emerald);
        background: rgba(16, 185, 129, 0.12);
        border-color: rgba(16, 185, 129, 0.25);
    }

    .accent-cyan {
        color: var(--accent-cyan);
        background: rgba(34, 211, 238, 0.12);
        border-color: rgba(34, 211, 238, 0.25);
    }

    .accent-indigo {
        color: var(--accent-indigo);
        background: rgba(99, 102, 241, 0.12);
        border-color: rgba(99, 102, 241, 0.25);
    }

    .accent-amber {
        color: var(--accent-amber);
        background: rgba(245, 158, 11, 0.12);
        border-color: rgba(245, 158, 11, 0.25);
    }

    .action-card h3 {
        margin: 0 0 0.35rem 0;
        font-size: 1rem;
        font-weight: 750;
    }

    .action-card p {
        margin: 0;
        color: var(--text-secondary);
        font-size: 0.92rem;
        line-height: 1.35;
    }

    :global([data-theme="light"]) .stat-card,
    :global([data-theme="light"]) .action-card {
        box-shadow:
            0 10px 24px rgba(0, 0, 0, 0.06),
            0 0 0 1px rgba(255, 255, 255, 0.85);
    }

    :global([data-theme="light"]) .stat-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
    }

    :global([data-theme="light"]) .stat-icon {
        background: rgba(0, 0, 0, 0.03);
        border-color: rgba(0, 0, 0, 0.06);
    }

    :global([data-theme="light"]) .action-card {
        background: #ffffff;
        border-color: rgba(0, 0, 0, 0.06);
    }

    @media (max-width: 768px) {
        .stats-grid {
            grid-template-columns: 1fr;
            gap: 0.9rem;
            margin-bottom: 1.25rem;
        }

        .actions-grid {
            grid-template-columns: 1fr;
        }
    }
</style>
