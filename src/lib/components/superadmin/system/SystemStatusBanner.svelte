<script lang="ts">
    import Icon from "$lib/components/ui/Icon.svelte";
    import { t } from "svelte-i18n";

    let { health } = $props<{
        health: any;
    }>();

    function formatUptime(seconds: number): string {
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);

        if (days > 0) return `${days}d ${hours}h ${minutes}m`;
        if (hours > 0) return `${hours}h ${minutes}m`;
        return `${minutes}m`;
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

<div
    class="status-banner"
    class:connected={health.database.is_connected}
    class:disconnected={!health.database.is_connected}
>
    <div class="status-icon">
        <Icon
            name={health.database.is_connected ? "check-circle" : "x-circle"}
            size={24}
        />
    </div>
    <div class="status-info">
        <h3>
            {health.database.is_connected
                ? $t("superadmin.system.db_connected") || "Database Connected"
                : $t("superadmin.system.db_disconnected") ||
                  "Database Disconnected"}
        </h3>
        <p>
            {health.database.database_type} • {health.database.total_tables}
            {$t("superadmin.system.tables") || "tables"} • {formatBytes(
                health.database.database_size_bytes,
            )}
        </p>
    </div>
    <div class="status-meta">
        <span class="version-badge">v{health.app_version}</span>
        <span class="os-badge"
            >{health.resources.os_name} {health.resources.os_version}</span
        >
        <span class="uptime"
            >{$t("superadmin.system.uptime") || "Uptime:"}
            {formatUptime(health.uptime_seconds)}</span
        >
    </div>
</div>

<style>
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
</style>
