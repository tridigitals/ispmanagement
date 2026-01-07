<script lang="ts">
    import { user, isAuthenticated, isAdmin } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";

    onMount(() => {
        if (!$isAuthenticated) {
            goto("/login");
        }
    });
</script>

<div class="dashboard-content fade-in">
    <div class="page-header">
        <div>
            <h1>Hello, {$user?.name}!</h1>
            <p>Welcome to your personal dashboard.</p>
        </div>
    </div>

    {#if $isAdmin}
        <div class="admin-banner" on:click={() => goto('/admin')}>
            <div class="banner-content">
                <span class="icon">🛡️</span>
                <div>
                    <h3>Admin Access Available</h3>
                    <p>Click here to switch to the System Administration view.</p>
                </div>
            </div>
            <div class="arrow">→</div>
        </div>
    {/if}

    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-icon">👤</div>
            <div class="stat-content">
                <span class="stat-value">{$user?.role}</span>
                <span class="stat-label">Current Role</span>
            </div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">📅</div>
            <div class="stat-content">
                <span class="stat-value">{new Date($user?.created_at || Date.now()).toLocaleDateString()}</span>
                <span class="stat-label">Member Since</span>
            </div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">✅</div>
            <div class="stat-content">
                <span class="stat-value">Active</span>
                <span class="stat-label">Account Status</span>
            </div>
        </div>
    </div>

    <!-- Placeholder for user activity -->
    <div class="section-header">
        <h2>My Recent Activity</h2>
    </div>
    
    <div class="card empty-state">
        <div class="empty-icon">📝</div>
        <h3>No recent activity</h3>
        <p>Your actions and logs will appear here.</p>
    </div>
</div>

<style>
    .dashboard-content {
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

    .admin-banner {
        background: linear-gradient(135deg, rgba(99, 102, 241, 0.1), rgba(168, 85, 247, 0.1));
        border: 1px solid rgba(99, 102, 241, 0.2);
        border-radius: var(--border-radius);
        padding: 1.5rem;
        margin-bottom: 2rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        cursor: pointer;
        transition: all 0.2s ease;
    }

    .admin-banner:hover {
        background: linear-gradient(135deg, rgba(99, 102, 241, 0.15), rgba(168, 85, 247, 0.15));
        transform: translateY(-2px);
    }

    .banner-content {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .banner-content .icon {
        font-size: 1.5rem;
    }

    .banner-content h3 {
        font-size: 1.1rem;
        margin-bottom: 0.25rem;
        color: var(--color-primary-light);
    }

    .banner-content p {
        font-size: 0.9rem;
        color: var(--text-secondary);
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
        transition: transform 0.2s ease;
    }

    .stat-card:hover {
        transform: translateY(-2px);
    }

    .stat-icon {
        font-size: 2rem;
        opacity: 0.8;
    }

    .stat-content {
        display: flex;
        flex-direction: column;
    }

    .stat-value {
        font-size: 1.5rem;
        font-weight: 700;
        text-transform: capitalize;
    }

    .stat-label {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }

    .section-header {
        margin-top: 3rem;
        margin-bottom: 1rem;
    }

    .card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
    }

    .empty-state {
        padding: 4rem;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        text-align: center;
    }

    .empty-icon {
        font-size: 3rem;
        margin-bottom: 1rem;
        opacity: 0.5;
    }

    .empty-state h3 {
        font-size: 1.25rem;
        margin-bottom: 0.5rem;
    }

    .empty-state p {
        color: var(--text-secondary);
    }

    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: translateY(0); }
    }
</style>