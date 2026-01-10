<script lang="ts">
    import { isAuthenticated, isAdmin, can } from "$lib/stores/auth";
    import { team, settings } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";

    let stats = {
        members: 0,
        settings: 0,
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
            const [membersRes, settingsRes] = await Promise.all([
                team.list(),
                settings.getAll(),
            ]);

            stats.members = membersRes.length;
            stats.settings = settingsRes.length;
        } catch (err) {
            console.error("Failed to load admin stats:", err);
        } finally {
            loading = false;
        }
    });
</script>

<div class="admin-content fade-in">
    {#if loading}
        <div class="loading">Loading system data...</div>
    {:else}
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-icon">👥</div>
                <div class="stat-content">
                    <span class="stat-value">{stats.members}</span>
                    <span class="stat-label">Team Members</span>
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
            {#if $can("read", "team")}
                <button
                    class="action-card"
                    on:click={() => goto("/admin/team")}
                >
                    <div class="action-icon">👥</div>
                    <h3>Manage Team</h3>
                    <p>View, edit, and invite team members.</p>
                </button>
            {/if}

            {#if $can("read", "roles")}
                <button
                    class="action-card"
                    on:click={() => goto("/admin/roles")}
                >
                    <div class="action-icon">🔐</div>
                    <h3>Roles & Permissions</h3>
                    <p>Manage roles and access control.</p>
                </button>
            {/if}

            {#if $can("read", "settings")}
                <button
                    class="action-card"
                    on:click={() => goto("/admin/settings")}
                >
                    <div class="action-icon">⚙️</div>
                    <h3>Global Settings</h3>
                    <p>Configure application policies and defaults.</p>
                </button>
            {/if}
        </div>
    {/if}
</div>

<style>
    .admin-content {
        padding: 1.5rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .section-header {
        margin-top: 3rem;
        margin-bottom: 1.5rem;
    }

    .section-header h2 {
        font-size: 1.1rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 1.5rem;
    }

    .stat-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
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
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
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
        box-shadow: var(--shadow-sm);
    }

    .action-icon {
        font-size: 2rem;
        margin-bottom: 0.5rem;
    }

    .action-card h3 {
        font-size: 1.1rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .action-card p {
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .loading {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 50vh;
        color: var(--text-secondary);
    }

    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
</style>
