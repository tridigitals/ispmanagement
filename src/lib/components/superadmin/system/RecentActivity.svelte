<script lang="ts">
    import Icon from "$lib/components/ui/Icon.svelte";
    import { fade } from "svelte/transition";
    import { t } from "svelte-i18n";

    let { health } = $props<{
        health: any;
    }>();

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

    function formatTime(isoString: string): string {
        return new Date(isoString).toLocaleTimeString();
    }
</script>

<div class="card">
    <div class="card-header">
        <Icon name="clock" size={20} />
        <h3>
            {$t("superadmin.system.recent_activity") || "Recent Activity"}
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
                                >{formatTime(activity.created_at)}</span
                            >
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
</div>

<style>
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
</style>
