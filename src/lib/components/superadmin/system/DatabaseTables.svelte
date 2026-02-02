<script lang="ts">
    import Icon from "$lib/components/ui/Icon.svelte";
    import { t } from "svelte-i18n";

    let { health } = $props<{
        health: any;
    }>();
</script>

<div class="card">
    <div class="card-header">
        <Icon name="database" size={20} />
        <h3>
            {$t("superadmin.system.database_tables") || "Database Tables"}
        </h3>
    </div>
    <div class="card-body">
        <table class="mini-table">
            <thead>
                <tr>
                    <th>
                        {$t("superadmin.system.table_headers.table") || "Table"}
                    </th>
                    <th class="text-right">
                        {$t("superadmin.system.table_headers.rows") || "Rows"}
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
</style>
