<script lang="ts">
    import { isAdmin, can, user } from "$lib/stores/auth";
    import { team, settings, api } from "$lib/api/client";
    import type { TenantSubscriptionDetails } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { get } from "svelte/store";
    import { page } from "$app/stores";
    import { getSlugFromDomain } from "$lib/utils/domain";
    import Icon from "$lib/components/Icon.svelte";

    let memberCount = $state(0);
    let settingsCount = $state(0);
    let subscription = $state<TenantSubscriptionDetails | null>(null);
    let loading = $state(true);

    // Dynamic Link Logic
    let domainSlug = $derived(getSlugFromDomain($page.url.hostname));
    let isCustomDomain = $derived(domainSlug && domainSlug === $user?.tenant_slug);
    let tenantPrefix = $derived($user?.tenant_slug && !isCustomDomain ? `/${$user.tenant_slug}` : "");

    onMount(() => {
        initData();
    });

    async function initData() {
        if (!get(isAdmin)) {
            goto("/unauthorized");
            return;
        }

        try {
            const [membersRes, settingsRes] = await Promise.all([
                team.list(),
                settings.getAll(),
            ]);

            memberCount = membersRes.length;
            settingsCount = settingsRes.length;

            const currentUser = get(user);
            if (currentUser?.tenant_id) {
                subscription = await api.plans.getSubscriptionDetails(currentUser.tenant_id);
            }
        } catch (err) {
            console.error("Failed to load admin stats:", err);
        } finally {
            loading = false;
        }
    }

    function formatBytes(bytes: number) {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
    }
</script>

<div class="admin-content fade-in">
    {#if loading}
        <div class="loading">
            <div class="spinner"></div>
            <p>Loading system data...</p>
        </div>
    {:else}
        <div class="stats-grid">
            <div class="stat-card" onclick={() => goto(`${tenantPrefix}/admin/team`)} role="button" tabindex="0">
                <div class="stat-icon">
                    <Icon name="users" size={32} />
                </div>
                <div class="stat-content">
                    <span class="stat-value">{memberCount}</span>
                    <span class="stat-label">Team Members</span>
                </div>
            </div>
            
            <div class="stat-card" onclick={() => goto(`${tenantPrefix}/admin/settings`)} role="button" tabindex="0">
                <div class="stat-icon">
                    <Icon name="settings" size={32} />
                </div>
                <div class="stat-content">
                    <span class="stat-value">{settingsCount}</span>
                    <span class="stat-label">Global Settings</span>
                </div>
            </div>

            <!-- Subscription Status Card -->
            <div class="stat-card featured" onclick={() => goto(`${tenantPrefix}/admin/subscription`)} role="button" tabindex="0">
                <div class="stat-icon">
                    <Icon name="credit-card" size={32} />
                </div>
                <div class="stat-content w-full">
                    {#if subscription}
                        <div class="plan-header">
                            <span class="plan-name">{subscription.plan_name}</span>
                            <span class="status-pill active">{subscription.status}</span>
                        </div>
                        <div class="progress-container">
                            <div 
                                class="progress-bar" 
                                style="width: {subscription.storage_limit ? Math.min(100, (subscription.storage_usage / subscription.storage_limit) * 100) : 0}%"
                            ></div>
                        </div>
                        <span class="usage-text">
                            {formatBytes(subscription.storage_usage)} used
                        </span>
                    {:else}
                        <span class="stat-value">Free</span>
                        <span class="stat-label">Plan Status</span>
                    {/if}
                </div>
            </div>
        </div>

        <div class="section-header">
            <h2>Quick Actions</h2>
        </div>

        <div class="actions-grid">
            {#if $can("read", "team")}
                <button
                    class="action-card"
                    onclick={() => goto(`${tenantPrefix}/admin/team`)}
                >
                    <div class="action-icon">👥</div>
                    <h3>Manage Team</h3>
                    <p>View, edit, and invite team members.</p>
                </button>
            {/if}

            {#if $can("read", "roles")}
                <button
                    class="action-card"
                    onclick={() => goto(`${tenantPrefix}/admin/roles`)}
                >
                    <div class="action-icon">🔐</div>
                    <h3>Roles & Permissions</h3>
                    <p>Manage roles and access control.</p>
                </button>
            {/if}

            {#if $can("read", "settings")}
                <button
                    class="action-card"
                    onclick={() => goto(`${tenantPrefix}/admin/settings`)}
                >
                    <div class="action-icon">⚙️</div>
                    <h3>Global Settings</h3>
                    <p>Configure application policies and defaults.</p>
                </button>
            {/if}
            
            <button
                class="action-card"
                onclick={() => goto(`${tenantPrefix}/admin/subscription`)}
            >
                <div class="action-icon">💳</div>
                <h3>Billing</h3>
                <p>Manage subscription and invoices.</p>
            </button>
        </div>
    {/if}
</div>

<style>
    .admin-content {
        padding: 2rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 1.5rem;
        margin-bottom: 3rem;
    }

    .stat-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.5rem;
        display: flex;
        align-items: center;
        gap: 1.25rem;
        cursor: pointer;
        transition: all 0.2s;
    }

    .stat-card:hover {
        border-color: var(--color-primary);
        transform: translateY(-2px);
    }

    .stat-card.featured {
        border-color: var(--color-primary);
        background: var(--color-primary-subtle);
    }

    .stat-icon {
        font-size: 2rem;
        width: 56px;
        height: 56px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--bg-app);
        border-radius: 12px;
    }

    .stat-content {
        flex: 1;
        display: flex;
        flex-direction: column;
    }

    .stat-value {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary);
    }

    .stat-label {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }

    /* Subscription Widget Specifics */
    .plan-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 0.5rem;
    }

    .plan-name {
        font-weight: 700;
        font-size: 1.1rem;
    }

    .status-pill {
        font-size: 0.7rem;
        padding: 0.15rem 0.5rem;
        border-radius: 10px;
        text-transform: uppercase;
        font-weight: 800;
    }

    .status-pill.active {
        background: rgba(34, 197, 94, 0.2);
        color: #22c55e;
    }

    .progress-container {
        height: 6px;
        background: var(--bg-tertiary);
        border-radius: 3px;
        overflow: hidden;
        margin-bottom: 0.4rem;
    }

    .progress-bar {
        height: 100%;
        background: var(--color-primary);
        border-radius: 3px;
    }

    .usage-text {
        font-size: 0.75rem;
        color: var(--text-secondary);
    }

    /* Actions Grid */
    .section-header {
        margin-bottom: 1.5rem;
    }

    .section-header h2 {
        font-size: 1.25rem;
        font-weight: 600;
    }

    .actions-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        gap: 1.5rem;
    }

    .action-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.5rem;
        text-align: left;
        cursor: pointer;
        transition: all 0.2s;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .action-card:hover {
        border-color: var(--color-primary);
        box-shadow: var(--shadow-md);
    }

    .action-icon {
        font-size: 1.5rem;
        margin-bottom: 0.5rem;
    }

    .action-card h3 {
        font-size: 1rem;
        font-weight: 600;
        margin: 0;
    }

    .action-card p {
        font-size: 0.85rem;
        color: var(--text-secondary);
        margin: 0;
    }

    .loading {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 40vh;
        gap: 1rem;
        color: var(--text-secondary);
    }

    .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @keyframes spin { to { transform: rotate(360deg); } }

    .fade-in {
        animation: fadeIn 0.4s ease-out;
    }

    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: translateY(0); }
    }

    .w-full { width: 100%; }
</style>