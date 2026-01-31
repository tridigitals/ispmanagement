<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { isSuperAdmin } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import TableToolbar from "$lib/components/TableToolbar.svelte";
    import Pagination from "$lib/components/Pagination.svelte";
    import type { AuditLog } from "$lib/api/client";

    let logs = $state<AuditLog[]>([]);
    let loading = $state(true);
    let page = $state(1);
    let total = $state(0);
    let pageSize = $state(20);
    let isMobile = $state(false);
    let viewMode = $state<"table" | "cards">("table");
    let expandedLogId = $state<string | null>(null);

    // Filters
    let searchQuery = $state("");
    let actionFilter = $state("");
    let dateFrom = $state("");
    let dateTo = $state("");
    let userIdFilter = $state("");
    let tenantIdFilter = $state("");

    // Debounced reload (search + filters)
    let searchTimer: any;
    function handleSearch() {
        clearTimeout(searchTimer);
        searchTimer = setTimeout(() => {
            page = 1;
            loadLogs();
        }, 500);
    }

    function setQuickRange(days: number) {
        const now = new Date();
        const from = new Date(now.getTime() - days * 24 * 60 * 60 * 1000);

        const toLocal = (d: Date) => {
            const pad = (n: number) => String(n).padStart(2, "0");
            return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
        };

        dateFrom = toLocal(from);
        dateTo = toLocal(now);
        handleSearch();
    }

    function clearFilters() {
        searchQuery = "";
        actionFilter = "";
        dateFrom = "";
        dateTo = "";
        userIdFilter = "";
        tenantIdFilter = "";
        page = 1;
        void loadLogs();
    }

    async function loadLogs() {
        if (!$isSuperAdmin) return;

        loading = true;
        try {
            // Prepare filters - remove empty strings
            const activeFilters: any = {};
            if (searchQuery) activeFilters.search = searchQuery;
            if (actionFilter) activeFilters.action = actionFilter;
            if (dateFrom)
                activeFilters.date_from = new Date(
                    dateFrom,
                ).toISOString();
            if (dateTo) activeFilters.date_to = new Date(dateTo).toISOString();
            if (userIdFilter) activeFilters.user_id = userIdFilter;
            // if (tenantIdFilter) activeFilters.tenant_id = tenantIdFilter;

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

    onMount(() => {
        let cleanup: (() => void) | undefined;

        if (!$isSuperAdmin) {
            goto("/dashboard");
            return;
        }

        if (typeof window !== "undefined") {
            const mq = window.matchMedia("(max-width: 720px)");
            const sync = () => (isMobile = mq.matches);
            sync();
            try {
                mq.addEventListener("change", sync);
                cleanup = () => mq.removeEventListener("change", sync);
            } catch {
                // @ts-ignore
                mq.addListener?.(sync);
                // @ts-ignore
                cleanup = () => mq.removeListener?.(sync);
            }
        }

        void loadLogs();
        return cleanup;
    });

    $effect(() => {
        if (isMobile) viewMode = "cards";
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

    function getActionCategory(action: string) {
        const head = String(action || "").split("_")[0].toLowerCase();
        if (["auth", "login", "logout", "2fa"].includes(head)) return "auth";
        if (["user", "users"].includes(head)) return "user";
        if (["tenant", "tenants"].includes(head)) return "tenant";
        if (["setting", "settings"].includes(head)) return "settings";
        return "other";
    }

    function toggleExpand(id: string) {
        expandedLogId = expandedLogId === id ? null : id;
    }
</script>

<div class="superadmin-content fade-in">
    <div class="glass-card">
        <div class="card-header glass">
            <div>
                <h3>Audit Logs</h3>
                <span class="muted">Track system activity and security events</span>
            </div>
            <div class="header-actions">
                <span class="count-badge">{total} logs</span>
                <button
                    class="btn-icon"
                    onclick={() => loadLogs()}
                    title="Refresh logs"
                    aria-label="Refresh logs"
                    type="button"
                >
                    <Icon name="refresh-cw" size={18} />
                </button>
            </div>
        </div>

        <div class="toolbar-wrapper">
            <TableToolbar bind:searchQuery={searchQuery} placeholder="Search logs..." onsearch={handleSearch}>
                {#snippet filters()}
                    <div class="filters-row">
                        <div class="field">
                            <label class="field-label" for="filter-action">Action (exact)</label>
                            <input
                                id="filter-action"
                                type="text"
                                bind:value={actionFilter}
                                oninput={handleSearch}
                                placeholder="e.g. login, create_user"
                                class="field-input"
                            />
                        </div>

                        <div class="field">
                            <label class="field-label" for="filter-date-from">From</label>
                            <input
                                id="filter-date-from"
                                type="datetime-local"
                                bind:value={dateFrom}
                                onchange={handleSearch}
                                class="field-input"
                            />
                        </div>

                        <div class="field">
                            <label class="field-label" for="filter-date-to">To</label>
                            <input
                                id="filter-date-to"
                                type="datetime-local"
                                bind:value={dateTo}
                                onchange={handleSearch}
                                class="field-input"
                            />
                        </div>

                        <div class="quick-row" aria-label="Quick ranges">
                            <button type="button" class="chip" onclick={() => setQuickRange(1)}>24h</button>
                            <button type="button" class="chip" onclick={() => setQuickRange(7)}>7d</button>
                            <button type="button" class="chip" onclick={() => setQuickRange(30)}>30d</button>
                            <button type="button" class="chip danger" onclick={clearFilters}>Clear</button>
                        </div>
                    </div>
                {/snippet}

                {#snippet actions()}
                    {#if !isMobile}
                        <div class="view-toggle" aria-label="View mode">
                            <button
                                type="button"
                                class="view-btn"
                                class:active={viewMode === "table"}
                                onclick={() => (viewMode = "table")}
                                title="Table view"
                                aria-label="Table view"
                            >
                                <Icon name="list" size={18} />
                            </button>
                            <button
                                type="button"
                                class="view-btn"
                                class:active={viewMode === "cards"}
                                onclick={() => (viewMode = "cards")}
                                title="Card view"
                                aria-label="Card view"
                            >
                                <Icon name="grid" size={18} />
                            </button>
                        </div>
                    {/if}
                {/snippet}
            </TableToolbar>
        </div>

        {#if viewMode === "cards" || isMobile}
            <div class="cards-wrapper">
                {#if loading && logs.length === 0}
                    <div class="loading-state">
                        <div class="spinner"></div>
                        <p>Loading logs...</p>
                    </div>
                {:else if logs.length === 0}
                    <div class="empty-state">
                        <Icon name="activity" size={48} />
                        <h4>No logs found</h4>
                        <p>Try adjusting your filters.</p>
                    </div>
                {:else}
                    <div class="log-cards" aria-label="Audit logs">
                        {#each logs as l (l.id)}
                            <div class="log-card">
                                <div class="log-top">
                                    <div class="log-left">
                                        <div class="log-time">
                                            {new Date(l.created_at).toLocaleString()}
                                        </div>
                                        <span class="action-pill {getActionCategory(l.action)}">
                                            {l.action}
                                        </span>
                                    </div>
                                    <button
                                        type="button"
                                        class="btn-icon"
                                        onclick={() => toggleExpand(l.id)}
                                        aria-label={expandedLogId === l.id ? "Collapse details" : "Expand details"}
                                        title={expandedLogId === l.id ? "Collapse" : "Expand"}
                                    >
                                        <Icon name={expandedLogId === l.id ? "chevron-up" : "chevron-down"} size={18} />
                                    </button>
                                </div>

                                <div class="log-grid">
                                    <div class="kv">
                                        <span class="k">User</span>
                                        <span class="v">
                                            {#if l.user_email}
                                                {l.user_name ? `${l.user_name} — ${l.user_email}` : l.user_email}
                                            {:else if l.user_id}
                                                <span class="text-mono">{l.user_id.substring(0, 8)}…</span>
                                            {:else}
                                                —
                                            {/if}
                                        </span>
                                    </div>

                                    <div class="kv">
                                        <span class="k">Tenant</span>
                                        <span class="v">
                                            {#if l.tenant_name}
                                                {l.tenant_name}
                                            {:else if l.tenant_id}
                                                <span class="text-mono">{l.tenant_id.substring(0, 8)}…</span>
                                            {:else}
                                                <span class="badge-global">Global</span>
                                            {/if}
                                        </span>
                                    </div>

                                    <div class="kv">
                                        <span class="k">Resource</span>
                                        <span class="v">
                                            {l.resource}
                                            {#if l.resource_name}
                                                <span class="sub">{l.resource_name}</span>
                                            {:else if l.resource_id}
                                                <span class="sub text-mono">{l.resource_id.substring(0, 8)}…</span>
                                            {/if}
                                        </span>
                                    </div>

                                    <div class="kv">
                                        <span class="k">IP</span>
                                        <span class="v text-mono">{l.ip_address || "—"}</span>
                                    </div>
                                </div>

                                {#if expandedLogId === l.id}
                                    <div class="details-block">
                                        <div class="details-title">Details</div>
                                        <div class="details-text">{l.details || "—"}</div>
                                    </div>
                                {/if}
                            </div>
                        {/each}
                    </div>

                    <div class="cards-pagination">
                        <Pagination
                            count={total}
                            page={page - 1}
                            pageSize={pageSize}
                            onchange={(p: number) => handlePageChange(p)}
                            onpageSizeChange={(s: number) => handlePageSizeChange(s)}
                        />
                    </div>
                {/if}
            </div>
        {:else}
            <div class="table-wrapper">
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
                            <span class="text-secondary">
                                {new Date(item.created_at).toLocaleString()}
                            </span>
                        {:else if key === "action"}
                            <span class="action-pill {getActionCategory(item.action)}">
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
                                <span class="text-mono text-xs text-muted"
                                    >{item.user_id.substring(0, 8)}…</span
                                >
                            {:else}
                                <span class="text-muted">—</span>
                            {/if}
                        {:else if key === "tenant"}
                            {#if item.tenant_name}
                                <span class="font-medium"
                                    >{item.tenant_name}</span
                                >
                            {:else if item.tenant_id}
                                <span class="text-mono text-xs"
                                    >{item.tenant_id.substring(0, 8)}…</span
                                >
                            {:else}
                                <span class="badge-global">Global</span>
                            {/if}
                        {:else if key === "resource"}
                            <span class="font-medium block"
                                >{item.resource}</span
                            >
                            {#if item.resource_name}
                                <span class="text-xs text-muted block"
                                    >{item.resource_name}</span
                                >
                            {:else if item.resource_id}
                                <span class="text-xs text-muted block text-mono"
                                    >{item.resource_id.substring(0, 8)}…</span
                                >
                            {/if}
                        {:else if key === "details"}
                            <div class="details-cell" title={item.details}>
                                {item.details || "—"}
                            </div>
                        {:else if key === "ip"}
                            <span class="text-xs text-mono text-muted"
                                >{item.ip_address || "—"}</span
                            >
                        {/if}
                    {/snippet}
                </Table>
            </div>
        {/if}
    </div>
</div>

<style>
    .glass-card {
        background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
        border-radius: 16px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.28);
        overflow: hidden;
    }

    :global([data-theme="light"]) .glass-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 12px 32px rgba(0, 0, 0, 0.08),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }

    .card-header {
        padding: 1.1rem 1.25rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.08);
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: rgba(255, 255, 255, 0.015);
    }

    :global([data-theme="light"]) .card-header {
        border-bottom-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.01);
    }

    .card-header h3 {
        margin: 0;
        font-size: 1.1rem;
        font-weight: 800;
        color: var(--text-primary);
        letter-spacing: -0.01em;
    }

    .muted {
        display: block;
        margin-top: 0.25rem;
        color: var(--text-secondary);
        font-size: 0.92rem;
    }

    .header-actions {
        display: inline-flex;
        gap: 0.5rem;
        align-items: center;
    }

    .count-badge {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-primary);
        padding: 0.35rem 0.75rem;
        border-radius: 999px;
        font-size: 0.85rem;
        font-weight: 650;
        white-space: nowrap;
    }

    :global([data-theme="light"]) .count-badge {
        background: rgba(0, 0, 0, 0.03);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .btn-icon {
        width: 40px;
        height: 40px;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.02);
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
    }

    :global([data-theme="light"]) .btn-icon {
        border-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.02);
    }

    .btn-icon:hover {
        background: rgba(99, 102, 241, 0.12);
        color: var(--text-primary);
        border-color: rgba(99, 102, 241, 0.35);
    }

    .toolbar-wrapper {
        padding: 1rem 1.25rem 0.25rem 1.25rem;
    }

    .filters-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
        align-items: flex-end;
        width: 100%;
    }

    .field {
        display: flex;
        flex-direction: column;
        gap: 0.35rem;
        min-width: 220px;
    }

    .field-label {
        font-size: 0.78rem;
        font-weight: 700;
        color: var(--text-secondary);
    }

    .field-input {
        width: 100%;
        padding: 0.6rem 0.8rem;
        border: 1px solid var(--border-color);
        border-radius: 12px;
        background: var(--bg-surface);
        color: var(--text-primary);
        font-size: 0.9rem;
        transition: border-color 0.2s;
    }

    .field-input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    }

    .quick-row {
        display: inline-flex;
        flex-wrap: wrap;
        gap: 0.5rem;
        align-items: center;
        min-width: 0;
        padding-bottom: 2px;
    }

    .chip {
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.03);
        color: var(--text-secondary);
        padding: 0.45rem 0.7rem;
        border-radius: 999px;
        cursor: pointer;
        font-weight: 650;
        font-size: 0.82rem;
        transition: all 0.2s;
        white-space: nowrap;
    }

    :global([data-theme="light"]) .chip {
        border-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.02);
    }

    .chip:hover {
        background: rgba(99, 102, 241, 0.12);
        color: var(--text-primary);
        border-color: rgba(99, 102, 241, 0.35);
    }

    .chip.danger:hover {
        background: rgba(239, 68, 68, 0.12);
        border-color: rgba(239, 68, 68, 0.28);
    }

    .view-toggle {
        display: inline-flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.25rem;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.03);
    }

    :global([data-theme="light"]) .view-toggle {
        border-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.02);
    }

    .view-btn {
        width: 38px;
        height: 38px;
        border-radius: 10px;
        border: none;
        background: transparent;
        color: var(--text-secondary);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
        padding: 0;
    }

    .view-btn:hover {
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-primary);
    }

    :global([data-theme="light"]) .view-btn:hover {
        background: rgba(0, 0, 0, 0.04);
    }

    .view-btn.active {
        background: rgba(99, 102, 241, 0.18);
        border: 1px solid rgba(99, 102, 241, 0.25);
        color: var(--text-primary);
    }

    .table-wrapper {
        padding: 0 1.25rem 1rem 1.25rem;
    }

    .cards-wrapper {
        padding: 0 1.25rem 1rem 1.25rem;
    }

    /* Cell Styles */
    .user-info {
        display: flex;
        flex-direction: column;
        line-height: 1.2;
    }

    .user-name {
        font-weight: 650;
        color: var(--text-primary);
        font-size: 0.9rem;
    }

    .user-email {
        font-size: 0.8rem;
        color: var(--text-secondary);
    }

    .action-pill {
        display: inline-block;
        padding: 0.2rem 0.6rem;
        border-radius: 999px;
        font-size: 0.75rem;
        font-weight: 650;
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-primary);
        white-space: nowrap;
    }

    :global([data-theme="light"]) .action-pill {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .action-pill.user {
        background: rgba(59, 130, 246, 0.1);
        border-color: rgba(59, 130, 246, 0.18);
        color: #60a5fa;
    }
    .action-pill.tenant {
        background: rgba(16, 185, 129, 0.1);
        border-color: rgba(16, 185, 129, 0.18);
        color: #34d399;
    }
    .action-pill.auth {
        background: rgba(245, 158, 11, 0.1);
        border-color: rgba(245, 158, 11, 0.18);
        color: #fbbf24;
    }
    .action-pill.settings {
        background: rgba(99, 102, 241, 0.1);
        border-color: rgba(99, 102, 241, 0.18);
        color: #818cf8;
    }
    .action-pill.other {
        background: rgba(255, 255, 255, 0.03);
        color: var(--text-secondary);
    }

    .badge-global {
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-secondary);
        padding: 0.1rem 0.4rem;
        border-radius: 4px;
        font-size: 0.7rem;
        font-weight: 600;
        text-transform: uppercase;
    }

    :global([data-theme="light"]) .badge-global {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .details-cell {
        max-width: 300px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 0.9rem;
        color: var(--text-secondary);
    }

    .text-muted {
        color: var(--text-muted);
    }

    .text-secondary {
        color: var(--text-secondary);
    }

    .text-xs {
        font-size: 0.75rem;
    }

    .text-mono {
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .font-medium {
        font-weight: 650;
        color: var(--text-primary);
    }

    .block {
        display: block;
    }

    /* Cards */
    .log-cards {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 0.9rem;
    }

    .log-card {
        background: rgba(255, 255, 255, 0.02);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 16px;
        padding: 1rem;
    }

    :global([data-theme="light"]) .log-card {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .log-top {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 0.75rem;
        margin-bottom: 0.75rem;
    }

    .log-left {
        display: flex;
        flex-direction: column;
        gap: 0.35rem;
        min-width: 0;
    }

    .log-time {
        color: var(--text-secondary);
        font-size: 0.85rem;
        font-weight: 650;
    }

    .log-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 0.75rem 0.9rem;
    }

    .kv {
        display: flex;
        flex-direction: column;
        gap: 0.2rem;
        min-width: 0;
    }

    .k {
        font-size: 0.78rem;
        color: var(--text-secondary);
        font-weight: 700;
    }

    .v {
        font-size: 0.9rem;
        color: var(--text-primary);
        font-weight: 600;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .sub {
        display: block;
        margin-top: 0.15rem;
        color: var(--text-secondary);
        font-weight: 500;
        font-size: 0.82rem;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .details-block {
        margin-top: 0.85rem;
        padding-top: 0.85rem;
        border-top: 1px solid rgba(255, 255, 255, 0.08);
    }

    :global([data-theme="light"]) .details-block {
        border-top-color: rgba(0, 0, 0, 0.06);
    }

    .details-title {
        font-size: 0.78rem;
        color: var(--text-secondary);
        font-weight: 800;
        margin-bottom: 0.35rem;
    }

    .details-text {
        color: var(--text-primary);
        font-size: 0.9rem;
        line-height: 1.45;
        white-space: pre-wrap;
        word-break: break-word;
    }

    .cards-pagination {
        margin-top: 0.75rem;
    }

    .loading-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 2.5rem 1rem;
        color: var(--text-secondary);
    }

    .spinner {
        width: 34px;
        height: 34px;
        border-radius: 50%;
        border: 3px solid rgba(255, 255, 255, 0.12);
        border-top-color: rgba(99, 102, 241, 0.9);
        animation: spin 0.9s linear infinite;
        margin-bottom: 0.75rem;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .empty-state {
        padding: 2.25rem 1rem;
        text-align: center;
        color: var(--text-secondary);
    }

    .empty-state h4 {
        margin: 0.65rem 0 0.25rem 0;
        color: var(--text-primary);
    }

    @media (max-width: 768px) {
        .toolbar-wrapper {
            padding: 0.9rem 1rem 0 1rem;
        }

        .table-wrapper,
        .cards-wrapper {
            padding: 0 1rem 1rem 1rem;
        }

        .filters-row {
            align-items: stretch;
        }

        .field {
            min-width: 0;
            width: 100%;
        }

        .log-cards {
            grid-template-columns: 1fr;
        }

        .log-grid {
            grid-template-columns: 1fr;
        }
    }
</style>
