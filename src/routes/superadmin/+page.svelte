<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import StatsCard from "$lib/components/StatsCard.svelte";
    import { fade } from "svelte/transition";

    let stats = {
        totalTenants: 0,
        activeTenants: 0,
        totalUsers: 0,
        activeUsers: 0,
    };
    let loading = true;

    onMount(async () => {
        loadStats();
    });

    async function loadStats() {
        loading = true;
        try {
            // In a real app, you might have a specific stats endpoint
            // For now, we can derive some stats from lists or use existing endpoints
            const tenantsRes = await api.superadmin.listTenants();
            const tenants = tenantsRes.data || [];

            // Basic approximation if no dedicated stats endpoint exists yet
            stats.totalTenants = tenants.length;
            stats.activeTenants = tenants.filter(
                (t: any) => t.is_active,
            ).length;

            // If there's a user list endpoint, fetch it too, else mock or leave 0
            // const usersRes = await api.users.list();
            // stats.totalUsers = usersRes.data.length;
        } catch (e) {
            console.error("Failed to load stats", e);
        } finally {
            loading = false;
        }
    }
</script>

<div class="page-container fade-in">
    <div class="header">
        <div class="header-content">
            <h1>Dashboard</h1>
            <p class="subtitle">Platform overview and statistics</p>
        </div>
    </div>

    <!-- Stats Grid -->
    <div class="stats-grid">
        <StatsCard
            title="Total Tenants"
            value={stats.totalTenants}
            icon="building"
            trend="0%"
            color="primary"
        />
        <StatsCard
            title="Active Tenants"
            value={stats.activeTenants}
            icon="check-circle"
            color="success"
        />
        <StatsCard
            title="Total Users"
            value={stats.totalUsers}
            icon="users"
            color="info"
        />
        <StatsCard
            title="System Health"
            value="100%"
            icon="activity"
            color="warning"
        />
    </div>

    <div class="quick-actions card">
        <h3>Quick Actions</h3>
        <div class="action-buttons">
            <a href="/superadmin/tenants" class="action-card">
                <div class="icon-bg primary">
                    <Icon name="building" size={24} />
                </div>
                <div class="action-text">
                    <h4>Manage Tenants</h4>
                    <p>Create, edit, or remove organizations</p>
                </div>
                <Icon name="arrow-right" size={20} class="arrow" />
            </a>

            <a href="/superadmin/users" class="action-card">
                <div class="icon-bg info">
                    <Icon name="users" size={24} />
                </div>
                <div class="action-text">
                    <h4>Manage Users</h4>
                    <p>View global user base and roles</p>
                </div>
                <Icon name="arrow-right" size={20} class="arrow" />
            </a>

            <a href="/superadmin/settings" class="action-card">
                <div class="icon-bg warning">
                    <Icon name="settings" size={24} />
                </div>
                <div class="action-text">
                    <h4>Platform Settings</h4>
                    <p>Configure global system policies</p>
                </div>
                <Icon name="arrow-right" size={20} class="arrow" />
            </a>
        </div>
    </div>
</div>

<style>
    .page-container {
        padding: 2rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .header {
        margin-bottom: 2rem;
    }

    h1 {
        font-size: 1.8rem;
        font-weight: 700;
        margin: 0 0 0.5rem 0;
        color: var(--text-primary);
    }

    .subtitle {
        color: var(--text-secondary);
        font-size: 0.95rem;
        margin: 0;
    }

    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2rem;
    }

    .quick-actions {
        padding: 1.5rem;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg);
    }

    .quick-actions h3 {
        margin: 0 0 1.5rem 0;
        font-size: 1.1rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .action-buttons {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 1.5rem;
    }

    .action-card {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1.25rem;
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        text-decoration: none;
        transition: all 0.2s;
        color: inherit;
    }

    .action-card:hover {
        border-color: var(--color-primary);
        transform: translateY(-2px);
        box-shadow: var(--shadow-sm);
    }

    .icon-bg {
        width: 48px;
        height: 48px;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--bg-surface);
    }

    .icon-bg.primary {
        background: rgba(var(--color-primary-rgb), 0.1);
        color: var(--color-primary);
    }
    .icon-bg.info {
        background: rgba(59, 130, 246, 0.1);
        color: #3b82f6;
    }
    .icon-bg.warning {
        background: rgba(245, 158, 11, 0.1);
        color: #f59e0b;
    }

    .action-text {
        flex: 1;
    }

    .action-text h4 {
        margin: 0 0 0.25rem 0;
        font-size: 1rem;
        color: var(--text-primary);
    }
</style>
