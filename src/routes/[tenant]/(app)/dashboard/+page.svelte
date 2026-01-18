<script lang="ts">
    import { user, isAuthenticated, isAdmin, can } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { t } from "svelte-i18n";
    import Icon from "$lib/components/Icon.svelte";

    onMount(() => {
        // Auth handled by layout
    });

    const greeting = () => {
        const hour = new Date().getHours();
        if (hour < 12) return $t("dashboard.greeting.morning");
        if (hour < 17) return $t("dashboard.greeting.afternoon");
        return $t("dashboard.greeting.evening");
    };
</script>

<div class="dashboard-content fade-in">
    <header class="welcome-header">
        <div class="welcome-text">
            <h1>{greeting()}, {$user?.name}!</h1>
            <p>{$t("dashboard.greeting.welcome_message")}</p>
        </div>
        <div class="header-actions">
            {#if $can("upload", "storage")}
                <button class="btn btn-primary" onclick={() => goto(`/${$user?.tenant_slug}/storage`)}>
                    <Icon name="hard-drive" size={16} />
                    Manage Files
                </button>
            {/if}
        </div>
    </header>

    {#if $isAdmin}
        <div
            class="admin-banner"
            onclick={() => goto(`/${$user?.tenant_slug}/admin`)}
            onkeydown={(e) => e.key === "Enter" && goto(`/${$user?.tenant_slug}/admin`)}
            role="button"
            tabindex="0"
        >
            <div class="banner-content">
                <div class="banner-icon">
                    <Icon name="shield" size={24} />
                </div>
                <div>
                    <h3>{$t("dashboard.admin_mode.title")}</h3>
                    <p>{$t("dashboard.admin_mode.description")}</p>
                </div>
            </div>
            <Icon name="arrow-right" size={20} />
        </div>
    {/if}

    <!-- Stats Row (User Focused) -->
    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-header">
                <div class="icon-wrapper primary">
                    <Icon name="profile" size={20} />
                </div>
                <span class="trend positive">+2.4%</span>
            </div>
            <div class="stat-body">
                <span class="stat-value">{$user?.role}</span>
                <span class="stat-label">{$t("dashboard.stats.account_role")}</span>
            </div>
        </div>

        <div class="stat-card">
            <div class="stat-header">
                <div class="icon-wrapper success">
                    <Icon name="calendar" size={20} />
                </div>
            </div>
            <div class="stat-body">
                <span class="stat-value">{new Date($user?.created_at || Date.now()).toLocaleDateString()}</span>
                <span class="stat-label">{$t("dashboard.stats.member_since")}</span>
            </div>
        </div>

        <div class="stat-card">
            <div class="stat-header">
                <div class="icon-wrapper info">
                    <Icon name="check" size={20} />
                </div>
            </div>
            <div class="stat-body">
                <span class="stat-value">{$t("dashboard.stats.active")}</span>
                <span class="stat-label">{$t("dashboard.stats.system_status")}</span>
            </div>
        </div>
    </div>

    <div class="main-grid">
        <section class="activity-section">
            <div class="section-header">
                <h2>{$t("dashboard.recent_activity.title")}</h2>
                <button class="text-btn">{$t("dashboard.recent_activity.view_all")}</button>
            </div>

            <div class="card activity-card">
                <div class="empty-state">
                    <div class="empty-icon-circle">
                        <Icon name="activity" size={32} />
                    </div>
                    <h3>{$t("dashboard.recent_activity.empty.title")}</h3>
                    <p>{$t("dashboard.recent_activity.empty.description")}</p>
                    <button class="btn btn-secondary mt-4">{$t("dashboard.recent_activity.empty.learn_more")}</button>
                </div>
            </div>
        </section>

        <aside class="quick-actions">
            <div class="section-header">
                <h2>{$t("dashboard.quick_actions.title")}</h2>
            </div>
            <div class="actions-list">
                <button class="action-item" onclick={() => goto(`/${$user?.tenant_slug}/profile`)}>
                    <Icon name="profile" size={18} />
                    {$t("dashboard.quick_actions.update_profile")}
                </button>
                <button class="action-item">
                    <Icon name="mail" size={18} />
                    {$t("dashboard.quick_actions.check_messages")}
                </button>
                <button class="action-item">
                    <Icon name="lock" size={18} />
                    {$t("dashboard.quick_actions.security_settings")}
                </button>
                <button class="action-item">
                    <Icon name="help-circle" size={18} />
                    {$t("dashboard.quick_actions.contact_support")}
                </button>
            </div>
        </aside>
    </div>
</div>

<style>
    .dashboard-content {
        padding: 2rem;
        max-width: 1200px;
        margin: 0 auto;
        display: flex;
        flex-direction: column;
        gap: 2rem;
    }

    @media (max-width: 640px) {
        .dashboard-content {
            padding: 1rem;
            gap: 1.5rem;
        }

        .welcome-header {
            flex-direction: column;
            align-items: flex-start;
            gap: 1rem;
        }

        .header-actions {
            width: 100%;
        }

        .header-actions .btn {
            width: 100%;
            justify-content: center;
        }
    }

    /* Header */
    .welcome-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
        margin-bottom: 0.5rem;
    }

    .welcome-text h1 {
        font-size: 1.85rem;
        font-weight: 700;
        color: var(--text-primary);
        margin: 0 0 0.5rem 0;
    }

    .welcome-text p {
        color: var(--text-secondary);
        font-size: 1rem;
        margin: 0;
    }

    /* Admin Banner */
    .admin-banner {
        background: var(--color-primary);
        color: white;
        border-radius: var(--radius-lg);
        padding: 1.25rem 1.5rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        cursor: pointer;
        transition: transform 0.2s, box-shadow 0.2s;
        box-shadow: 0 4px 12px rgba(99, 102, 241, 0.2);
    }

    .admin-banner:hover {
        transform: translateY(-2px);
        box-shadow: 0 6px 16px rgba(99, 102, 241, 0.3);
    }

    .banner-content {
        display: flex;
        align-items: center;
        gap: 1.25rem;
    }

    .banner-icon {
        background: rgba(255, 255, 255, 0.2);
        width: 48px;
        height: 48px;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .banner-content h3 {
        margin: 0 0 0.15rem 0;
        font-size: 1.1rem;
        font-weight: 600;
    }

    .banner-content p {
        margin: 0;
        font-size: 0.9rem;
        opacity: 0.9;
    }

    /* Stats Grid */
    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: 1.5rem;
    }

    @media (max-width: 640px) {
        .stats-grid {
            grid-template-columns: 1fr;
            gap: 1rem;
        }
    }

    .stat-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg);
        padding: 1.5rem;
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
        transition: border-color 0.2s;
    }

    .stat-card:hover {
        border-color: var(--color-primary);
    }

    .stat-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .icon-wrapper {
        width: 40px;
        height: 40px;
        border-radius: 10px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .icon-wrapper.primary {
        background: rgba(99, 102, 241, 0.1);
        color: var(--color-primary);
    }
    .icon-wrapper.success {
        background: rgba(34, 197, 94, 0.1);
        color: #22c55e;
    }
    .icon-wrapper.info {
        background: rgba(59, 130, 246, 0.1);
        color: #3b82f6;
    }

    .trend {
        font-size: 0.75rem;
        font-weight: 600;
        padding: 0.25rem 0.5rem;
        border-radius: 20px;
    }

    .trend.positive {
        background: rgba(34, 197, 94, 0.1);
        color: #22c55e;
    }

    .stat-body {
        display: flex;
        flex-direction: column;
    }

    .stat-value {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary);
        text-transform: capitalize;
    }

    .stat-label {
        font-size: 0.875rem;
        color: var(--text-secondary);
        font-weight: 500;
    }

    /* Main Grid */
    .main-grid {
        display: grid;
        grid-template-columns: 2fr 1fr;
        gap: 2rem;
    }

    @media (max-width: 900px) {
        .main-grid {
            grid-template-columns: 1fr;
        }
    }

    .section-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }

    .section-header h2 {
        font-size: 1.15rem;
        font-weight: 600;
        color: var(--text-primary);
        margin: 0;
    }

    .text-btn {
        background: transparent;
        border: none;
        color: var(--color-primary);
        font-size: 0.875rem;
        font-weight: 600;
        cursor: pointer;
    }

    /* Activity Card */
    .activity-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg);
        min-height: 300px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .empty-state {
        text-align: center;
        padding: 2rem;
        max-width: 320px;
    }

    .empty-icon-circle {
        width: 64px;
        height: 64px;
        background: var(--bg-tertiary);
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0 auto 1.5rem;
        color: var(--text-secondary);
        opacity: 0.5;
    }

    .empty-state h3 {
        font-size: 1.1rem;
        font-weight: 600;
        margin-bottom: 0.5rem;
    }

    .empty-state p {
        color: var(--text-secondary);
        font-size: 0.9rem;
        line-height: 1.5;
    }

    /* Quick Actions */
    .actions-list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .action-item {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        padding: 1rem;
        display: flex;
        align-items: center;
        gap: 1rem;
        font-size: 0.9rem;
        font-weight: 500;
        color: var(--text-primary);
        cursor: pointer;
        transition: all 0.2s;
        text-align: left;
    }

    .action-item:hover {
        border-color: var(--color-primary);
        background: var(--bg-hover);
        transform: translateX(4px);
    }

    /* Buttons */
    .btn {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.6rem 1.2rem;
        border-radius: 8px;
        font-size: 0.9rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover {
        filter: brightness(1.1);
    }

    .btn-secondary {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .mt-4 {
        margin-top: 1rem;
    }

    .fade-in {
        animation: fadeIn 0.4s ease-out;
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
