<script lang="ts">
    import { onMount } from "svelte";
    import { api, type Invoice } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import TableToolbar from "$lib/components/TableToolbar.svelte";
    import { toast } from "$lib/stores/toast";
    import { formatMoney } from "$lib/utils/money";
    import { goto } from "$app/navigation";
    import { getTenantsCached } from "$lib/stores/superadminTenantsCache";
    import { t } from "svelte-i18n";
    import { get } from "svelte/store";

    type InvoiceStatus =
        | "all"
        | "pending"
        | "paid"
        | "failed"
        | "verification_pending"
        | "expired";

    let invoices = $state<Invoice[]>([]);
    let loading = $state(true);
    let error = $state("");

    let tenantNameById = $state<Record<string, { name: string; slug?: string }>>(
        {},
    );

    let search = $state("");
    let statusFilter = $state<InvoiceStatus>("all");
    let viewMode = $state<"cards" | "table">("cards");
    let isMobile = $state(false);

    const columns = $derived.by(() => [
        {
            key: "invoice_number",
            label: $t("superadmin.invoices.list.columns.invoice_number") || "Invoice #",
            sortable: true,
        },
        {
            key: "tenant",
            label: $t("superadmin.invoices.list.columns.tenant") || "Tenant",
            sortable: true,
        },
        {
            key: "amount",
            label: $t("superadmin.invoices.list.columns.amount") || "Amount",
            sortable: true,
        },
        {
            key: "status",
            label: $t("superadmin.invoices.list.columns.status") || "Status",
            sortable: true,
        },
        {
            key: "due_date",
            label: $t("superadmin.invoices.list.columns.due_date") || "Due Date",
            sortable: true,
        },
        {
            key: "created_at",
            label: $t("superadmin.invoices.list.columns.created_at") || "Created At",
            sortable: true,
        },
        {
            key: "actions",
            label: $t("superadmin.invoices.list.columns.actions") || "Actions",
            align: "right",
        },
    ]);

    const stats = $derived({
        total: invoices.length,
        pending: invoices.filter(
            (i) => i.status === "pending" || i.status === "verification_pending",
        ).length,
        paid: invoices.filter((i) => i.status === "paid").length,
        failed: invoices.filter((i) => i.status === "failed").length,
    });

    const filteredInvoices = $derived(
        invoices.filter((inv) => {
            const q = search.trim().toLowerCase();
            const tenant =
                inv.tenant_id && tenantNameById[inv.tenant_id]
                    ? tenantNameById[inv.tenant_id].name
                    : "";
            const matchesSearch =
                !q ||
                inv.invoice_number.toLowerCase().includes(q) ||
                (inv.tenant_id ?? "").toLowerCase().includes(q) ||
                tenant.toLowerCase().includes(q) ||
                (inv.status ?? "").toLowerCase().includes(q);

            const matchesStatus =
                statusFilter === "all" || inv.status === statusFilter;

            return matchesSearch && matchesStatus;
        }),
    );

    onMount(() => {
        let cleanup: (() => void) | undefined;

        if (typeof window !== "undefined") {
            const mq = window.matchMedia("(max-width: 720px)");
            const sync = () => {
                isMobile = mq.matches;
            };
            sync();

            try {
                mq.addEventListener("change", sync);
                cleanup = () => mq.removeEventListener("change", sync);
            } catch {
                // Safari/older WebView fallback
                // @ts-ignore
                mq.addListener?.(sync);
                // @ts-ignore
                cleanup = () => mq.removeListener?.(sync);
            }
        }

        void loadInvoices();
        return cleanup;
    });

    $effect(() => {
        if (isMobile && viewMode === "table") viewMode = "cards";
    });

    async function loadInvoices() {
        loading = true;
        try {
            error = "";
            const [invoicesRes, tenantsRes] = await Promise.all([
                api.payment.listAllInvoices(),
                getTenantsCached().then((data) => ({ data, total: data.length })).catch(() => ({ data: [], total: 0 })),
            ]);

            invoices = invoicesRes || [];
            tenantNameById = Object.fromEntries(
                (tenantsRes.data || []).map((t: any) => [
                    t.id,
                    { name: t.name, slug: t.slug },
                ]),
            );
        } catch (e: any) {
            error = e.message || e.toString();
            toast.error(
                get(t)("superadmin.invoices.list.errors.load_failed") ||
                    "Failed to load invoices",
            );
        } finally {
            loading = false;
        }
    }

    function formatCurrency(amount: number, currency?: string) {
        return formatMoney(amount, { currency });
    }

    function tenantLabel(tenantId?: string) {
        if (!tenantId) return { name: "—", slug: "" };
        return tenantNameById[tenantId] || { name: tenantId, slug: "" };
    }

    async function checkStatus(id: string) {
        try {
            const status = await api.payment.checkStatus(id);
            toast.success(
                (get(t)("superadmin.invoices.list.toasts.status") ||
                    "Invoice status: ") + status,
            );
            void loadInvoices();
        } catch (e: any) {
            toast.error(
                (get(t)("superadmin.invoices.list.errors.check_failed") ||
                    "Failed to check status: ") + e.message,
            );
        }
    }
</script>

<div class="page-container fade-in">
    <div class="page-header">
        <div class="header-content">
            <h1>{$t("superadmin.invoices.list.title") || "All Invoices"}</h1>
            <p class="subtitle">
                {$t("superadmin.invoices.list.subtitle") ||
                    "Monitor all payments and transactions across the platform"}
            </p>
        </div>
        <button class="btn btn-secondary" onclick={loadInvoices}>
            <Icon name="refresh-cw" size={18} />
            <span>{$t("common.refresh") || "Refresh"}</span>
        </button>
    </div>

    <div
        class="stats-row"
        aria-label={$t("superadmin.invoices.aria.stats") || "Invoice stats"}
    >
        <button
            class="stat-btn"
            class:active={statusFilter === "all"}
            type="button"
            onclick={() => (statusFilter = "all")}
        >
            <div class="stat-title">{$t("superadmin.invoices.list.filters.all") || "All"}</div>
            <div class="stat-value">{stats.total}</div>
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "pending"}
            type="button"
            onclick={() => (statusFilter = "pending")}
        >
            <div class="stat-title">{$t("superadmin.invoices.list.filters.pending") || "Pending"}</div>
            <div class="stat-value">{stats.pending}</div>
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "paid"}
            type="button"
            onclick={() => (statusFilter = "paid")}
        >
            <div class="stat-title">{$t("superadmin.invoices.list.filters.paid") || "Paid"}</div>
            <div class="stat-value">{stats.paid}</div>
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "failed"}
            type="button"
            onclick={() => (statusFilter = "failed")}
        >
            <div class="stat-title">{$t("superadmin.invoices.list.filters.failed") || "Failed"}</div>
            <div class="stat-value">{stats.failed}</div>
        </button>
    </div>

    <div class="card content-card">
        {#if error}
            <div class="alert alert-error">{error}</div>
        {/if}

        <div class="toolbar-wrapper">
            <TableToolbar
                bind:searchQuery={search}
                placeholder={$t("superadmin.invoices.list.search") || "Search invoices..."}
            >
                {#snippet filters()}
                    <div class="filter-row">
                        <div class="status-filter">
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={statusFilter === "all"}
                                onclick={() => (statusFilter = "all")}
                            >
                                {$t("superadmin.invoices.list.filters.all") ||
                                    $t("common.all") ||
                                    "All"}
                            </button>
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={statusFilter === "pending"}
                                onclick={() => (statusFilter = "pending")}
                            >
                                {$t("superadmin.invoices.list.filters.pending") ||
                                    "Pending"}
                            </button>
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={statusFilter === "paid"}
                                onclick={() => (statusFilter = "paid")}
                            >
                                {$t("superadmin.invoices.list.filters.paid") ||
                                    "Paid"}
                            </button>
                            <button
                                type="button"
                                class="filter-chip"
                                class:active={statusFilter === "failed"}
                                onclick={() => (statusFilter = "failed")}
                            >
                                {$t("superadmin.invoices.list.filters.failed") ||
                                    "Failed"}
                            </button>
                        </div>

                        {#if !isMobile}
                            <button
                                type="button"
                                class="btn-icon view-btn"
                                class:active={viewMode === "cards"}
                                title={$t("superadmin.invoices.list.view.cards") ||
                                    "Cards view"}
                                onclick={() => (viewMode = "cards")}
                            >
                                <Icon name="grid" size={18} />
                            </button>
                            <button
                                type="button"
                                class="btn-icon view-btn"
                                class:active={viewMode === "table"}
                                title={$t("superadmin.invoices.list.view.table") ||
                                    "Table view"}
                                onclick={() => (viewMode = "table")}
                            >
                                <Icon name="list" size={18} />
                            </button>
                        {/if}
                    </div>
                {/snippet}
            </TableToolbar>
        </div>

        {#if loading}
            <div class="loading-state">
                <div class="spinner"></div>
                <p>
                    {$t("superadmin.invoices.list.loading") ||
                        "Loading invoices..."}
                </p>
            </div>
        {:else if filteredInvoices.length === 0}
            <div class="empty-grid">
                <div class="empty-icon">
                    <Icon name="file-text" size={56} />
                </div>
                <h4>
                    {$t("superadmin.invoices.list.empty") || "No invoices found"}
                </h4>
                <p>
                    {$t("superadmin.invoices.list.empty_hint") ||
                        "Try adjusting your search or filters."}
                </p>
            </div>
        {:else}
            {#if viewMode === "cards" || isMobile}
                <div
                    class="invoices-grid"
                    aria-label={$t("superadmin.invoices.aria.cards") ||
                        "Invoice cards"}
                >
                    {#each filteredInvoices as inv (inv.id)}
                        <div class="invoice-card">
                            <div class="invoice-top">
                                <div class="invoice-title">
                                    <div class="invoice-number">
                                        #{inv.invoice_number}
                                    </div>
                                    <div class="invoice-tenant">
                                        {#if inv.tenant_id}
                                            <span class="tenant-name">
                                                {tenantLabel(inv.tenant_id).name}
                                            </span>
                                            {#if tenantLabel(inv.tenant_id).slug}
                                                <span class="tenant-slug muted-text">
                                                    {tenantLabel(inv.tenant_id).slug}
                                                </span>
                                            {/if}
                                        {:else}
                                            <span class="tenant-name">—</span>
                                        {/if}
                                    </div>
                                </div>
                                <span class="status-pill {inv.status}">
                                    {inv.status}
                                </span>
                            </div>

                            <div class="invoice-meta">
                                <div class="meta-item">
                                    <span class="meta-label">
                                        {$t("superadmin.invoices.cards.amount") ||
                                            "Amount"}
                                    </span>
                                    <span class="meta-value">
                                        {formatCurrency(inv.amount, inv.currency_code)}
                                    </span>
                                </div>
                                <div class="meta-item">
                                    <span class="meta-label">
                                        {$t("superadmin.invoices.cards.due") ||
                                            "Due"}
                                    </span>
                                    <span class="meta-value">
                                        {new Date(inv.due_date).toLocaleDateString()}
                                    </span>
                                </div>
                            </div>

                            <div class="invoice-actions">
                                <button
                                    class="btn-icon"
                                    type="button"
                                    title={$t("superadmin.invoices.actions.check_status") ||
                                        "Check Status"}
                                    onclick={() => checkStatus(inv.id)}
                                >
                                    <Icon name="refresh-cw" size={18} />
                                </button>
                                <button
                                    class="btn-icon"
                                    type="button"
                                    title={$t("superadmin.invoices.actions.view_details") ||
                                        "View Details"}
                                    onclick={() =>
                                        goto(`/superadmin/invoices/${inv.id}`)}
                                >
                                    <Icon name="eye" size={18} />
                                </button>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}

            {#if viewMode === "table" && !isMobile}
                <div
                    class="table-wrapper"
                    aria-label={$t("superadmin.invoices.aria.table") ||
                        "Invoices table"}
                >
                    <Table
                        data={filteredInvoices}
                        {columns}
                        loading={false}
                        keyField="id"
                        pagination={true}
                        pageSize={10}
                        mobileView="scroll"
                    >
                        {#snippet cell({ item, column, key })}
                            {#if key === "tenant"}
                                <div class="table-tenant">
                                    {#if item.tenant_id}
                                        <div class="table-tenant-name">
                                            {tenantLabel(item.tenant_id).name}
                                        </div>
                                        {#if tenantLabel(item.tenant_id).slug}
                                            <div class="table-tenant-sub">
                                                {tenantLabel(item.tenant_id).slug}
                                            </div>
                                        {/if}
                                    {:else}
                                        —
                                    {/if}
                                </div>
                            {:else if key === "amount"}
                                {formatCurrency(item.amount, item.currency_code)}
                            {:else if key === "status"}
                                <span class="status-pill {item.status}">
                                    {item.status}
                                </span>
                            {:else if key === "due_date" || key === "created_at"}
                                {item[key]
                                    ? new Date(item[key]).toLocaleDateString()
                                    : "—"}
                            {:else if key === "actions"}
                                <div class="actions">
                                    <button
                                        class="action-btn"
                                        title={$t("superadmin.invoices.actions.check_status") ||
                                            "Check Status"}
                                        type="button"
                                        onclick={() => checkStatus(item.id)}
                                    >
                                        <Icon name="refresh-cw" size={18} />
                                    </button>
                                    <button
                                        type="button"
                                        class="action-btn"
                                        title={$t("superadmin.invoices.actions.view_details") ||
                                            "View Details"}
                                        onclick={() =>
                                            goto(
                                                `/superadmin/invoices/${item.id}`,
                                            )}
                                    >
                                        <Icon name="eye" size={18} />
                                    </button>
                                </div>
                            {:else}
                                {item[column.key]}
                            {/if}
                        {/snippet}
                    </Table>
                </div>
            {/if}
        {/if}
    </div>
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
        margin: 0 0 0.5rem;
    }
    .subtitle {
        color: var(--text-secondary);
    }
    .content-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        overflow: hidden;
    }

    .stats-row {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 0.75rem;
        margin-bottom: 1rem;
    }
    .stat-btn {
        text-align: left;
        padding: 0.9rem 1rem;
        border-radius: 14px;
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-primary);
        cursor: pointer;
        transition: 0.2s;
    }
    .stat-btn:hover {
        transform: translateY(-1px);
        border-color: rgba(99, 102, 241, 0.35);
    }
    .stat-btn.active {
        border-color: rgba(99, 102, 241, 0.6);
        box-shadow:
            0 18px 40px rgba(0, 0, 0, 0.2),
            0 0 0 1px rgba(99, 102, 241, 0.16);
    }
    .stat-title {
        color: var(--text-secondary);
        font-size: 0.85rem;
    }
    .stat-value {
        margin-top: 0.25rem;
        font-size: 1.5rem;
        font-weight: 800;
        letter-spacing: -0.02em;
    }

    .toolbar-wrapper {
        padding: 1rem 1rem 0.5rem 1rem;
    }
    .filter-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 0.75rem;
        flex-wrap: wrap;
    }
    .status-filter {
        display: flex;
        gap: 0.5rem;
        flex-wrap: wrap;
    }
    .filter-chip {
        padding: 0.45rem 0.9rem;
        border-radius: 12px;
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-secondary);
        font-weight: 700;
        cursor: pointer;
        transition: 0.2s;
    }
    .filter-chip.active {
        color: var(--text-primary);
        border-color: rgba(99, 102, 241, 0.6);
        background: rgba(99, 102, 241, 0.12);
    }
    .btn-icon {
        width: 40px;
        height: 40px;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.03);
        color: var(--text-secondary);
        display: inline-flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: 0.2s;
    }
    .btn-icon:hover {
        color: var(--text-primary);
        border-color: rgba(99, 102, 241, 0.4);
        background: rgba(99, 102, 241, 0.08);
    }
    .btn-icon.view-btn.active {
        color: var(--text-primary);
        border-color: rgba(99, 102, 241, 0.65);
        background: rgba(99, 102, 241, 0.14);
    }

    .loading-state {
        padding: 2.5rem 1rem;
        display: grid;
        place-items: center;
        gap: 0.75rem;
        color: var(--text-secondary);
    }
    .spinner {
        width: 22px;
        height: 22px;
        border-radius: 999px;
        border: 3px solid rgba(255, 255, 255, 0.14);
        border-top-color: rgba(99, 102, 241, 0.95);
        animation: spin 0.9s linear infinite;
    }
    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .empty-grid {
        padding: 2.5rem 1rem 3rem 1rem;
        display: grid;
        place-items: center;
        text-align: center;
        gap: 0.35rem;
        color: var(--text-secondary);
    }
    .empty-icon {
        width: 92px;
        height: 92px;
        border-radius: 22px;
        display: grid;
        place-items: center;
        background: rgba(99, 102, 241, 0.1);
        color: var(--color-primary);
        border: 1px solid rgba(99, 102, 241, 0.18);
        margin-bottom: 0.5rem;
    }
    .empty-grid h4 {
        margin: 0.3rem 0 0 0;
        color: var(--text-primary);
    }

    .status-pill {
        padding: 0.25rem 0.6rem;
        border-radius: 12px;
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
        border: 1px solid rgba(255, 255, 255, 0.1);
    }
    .status-pill.pending,
    .status-pill.verification_pending {
        background: rgba(245, 158, 11, 0.12);
        color: var(--color-warning, #f59e0b);
        border-color: rgba(245, 158, 11, 0.22);
    }
    .status-pill.paid {
        background: rgba(16, 185, 129, 0.12);
        color: var(--color-success, #10b981);
        border-color: rgba(16, 185, 129, 0.22);
    }
    .status-pill.failed {
        background: rgba(239, 68, 68, 0.12);
        color: var(--color-danger, #ef4444);
        border-color: rgba(239, 68, 68, 0.22);
    }

    .actions {
        display: flex;
        gap: 0.5rem;
        justify-content: flex-end;
    }
    .action-btn {
        width: 32px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: none;
        background: transparent;
        color: var(--text-secondary);
        cursor: pointer;
        border-radius: 6px;
    }
    .action-btn:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.6rem 1rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        border: none;
    }
    .btn-secondary {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .invoices-grid {
        padding: 1rem;
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
        gap: 1rem;
    }
    .invoice-card {
        background: linear-gradient(
            145deg,
            rgba(255, 255, 255, 0.06),
            rgba(255, 255, 255, 0.02)
        );
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 18px;
        padding: 1rem;
        display: flex;
        flex-direction: column;
        gap: 0.9rem;
    }
    .invoice-top {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 0.75rem;
    }
    .invoice-number {
        font-weight: 900;
        color: var(--text-primary);
        letter-spacing: -0.02em;
    }
    .invoice-tenant {
        margin-top: 0.2rem;
        display: grid;
        gap: 0.1rem;
    }
    .tenant-name {
        color: var(--text-secondary);
        font-weight: 700;
        font-size: 0.9rem;
    }
    .tenant-slug {
        font-size: 0.8rem;
    }
    .invoice-meta {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 0.75rem;
    }
    .meta-item {
        padding: 0.7rem 0.8rem;
        border-radius: 14px;
        background: rgba(0, 0, 0, 0.12);
        border: 1px solid rgba(255, 255, 255, 0.06);
    }
    .meta-label {
        display: block;
        font-size: 0.75rem;
        color: var(--text-tertiary, rgba(255, 255, 255, 0.6));
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
    }
    .meta-value {
        display: block;
        margin-top: 0.25rem;
        color: var(--text-primary);
        font-weight: 800;
    }
    .invoice-actions {
        display: flex;
        gap: 0.6rem;
        justify-content: flex-end;
    }

    .table-wrapper {
        padding: 0 1rem 1rem 1rem;
    }
    .table-tenant-name {
        font-weight: 800;
        color: var(--text-primary);
    }
    .table-tenant-sub {
        margin-top: 0.1rem;
        font-size: 0.8rem;
        color: var(--text-secondary);
    }

    .muted-text {
        color: var(--text-secondary);
    }

    @media (max-width: 720px) {
        .page-container {
            padding: 1.25rem;
        }
        .page-header {
            flex-direction: column;
            gap: 0.75rem;
            align-items: stretch;
        }
        .btn-secondary {
            justify-content: center;
        }
        .stats-row {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
        .invoices-grid {
            padding: 0.75rem;
            grid-template-columns: 1fr;
        }
        .invoice-meta {
            grid-template-columns: 1fr;
        }
    }

    :global([data-theme="light"]) .stat-btn {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 10px 28px rgba(0, 0, 0, 0.06),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }
    :global([data-theme="light"]) .filter-chip,
    :global([data-theme="light"]) .btn-icon {
        background: #ffffff;
        border-color: rgba(0, 0, 0, 0.08);
        color: var(--text-secondary);
    }
    :global([data-theme="light"]) .invoice-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 10px 28px rgba(0, 0, 0, 0.06),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }
    :global([data-theme="light"]) .meta-item {
        background: rgba(0, 0, 0, 0.03);
        border-color: rgba(0, 0, 0, 0.06);
    }
</style>
