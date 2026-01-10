<script lang="ts">
    import Icon from "./Icon.svelte";

    export let title: string;
    export let value: string | number;
    export let icon: string;
    export let color = "primary"; // primary, success, warning, danger
    export let trend: string | null = null; // "+5%", "-2%"
    export let trendType: "up" | "down" | "neutral" = "neutral";

    function getColorVar(color: string) {
        switch (color) {
            case "success": return "var(--color-success, #10b981)";
            case "warning": return "var(--color-warning, #f59e0b)";
            case "danger": return "var(--color-danger, #ef4444)";
            default: return "var(--color-primary, #6366f1)";
        }
    }
</script>

<div class="stats-card">
    <div class="icon-wrapper" style="background: {getColorVar(color)}20; color: {getColorVar(color)}">
        <Icon name={icon} size={24} />
    </div>
    <div class="content">
        <div class="title">{title}</div>
        <div class="value">{value}</div>
        {#if trend}
            <div class="trend {trendType}">
                <Icon name={trendType === 'up' ? 'trending-up' : trendType === 'down' ? 'trending-down' : 'minus'} size={14} />
                <span>{trend}</span>
            </div>
        {/if}
    </div>
</div>

<style>
    .stats-card {
        background: var(--bg-surface, #1e293b);
        border: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 16px;
        padding: 1.5rem;
        display: flex;
        align-items: flex-start;
        gap: 1rem;
        transition: transform 0.2s, box-shadow 0.2s;
    }

    .stats-card:hover {
        transform: translateY(-2px);
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
    }

    .icon-wrapper {
        width: 48px;
        height: 48px;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .content {
        flex: 1;
    }

    .title {
        color: var(--text-secondary, #94a3b8);
        font-size: 0.9rem;
        margin-bottom: 0.25rem;
    }

    .value {
        color: var(--text-primary, #fff);
        font-size: 1.5rem;
        font-weight: 700;
        line-height: 1.2;
    }

    .trend {
        display: flex;
        align-items: center;
        gap: 0.25rem;
        font-size: 0.8rem;
        margin-top: 0.5rem;
        font-weight: 600;
    }

    .trend.up { color: var(--color-success, #10b981); }
    .trend.down { color: var(--color-danger, #ef4444); }
    .trend.neutral { color: var(--text-secondary, #94a3b8); }
</style>
