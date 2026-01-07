<script lang="ts">
    import { isAuthenticated, isAdmin } from "$lib/stores/auth";
    import { users, settings } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";

    let stats = {
        users: 0,
        settings: 0,
        active: 0
    };
    let loading = true;

    onMount(async () => {
        if (!$isAuthenticated) {
            goto("/login");
            return;
        }
        if (!$isAdmin) {
            goto("/dashboard");
            return;
        }

        try {
            const [usersRes, settingsRes] = await Promise.all([
                users.list(1, 1), // Minimal fetch for count
                settings.getAll(),
            ]);
            
            stats.users = usersRes.total;
            stats.settings = settingsRes.length;
            stats.active = usersRes.total; // Rough approx for now
        } catch (err) {
            console.error("Failed to load admin stats:", err);
        } finally {
            loading = false;
        }
    });
</script>

<div class="admin-content fade-in">
    <div class="page-header">
        <div>
            <h1>Admin Overview</h1>
            <p>System status and quick access.</p>
        </div>
    </div>

    {#if loading}
        <div class="loading">Loading system data...</div>
    {:else}
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-icon">👥</div>
                <div class="stat-content">
                    <span class="stat-value">{stats.users}</span>
                    <span class="stat-label">Total Users</span>
                </div>
            </div>
            <div class="stat-card">
                <div class="stat-icon">⚙️</div>
                <div class="stat-content">
                    <span class="stat-value">{stats.settings}</span>
                    <span class="stat-label">Global Settings</span>
                </div>
            </div>
            <div class="stat-card">
                <div class="stat-icon">⚡</div>
                <div class="stat-content">
                    <span class="stat-value">Online</span>
                    <span class="stat-label">System Status</span>
                </div>
            </div>
        </div>

        <div class="section-header">
            <h2>Quick Actions</h2>
        </div>

        <div class="actions-grid">
            <button class="action-card" on:click={() => goto('/admin/users')}>
                <div class="action-icon">👥</div>
                <h3>Manage Users</h3>
                <p>View, edit, and create system users.</p>
            </button>

            <button class="action-card" on:click={() => goto('/admin/settings')}>
                <div class="action-icon">⚙️</div>
                <h3>Global Settings</h3>
                <p>Configure application policies and defaults.</p>
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

    .page-header {
        margin-bottom: 2rem;
    }

    .page-header h1 {
        font-size: 1.75rem;
        margin-bottom: 0.5rem;
    }

    .page-header p {
        color: var(--text-secondary);
    }

    .section-header {
        margin-top: 3rem;
        margin-bottom: 1.5rem;
    }

    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 1.5rem;
    }

    .stat-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
        padding: 1.5rem;
        display: flex;
        align-items: center;
        gap: 1.25rem;
    }

    .stat-icon {
        font-size: 2.5rem;
        opacity: 0.8;
    }

    .stat-content {
        display: flex;
        flex-direction: column;
    }

    .stat-value {
        font-size: 1.75rem;
        font-weight: 700;
        line-height: 1.2;
    }

    .stat-label {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }

    /* Actions Grid */
    .actions-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 1.5rem;
    }

    .action-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
        padding: 2rem;
        text-align: left;
        cursor: pointer;
        transition: all 0.2s ease;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .action-card:hover {
        border-color: var(--color-primary);
        transform: translateY(-2px);
        box-shadow: var(--shadow-md);
    }

    .action-icon {
        font-size: 2rem;
        margin-bottom: 0.5rem;
    }

    .action-card h3 {
        font-size: 1.25rem;
        color: var(--text-primary);
    }

    .action-card p {
        color: var(--text-secondary);
        font-size: 0.95rem;
    }

    .loading {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 50vh;
        color: var(--text-secondary);
    }

    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: translateY(0); }
    }
</style>