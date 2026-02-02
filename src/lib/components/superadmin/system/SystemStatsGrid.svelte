<script lang="ts">
    import StatsCard from "$lib/components/dashboard/StatsCard.svelte";
    import { t } from "svelte-i18n";

    let { health } = $props<{
        health: any;
    }>();
</script>

<div class="stats-grid">
    <StatsCard
        title={$t("superadmin.system.stats.total_tenants") || "Total Tenants"}
        value={health.database.tenants_count}
        icon="building"
        color="primary"
    />
    <StatsCard
        title={$t("superadmin.system.stats.total_users") || "Total Users"}
        value={health.database.users_count}
        icon="users"
        color="info"
    />
    <StatsCard
        title={$t("superadmin.system.stats.active_users") || "Active Users"}
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

<style>
    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2rem;
    }
</style>
