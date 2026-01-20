<script lang="ts">
    import { onMount, tick } from "svelte";
    import { api } from "$lib/api/client";
    import { isSuperAdmin } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import type { AuditLog } from "$lib/api/client";

    let logs: AuditLog[] = [];
    let loading = true;
    let page = 1;
    let total = 0;
    let pageSize = 20;

    // Filters
    let filters = {
        search: "",
        action: "",
        date_from: "",
        date_to: "",
        user_id: "",
        tenant_id: "",
    };

    // Debounce search
    let searchTimer: any;
    function handleSearch() {
        clearTimeout(searchTimer);
        searchTimer = setTimeout(() => {
            page = 1;
            loadLogs();
        }, 500);
    }

    async function loadLogs() {
        if (!$isSuperAdmin) return;

        loading = true;
        try {
            // Prepare filters - remove empty strings
            const activeFilters: any = {};
            if (filters.search) activeFilters.search = filters.search;
            if (filters.action) activeFilters.action = filters.action;
            if (filters.date_from)
                activeFilters.date_from = new Date(
                    filters.date_from,
                ).toISOString();
            if (filters.date_to)
                activeFilters.date_to = new Date(filters.date_to).toISOString();
            if (filters.user_id) activeFilters.user_id = filters.user_id;
            // if (filters.tenant_id) activeFilters.tenant_id = filters.tenant_id;

            const res = await api.superadmin.listAuditLogs(
                page,
                pageSize,
                activeFilters,
            );
            logs = res.data;
            total = res.total;
        } catch (err) {
            console.error("Failed to load audit logs:", err);
        } finally {
            loading = false;
        }
    }

    onMount(async () => {
        if (!$isSuperAdmin) {
            goto("/dashboard");
            return;
        }
        await loadLogs();
    });

    function handlePageChange(newPage: number) {
        page = newPage + 1;
        loadLogs();
    }

    function handlePageSizeChange(newSize: number) {
        pageSize = newSize;
        page = 1;
        loadLogs();
    }

    // Columns config
    const columns = [
        { key: "created_at", label: "Time", width: "180px" },
        { key: "action", label: "Action", width: "150px" },
        { key: "user", label: "User" },
        { key: "tenant", label: "Tenant" },
        { key: "resource", label: "Resource" },
        { key: "details", label: "Details" },
        { key: "ip", label: "IP", width: "120px" },
    ];
</script>

<div class="page-header">
    <div class="header-left">
        <h1>Audit Logs</h1>
        <p class="subtitle">Track system activity and security events</p>
    </div>
    <div class="header-right">
        <button class="btn-refresh" onclick={loadLogs} title="Refresh Logs">
            <Icon name="refresh-cw" size={18} />
        </button>
    </div>
</div>

<!-- Filters -->
<div class="filter-card">
    <div class="filter-group">
        <label class="filter-label" for="filter-search">Search</label>
        <div class="input-wrapper">
            <Icon name="search" size={16} class="input-icon" />
            <input
                id="filter-search"
                type="text"
                bind:value={filters.search}
                oninput={handleSearch}
                placeholder="Search resources, details..."
                class="filter-input"
            />
        </div>
    </div>

    <div class="filter-group">
        <label class="filter-label" for="filter-action">Action</label>
        <input
            id="filter-action"
            type="text"
            bind:value={filters.action}
            oninput={handleSearch}
            placeholder="e.g. login, create_user"
            class="filter-input"
        />
    </div>

    <div class="filter-group">
        <label class="filter-label" for="filter-date-from">From Date</label>
        <input
            id="filter-date-from"
            type="datetime-local"
            bind:value={filters.date_from}
            onchange={handleSearch}
            class="filter-input"
        />
    </div>

    <div class="filter-group">
        <label class="filter-label" for="filter-date-to">To Date</label>
        <input
            id="filter-date-to"
            type="datetime-local"
            bind:value={filters.date_to}
            onchange={handleSearch}
            class="filter-input"
        />
    </div>
</div>

<div class="content-card">
    <Table
        {columns}
        data={logs}
        {loading}
        pagination={true}
        {pageSize}
        count={total}
        onchange={handlePageChange}
        onpageSizeChange={handlePageSizeChange}
        serverSide={true}
    >
        {#snippet cell({ item, key })}
            {#if key === "created_at"}
                <span class="text-sm text-gray-500">
                    {new Date(item.created_at).toLocaleString()}
                </span>
            {:else if key === "action"}
                <span
                    class="action-pill {item.action
                        .split('_')[0]
                        .toLowerCase()}"
                >
                    {item.action}
                </span>
            {:else if key === "user"}
                {#if item.user_email}
                    <div class="user-info">
                        {#if item.user_name}
                            <span class="user-name">{item.user_name}</span>
                        {/if}
                        <span class="user-email">{item.user_email}</span>
                    </div>
                {:else if item.user_id}
                    <span class="font-mono text-xs text-muted"
                        >{item.user_id.substring(0, 8)}...</span
                    >
                {:else}
                    <span class="text-muted">—</span>
                {/if}
            {:else if key === "tenant"}
                {#if item.tenant_name}
                    <span class="font-medium text-gray-700"
                        >{item.tenant_name}</span
                    >
                {:else if item.tenant_id}
                    <span class="font-mono text-xs"
                        >{item.tenant_id.substring(0, 8)}...</span
                    >
                {:else}
                    <span class="badge-global">Global</span>
                {/if}
            {:else if key === "resource"}
                <span class="font-medium text-gray-700 block"
                    >{item.resource}</span
                >
                {#if item.resource_name}
                    <span class="text-xs text-muted block"
                        >{item.resource_name}</span
                    >
                {:else if item.resource_id}
                    <span class="text-xs text-muted block font-mono"
                        >{item.resource_id.substring(0, 8)}...</span
                    >
                {/if}
            {:else if key === "details"}
                <div class="details-cell" title={item.details}>
                    {item.details || "—"}
                </div>
            {:else if key === "ip"}
                <span class="text-xs font-mono text-gray-500"
                    >{item.ip_address || "—"}</span
                >
            {/if}
        {/snippet}
    </Table>
</div>

<style>
    .page-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 1.5rem;
    }

    h1 {
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

    .filter-card {
        background: var(--bg-surface);
        border-radius: var(--radius-lg);
        border: 1px solid var(--border-color);
        padding: 1.5rem;
        margin-bottom: 1.5rem;
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 1.5rem;
    }

    .filter-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .filter-label {
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--text-secondary);
    }

    .input-wrapper {
        position: relative;
        display: flex;
        align-items: center;
    }

    .input-icon {
        position: absolute;
        left: 10px;
        color: var(--text-muted);
        pointer-events: none;
    }

    .filter-input {
        width: 100%;
        padding: 0.6rem 0.8rem;
        padding-left: 2rem; /* space for icon if present */
        border: 1px solid var(--border-color);
        border-radius: var(--radius-sm);
        background: var(--bg-app);
        color: var(--text-primary);
        font-size: 0.9rem;
        transition: border-color 0.2s;
    }

    .filter-group input[type="datetime-local"].filter-input,
    .filter-group input:not([type="text"]).filter-input {
        padding-left: 0.8rem;
    }

    .filter-input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 2px var(--color-primary-subtle);
    }

    .content-card {
        background: var(--bg-surface);
        border-radius: var(--radius-lg);
        border: 1px solid var(--border-color);
        box-shadow: var(--shadow-sm);
        overflow: hidden;
        padding: 0; /* Table handles padding */
    }

    /* Cell Styles */
    .user-info {
        display: flex;
        flex-direction: column;
        line-height: 1.2;
    }

    .user-name {
        font-weight: 500;
        color: var(--text-primary);
        font-size: 0.9rem;
    }

    .user-email {
        font-size: 0.75rem;
        color: var(--text-secondary);
    }

    .action-pill {
        display: inline-block;
        padding: 0.2rem 0.6rem;
        border-radius: 4px;
        font-size: 0.75rem;
        font-weight: 600;
        background: var(--bg-active);
        color: var(--text-secondary);
        white-space: nowrap;
    }

    .action-pill.user {
        background: rgba(59, 130, 246, 0.1);
        color: #3b82f6;
    }
    .action-pill.tenant {
        background: rgba(16, 185, 129, 0.1);
        color: #10b981;
    }
    .action-pill.auth {
        background: rgba(245, 158, 11, 0.1);
        color: #f59e0b;
    }
    .action-pill.settings {
        background: rgba(99, 102, 241, 0.1);
        color: #6366f1;
    }

    .badge-global {
        background: #f1f5f9;
        color: #64748b;
        padding: 0.1rem 0.4rem;
        border-radius: 4px;
        font-size: 0.7rem;
        font-weight: 500;
        text-transform: uppercase;
    }

    .details-cell {
        max-width: 300px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 0.9rem;
    }

    .text-muted {
        color: var(--text-muted);
    }
    .text-xs {
        font-size: 0.75rem;
    }
    .text-sm {
        font-size: 0.875rem;
    }
    .font-mono {
        font-family: monospace;
    }
    .font-medium {
        font-weight: 500;
    }
    .block {
        display: block;
    }
</style>
