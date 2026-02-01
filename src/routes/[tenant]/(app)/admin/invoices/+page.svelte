<script lang="ts">
    import { onMount } from "svelte";
    import { api, type Invoice } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import { toast } from "$lib/stores/toast";
    import { formatMoney } from "$lib/utils/money";
    import { goto } from "$app/navigation";
    import { t } from "svelte-i18n";
    import { get } from "svelte/store";

    let invoices = $state<Invoice[]>([]);
    let loading = $state(true);
    let error = $state("");

    const columns = $derived.by(() => [
        {
            key: "invoice_number",
            label: $t("admin.invoices.columns.invoice_number") || "Invoice #",
            sortable: true,
        },
        {
            key: "description",
            label: $t("admin.invoices.columns.description") || "Description",
            sortable: true,
        },
        {
            key: "amount",
            label: $t("admin.invoices.columns.amount") || "Amount",
            sortable: true,
        },
        {
            key: "status",
            label: $t("admin.invoices.columns.status") || "Status",
            sortable: true,
        },
        {
            key: "due_date",
            label: $t("admin.invoices.columns.due_date") || "Due Date",
            sortable: true,
        },
        { key: "actions", label: "", align: "right" as const },
    ]);

    onMount(() => {
        loadInvoices();
    });

    async function loadInvoices() {
        loading = true;
        try {
            invoices = await api.payment.listInvoices();
        } catch (e: any) {
            error = e.toString();
            toast.error(
                get(t)("admin.invoices.toasts.load_failed") ||
                    "Failed to load invoices",
            );
        } finally {
            loading = false;
        }
    }

    function formatCurrency(amount: number, currency?: string) {
        return formatMoney(amount, { currency });
    }
</script>

<div class="page-container fade-in">
    <div class="page-header">
        <div class="header-content">
            <h1>{$t("admin.invoices.title") || "Billing & Invoices"}</h1>
            <p class="subtitle">
                {$t("admin.invoices.subtitle") ||
                    "View and manage your subscription payments"}
            </p>
        </div>
        <button class="btn btn-secondary" onclick={loadInvoices}>
            <Icon name="refresh-cw" size={18} />
            <span>{$t("common.refresh") || "Refresh"}</span>
        </button>
    </div>

    <div class="card content-card">
        {#if error}
            <div class="alert alert-error">{error}</div>
        {/if}

        <Table
            {loading}
            data={invoices}
            {columns}
            searchable={true}
            searchPlaceholder={$t("admin.invoices.search_placeholder") ||
                "Search invoices..."}
        >
            {#snippet cell({ item, column })}
                {#if column.key === "amount"}
                    {formatCurrency(item.amount, item.currency_code)}
                {:else if column.key === "status"}
                    <span class="status-pill {item.status}">{item.status}</span>
                {:else if column.key === "due_date"}
                    {new Date(item[column.key]).toLocaleDateString()}
                {:else if column.key === "actions"}
                    <div class="actions">
                        {#if item.status === "pending"}
                            <button
                                type="button"
                                class="btn btn-primary btn-sm"
                                onclick={() => goto(`/pay/${item.id}`)}
                            >
                                <Icon name="credit-card" size={14} />
                                {$t("admin.invoices.pay_now") || "Pay Now"}
                            </button>
                        {:else}
                            <button
                                type="button"
                                class="action-btn"
                                title={$t("admin.invoices.view_details") ||
                                    "View Details"}
                                aria-label={$t("admin.invoices.view_details") ||
                                    "View Details"}
                                onclick={() => goto(`/pay/${item.id}`)}
                            >
                                <Icon name="eye" size={18} />
                            </button>
                        {/if}
                    </div>
                {:else}
                    {item[column.key]}
                {/if}
            {/snippet}
        </Table>
    </div>
</div>

<style>
    .page-container {
        padding: clamp(1rem, 3vw, 2rem);
        max-width: 1200px;
        margin: 0 auto;
    }
    .page-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 2rem;
        gap: 1rem;
        flex-wrap: wrap;
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

    .status-pill {
        padding: 0.25rem 0.6rem;
        border-radius: 12px;
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
        border: 1px solid transparent;
    }
    .status-pill.pending {
        background: rgba(245, 158, 11, 0.14);
        color: var(--color-warning, #f59e0b);
        border-color: rgba(245, 158, 11, 0.22);
    }
    .status-pill.verification_pending {
        background: rgba(245, 158, 11, 0.14);
        color: var(--color-warning, #f59e0b);
        border-color: rgba(245, 158, 11, 0.22);
    }
    .status-pill.paid {
        background: rgba(16, 185, 129, 0.14);
        color: var(--color-success, #10b981);
        border-color: rgba(16, 185, 129, 0.22);
    }
    .status-pill.failed {
        background: rgba(239, 68, 68, 0.14);
        color: var(--color-danger, #ef4444);
        border-color: rgba(239, 68, 68, 0.22);
    }
    .status-pill.expired {
        background: rgba(148, 163, 184, 0.12);
        color: var(--text-secondary);
        border-color: rgba(148, 163, 184, 0.18);
    }

    .actions {
        display: flex;
        gap: 0.5rem;
        justify-content: flex-end;
        align-items: center;
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
        text-decoration: none;
    }
    .btn-sm {
        padding: 0.4rem 0.8rem;
        font-size: 0.85rem;
    }
    .btn-primary {
        background: var(--color-primary);
        color: white;
    }
    .btn-secondary {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    @media (max-width: 768px) {
        .page-header {
            flex-direction: column;
            align-items: stretch;
        }

        .btn.btn-secondary {
            width: 100%;
            justify-content: center;
        }

        .header-content h1 {
            font-size: 1.35rem;
        }

        .content-card {
            border-radius: 16px;
        }
    }
</style>
