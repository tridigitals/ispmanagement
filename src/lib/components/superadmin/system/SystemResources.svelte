<script lang="ts">
    import Icon from "$lib/components/ui/Icon.svelte";
    import { t } from "svelte-i18n";

    let { health } = $props<{
        health: any;
    }>();

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
            <h3>{$t("superadmin.system.memory_usage") || "Memory Usage"}</h3>
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

<style>
    .grid-2 {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2rem;
    }

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
</style>
