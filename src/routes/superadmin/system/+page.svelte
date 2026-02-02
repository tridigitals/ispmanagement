<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { api } from "$lib/api/client";
    import { isSuperAdmin } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import Icon from "$lib/components/ui/Icon.svelte";
    import StatsCard from "$lib/components/dashboard/StatsCard.svelte";
    import {
        systemHealthCache,
        type SystemHealth,
    } from "$lib/stores/systemHealth";
    import { fade } from "svelte/transition";
    import { t } from "svelte-i18n";

    let health: SystemHealth | null = null;
    let loading = true;
    let error = "";
    let refreshInterval: ReturnType<typeof setInterval>;

    onMount(() => {
        if (!$isSuperAdmin) {
            goto("/dashboard");
            return;
        }

        if ($systemHealthCache.health) {
            health = $systemHealthCache.health;
            loading = false;
            // Refresh in background to avoid UI flash
            void loadHealth();
        } else {
            void loadHealth();
        }
        // Auto-refresh every 30 seconds
        refreshInterval = setInterval(loadHealth, 30000);
    });

    onDestroy(() => {
        if (refreshInterval) clearInterval(refreshInterval);
    });

    async function loadHealth() {
        try {
            health = await api.superadmin.getSystemHealth();
            systemHealthCache.set({ health, fetchedAt: Date.now() });
            error = "";
        } catch (e: any) {
            console.error("Failed to load system health:", e);
            error = e.message || "Failed to load system health";
        } finally {
            loading = false;
        }
    }

    function formatUptime(seconds: number): string {
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);

        if (days > 0) return `${days}d ${hours}h ${minutes}m`;
        if (hours > 0) return `${hours}h ${minutes}m`;
        return `${minutes}m`;
    }

    function formatTime(isoString: string): string {
        return new Date(isoString).toLocaleTimeString();
    }

    function getActionColor(action: string): string {
        const prefix = action.split("_")[0].toLowerCase();
        switch (prefix) {
            case "user":
                return "info";
            case "tenant":
                return "success";
            case "auth":
                return "warning";
            case "settings":
                return "primary";
            default:
                return "muted";
        }
    }

    function formatBytes(bytes: number, decimals = 2) {
        if (bytes === 0) return "0 Bytes";
        const k = 1024;
        const dm = decimals < 0 ? 0 : decimals;
        const sizes = ["Bytes", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return (
            parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i]
        );
    }
</script>

<div class="page-container fade-in">
    <div class="page-header">
        <div class="header-content">
            <h1>{$t("superadmin.system.title") || "System Health"}</h1>
            <p class="subtitle">
                {$t("superadmin.system.subtitle") ||
                    "Monitor platform status and metrics"}
            </p>
        </div>
        <button
            class="btn-refresh"
            on:click={loadHealth}
            title={$t("common.refresh") || "Refresh"}
            aria-label={$t("common.refresh") || "Refresh"}
        >
            <Icon name="refresh-cw" size={18} />
        </button>
    </div>

    {#if loading}
        <div class="loading-state">
            <div class="spinner"></div>
            <p>
                {$t("superadmin.system.loading") || "Loading system health..."}
            </p>
        </div>
    {:else if error}
        <div class="error-card">
            <Icon name="alert-circle" size={24} />
            <p>{error}</p>
            <button class="btn btn-primary" on:click={loadHealth}>
                {$t("superadmin.system.retry") || "Retry"}
            </button>
        </div>
    {:else if health}
        <!-- Database Status Banner -->
        <div
            class="status-banner"
            class:connected={health.database.is_connected}
            class:disconnected={!health.database.is_connected}
        >
            <div class="status-icon">
                <Icon
                    name={health.database.is_connected
                        ? "check-circle"
                        : "x-circle"}
                    size={24}
                />
            </div>
            <div class="status-info">
                <h3>
                    {health.database.is_connected
                        ? $t("superadmin.system.db_connected") ||
                          "Database Connected"
                        : $t("superadmin.system.db_disconnected") ||
                          "Database Disconnected"}
                </h3>
                <p>
                    {health.database.database_type} • {health.database
                        .total_tables}
                    {$t("superadmin.system.tables") || "tables"} • {formatBytes(
                        health.database.database_size_bytes,
                    )}
                </p>
            </div>
            <div class="status-meta">
                <span class="version-badge">v{health.app_version}</span>
                <span class="os-badge"
                    >{health.resources.os_name}
                    {health.resources.os_version}</span
                >
                <span class="uptime"
                    >{$t("superadmin.system.uptime") || "Uptime:"}
                    {formatUptime(health.uptime_seconds)}</span
                >
            </div>
        </div>

        <div class="grid-2">
            <div class="card resource-card">
                <div class="card-header">
                    <Icon name="cpu" size={20} />
                    <h3>{$t("superadmin.system.cpu_usage") || "CPU Usage"}</h3>
                </div>
                <div class="card-body">
                    <div class="resource-header">
                        <span class="resource-value"
                            >{health.resources.cpu_usage.toFixed(1)}%</span
                        >
                    </div>
                    <div class="progress-bar">
                        <div
                            class="progress-fill"
                            style="width: {health.resources.cpu_usage}%"
                        ></div>
                    </div>
                </div>
            </div>
            <div class="card resource-card">
                <div class="card-header">
                    <Icon name="activity" size={20} />
                    <h3>
                        {$t("superadmin.system.memory_usage") || "Memory Usage"}
                    </h3>
                </div>
                <div class="card-body">
                    <div class="resource-header">
                        <span class="resource-value">
                            {formatBytes(health.resources.memory_used_bytes)} / {formatBytes(
                                health.resources.memory_total_bytes,
                            )}
                        </span>
                    </div>
                    <div class="progress-bar">
                        <div
                            class="progress-fill"
                            style="width: {(health.resources.memory_used_bytes /
                                health.resources.memory_total_bytes) *
                                100}%"
                        ></div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Request Metrics Section -->
        {#if health.request_metrics}
            <div class="section-header-inline">
                <Icon name="bar-chart-2" size={18} />
                <h3>
                    {$t("superadmin.system.request_metrics.title") ||
                        "Request Metrics"}
                </h3>
            </div>
            <div class="metrics-grid">
                <div class="metric-card">
                    <div class="metric-icon requests">
                        <Icon name="zap" size={18} />
                    </div>
                    <div class="metric-content">
                        <span class="metric-value"
                            >{health.request_metrics.requests_last_minute}</span
                        >
                        <span class="metric-label">Requests/min</span>
                    </div>
                </div>
                <div class="metric-card">
                    <div class="metric-icon response-time">
                        <Icon name="clock" size={18} />
                    </div>
                    <div class="metric-content">
                        <span class="metric-value"
                            >{health.request_metrics.avg_response_time_ms.toFixed(
                                1,
                            )}ms</span
                        >
                        <span class="metric-label">Avg Response</span>
                    </div>
                </div>
                <div class="metric-card">
                    <div class="metric-icon p95">
                        <Icon name="trending-up" size={18} />
                    </div>
                    <div class="metric-content">
                        <span class="metric-value"
                            >{health.request_metrics.p95_response_time_ms.toFixed(
                                1,
                            )}ms</span
                        >
                        <span class="metric-label">P95 Latency</span>
                    </div>
                </div>
                <div class="metric-card">
                    <div class="metric-icon total">
                        <Icon name="activity" size={18} />
                    </div>
                    <div class="metric-content">
                        <span class="metric-value"
                            >{health.request_metrics.total_requests.toLocaleString()}</span
                        >
                        <span class="metric-label">Total Requests</span>
                    </div>
                </div>
                <div class="metric-card">
                    <div class="metric-icon errors">
                        <Icon name="alert-triangle" size={18} />
                    </div>
                    <div class="metric-content">
                        <span class="metric-value"
                            >{health.request_metrics.error_count}</span
                        >
                        <span class="metric-label">Errors</span>
                    </div>
                </div>
                <div class="metric-card">
                    <div class="metric-icon rate-limited">
                        <Icon name="shield" size={18} />
                    </div>
                    <div class="metric-content">
                        <span class="metric-value"
                            >{health.request_metrics.rate_limited_count}</span
                        >
                        <span class="metric-label">Rate Limited</span>
                    </div>
                </div>
            </div>
        {/if}

        <!-- Stats Grid -->
        <div class="stats-grid">
            <StatsCard
                title={$t("superadmin.system.stats.total_tenants") ||
                    "Total Tenants"}
                value={health.database.tenants_count}
                icon="building"
                color="primary"
            />
            <StatsCard
                title={$t("superadmin.system.stats.total_users") ||
                    "Total Users"}
                value={health.database.users_count}
                icon="users"
                color="info"
            />
            <StatsCard
                title={$t("superadmin.system.stats.active_users") ||
                    "Active Users"}
                value={health.active_sessions}
                icon="user-check"
                color="success"
            />
            <StatsCard
                title={$t("superadmin.system.stats.audit_logs") || "Audit Logs"}
                value={health.database.audit_logs_count}
                icon="activity"
                color="warning"
            />
        </div>

        <div class="grid-2">
            <!-- Table Stats -->
            <div class="card">
                <div class="card-header">
                    <Icon name="database" size={20} />
                    <h3>
                        {$t("superadmin.system.database_tables") ||
                            "Database Tables"}
                    </h3>
                </div>
                <div class="card-body">
                    <table class="mini-table">
                        <thead>
                            <tr>
                                <th>
                                    {$t(
                                        "superadmin.system.table_headers.table",
                                    ) || "Table"}
                                </th>
                                <th class="text-right">
                                    {$t(
                                        "superadmin.system.table_headers.rows",
                                    ) || "Rows"}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each health.tables as table}
                                <tr>
                                    <td>
                                        <code>{table.name}</code>
                                    </td>
                                    <td class="text-right">
                                        <span class="row-count"
                                            >{table.row_count.toLocaleString()}</span
                                        >
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- Recent Activity -->
            <div class="card">
                <div class="card-header">
                    <Icon name="clock" size={20} />
                    <h3>
                        {$t("superadmin.system.recent_activity") ||
                            "Recent Activity"}
                    </h3>
                </div>
                <div class="card-body">
                    {#if health.recent_activity.length === 0}
                        <div class="empty-mini">
                            <p>
                                {$t("superadmin.system.no_recent_activity") ||
                                    "No recent activity"}
                            </p>
                        </div>
                    {:else}
                        <div class="activity-list">
                            {#each health.recent_activity as activity}
                                <div class="activity-item" in:fade>
                                    <div
                                        class="activity-dot {getActionColor(
                                            activity.action,
                                        )}"
                                    ></div>
                                    <div class="activity-content">
                                        <span class="activity-action"
                                            >{activity.action}</span
                                        >
                                        <span class="activity-resource"
                                            >{activity.resource}</span
                                        >
                                    </div>
                                    <div class="activity-meta">
                                        {#if activity.user_email}
                                            <span class="activity-user"
                                                >{activity.user_email}</span
                                            >
                                        {/if}
                                        <span class="activity-time"
                                            >{formatTime(
                                                activity.created_at,
                                            )}</span
                                        >
                                    </div>
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>
            </div>
        </div>

        <!-- Last Updated -->
        <div class="last-updated">
            <Icon name="clock" size={14} />
            {$t("superadmin.system.last_updated") || "Last updated:"}
            {new Date(health.collected_at).toLocaleString()}
        </div>
    {/if}
</div>

<style>
    .page-container {
        padding: 2rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .page-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 2rem;
    }

    .header-content h1 {
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

    .btn-refresh {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
        width: 40px;
        height: 40px;
        border-radius: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: all 0.2s;
    }

    .btn-refresh:hover {
        border-color: var(--color-primary);
        color: var(--color-primary);
    }

    /* Status Banner */
    .status-banner {
        display: flex;
        align-items: center;
        gap: 1.5rem;
        padding: 1.5rem 2rem;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg);
        margin-bottom: 2rem;
    }

    .status-banner.connected {
        border-left: 4px solid var(--color-success);
    }

    .status-banner.disconnected {
        border-left: 4px solid var(--color-danger);
        background: rgba(239, 68, 68, 0.05);
    }

    .status-icon {
        width: 48px;
        height: 48px;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .connected .status-icon {
        background: rgba(16, 185, 129, 0.1);
        color: var(--color-success);
    }

    .disconnected .status-icon {
        background: rgba(239, 68, 68, 0.1);
        color: var(--color-danger);
    }

    .status-info {
        flex: 1;
    }

    .status-info h3 {
        margin: 0 0 0.25rem 0;
        font-size: 1.1rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .status-info p {
        margin: 0;
        font-size: 0.9rem;
        color: var(--text-secondary);
    }

    .status-meta {
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: 0.5rem;
    }

    .version-badge {
        background: var(--color-primary);
        color: white;
        padding: 0.25rem 0.75rem;
        border-radius: 20px;
        font-size: 0.75rem;
        font-weight: 600;
    }

    .os-badge {
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
        padding: 0.2rem 0.6rem;
        border-radius: 4px;
        font-size: 0.75rem;
        font-family: monospace;
    }

    .uptime {
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    /* Resource Cards */
    .resource-card .card-body {
        padding: 1.5rem;
    }

    .resource-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 0.75rem;
    }

    .resource-value {
        font-size: 1.25rem;
        font-weight: 700;
        color: var(--text-primary);
    }

    .progress-bar {
        height: 8px;
        background: var(--bg-app);
        border-radius: 4px;
        overflow: hidden;
    }

    .progress-fill {
        height: 100%;
        background: var(--color-primary);
        border-radius: 4px;
        transition: width 1s ease-in-out;
    }

    /* Section Header Inline */
    .section-header-inline {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 1rem;
        color: var(--text-secondary);
    }

    .section-header-inline h3 {
        margin: 0;
        font-size: 0.85rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    /* Metrics Grid */
    .metrics-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
        gap: 1rem;
        margin-bottom: 2rem;
    }

    .metric-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        padding: 1rem;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        transition: border-color 0.2s;
    }

    .metric-card:hover {
        border-color: var(--color-primary);
    }

    .metric-icon {
        width: 36px;
        height: 36px;
        border-radius: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
    }

    .metric-icon.requests {
        background: rgba(59, 130, 246, 0.15);
        color: #3b82f6;
    }

    .metric-icon.response-time {
        background: rgba(16, 185, 129, 0.15);
        color: #10b981;
    }

    .metric-icon.p95 {
        background: rgba(139, 92, 246, 0.15);
        color: #8b5cf6;
    }

    .metric-icon.total {
        background: rgba(99, 102, 241, 0.15);
        color: #6366f1;
    }

    .metric-icon.errors {
        background: rgba(239, 68, 68, 0.15);
        color: #ef4444;
    }

    .metric-icon.rate-limited {
        background: rgba(245, 158, 11, 0.15);
        color: #f59e0b;
    }

    .metric-content {
        display: flex;
        flex-direction: column;
        min-width: 0;
    }

    .metric-value {
        font-size: 1.1rem;
        font-weight: 700;
        color: var(--text-primary);
    }

    .metric-label {
        font-size: 0.75rem;
        color: var(--text-secondary);
        white-space: nowrap;
    }

    /* Stats Grid */
    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2rem;
    }

    /* 2-Column Grid */
    .grid-2 {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2rem;
    }

    /* Card Styles */
    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg);
        overflow: hidden;
    }

    .card-header {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 1rem 1.5rem;
        border-bottom: 1px solid var(--border-color);
        color: var(--text-secondary);
    }

    .card-header h3 {
        margin: 0;
        font-size: 0.9rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .card-body {
        padding: 1rem 1.5rem;
        max-height: 400px;
        overflow-y: auto;
    }

    /* Mini Table */
    .mini-table {
        width: 100%;
        border-collapse: collapse;
    }

    .mini-table th {
        text-align: left;
        padding: 0.5rem 0;
        font-size: 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--text-secondary);
        border-bottom: 1px solid var(--border-color);
    }

    .mini-table td {
        padding: 0.75rem 0;
        border-bottom: 1px solid var(--border-subtle);
    }

    .mini-table code {
        background: var(--bg-app);
        padding: 0.2rem 0.5rem;
        border-radius: 4px;
        font-size: 0.85rem;
        color: var(--color-primary);
    }

    .row-count {
        font-family: monospace;
        font-weight: 600;
        color: var(--text-primary);
    }

    .text-right {
        text-align: right;
    }

    /* Activity List */
    .activity-list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .activity-item {
        display: flex;
        align-items: flex-start;
        gap: 0.75rem;
        padding: 0.75rem;
        background: var(--bg-app);
        border-radius: var(--radius-md);
    }

    .activity-dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        margin-top: 6px;
        flex-shrink: 0;
    }

    .activity-dot.primary {
        background: var(--color-primary);
    }
    .activity-dot.success {
        background: var(--color-success);
    }
    .activity-dot.warning {
        background: #f59e0b;
    }
    .activity-dot.info {
        background: #3b82f6;
    }
    .activity-dot.muted {
        background: var(--text-muted);
    }

    .activity-content {
        flex: 1;
        min-width: 0;
    }

    .activity-action {
        display: block;
        font-weight: 600;
        font-size: 0.85rem;
        color: var(--text-primary);
    }

    .activity-resource {
        display: block;
        font-size: 0.8rem;
        color: var(--text-secondary);
    }

    .activity-meta {
        text-align: right;
        font-size: 0.75rem;
        color: var(--text-muted);
    }

    .activity-user {
        display: block;
        color: var(--text-secondary);
    }

    .activity-time {
        display: block;
    }

    .empty-mini {
        padding: 2rem;
        text-align: center;
        color: var(--text-secondary);
    }

    /* Loading State */
    .loading-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 4rem;
        color: var(--text-secondary);
        gap: 1rem;
    }

    .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    /* Error Card */
    .error-card {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;
        padding: 3rem;
        background: var(--bg-surface);
        border: 1px solid var(--color-danger);
        border-radius: var(--radius-lg);
        text-align: center;
        color: var(--color-danger);
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
        filter: brightness(1.1);
    }

    /* Last Updated */
    .last-updated {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        padding: 1rem;
        font-size: 0.85rem;
        color: var(--text-muted);
    }

    /* Responsive */
    @media (max-width: 768px) {
        .page-container {
            padding: 1rem;
        }

        .status-banner {
            flex-direction: column;
            text-align: center;
            gap: 1rem;
        }

        .status-meta {
            align-items: center;
        }

        .grid-2 {
            grid-template-columns: 1fr;
        }
    }
</style>

