<script lang="ts">
    import { onMount } from "svelte";
    import { api, type Invoice } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import { toast } from "$lib/stores/toast";

    let invoices: Invoice[] = [];
    let loading = true;
    let error = "";

    const columns = [
        { key: "invoice_number", label: "Invoice #", sortable: true },
        { key: "description", label: "Description", sortable: true },
        { key: "amount", label: "Amount", sortable: true },
        { key: "status", label: "Status", sortable: true },
        { key: "due_date", label: "Due Date", sortable: true },
        { key: "actions", label: "Actions", align: "right" },
    ];

    onMount(() => {
        loadInvoices();
    });

    async function loadInvoices() {
        loading = true;
        try {
            invoices = await api.payment.listInvoices();
        } catch (e: any) {
            error = e.toString();
            toast.error("Failed to load invoices");
        } finally {
            loading = false;
        }
    }

    function formatCurrency(amount: number) {
        return new Intl.NumberFormat('id-ID', {
            style: 'currency',
            currency: 'IDR'
        }).format(amount);
    }
</script>

<div class="page-container fade-in">
    <div class="page-header">
        <div class="header-content">
            <h1>Billing & Invoices</h1>
            <p class="subtitle">View and manage your subscription payments</p>
        </div>
        <button class="btn btn-secondary" on:click={loadInvoices}>
            <Icon name="refresh-cw" size={18} />
            <span>Refresh</span>
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
            searchPlaceholder="Search invoices..."
        >
            <svelte:fragment slot="cell" let:item let:column>
                {#if column.key === "amount"}
                    {formatCurrency(item.amount)}
                {:else if column.key === "status"}
                    <span class="status-pill {item.status}">{item.status}</span>
                {:else if column.key === "due_date"}
                    {new Date(item[column.key]).toLocaleDateString()}
                {:else if column.key === "actions"}
                    <div class="actions">
                        {#if item.status === 'pending'}
                            <a href="/pay/{item.id}" class="btn btn-primary btn-sm">
                                <Icon name="credit-card" size={14} />
                                Pay Now
                            </a>
                        {:else}
                            <a href="/pay/{item.id}" class="action-btn" title="View Details">
                                <Icon name="eye" size={18} />
                            </a>
                        {/if}
                    </div>
                {:else}
                    {item[column.key]}
                {/if}
            </svelte:fragment>
        </Table>
    </div>
</div>

<style>
    .page-container { padding: 2rem; max-width: 1200px; margin: 0 auto; }
    .page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 2rem; }
    .header-content h1 { font-size: 1.8rem; font-weight: 700; margin: 0 0 0.5rem; }
    .subtitle { color: var(--text-secondary); }
    .content-card { background: var(--bg-surface); border: 1px solid var(--border-color); border-radius: 12px; overflow: hidden; }
    
    .status-pill {
        padding: 0.25rem 0.6rem;
        border-radius: 12px;
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
    }
    .status-pill.pending { background: #fef3c7; color: #d97706; }
    .status-pill.paid { background: #dcfce7; color: #16a34a; }
    .status-pill.failed { background: #fee2e2; color: #dc2626; }

    .actions { display: flex; gap: 0.5rem; justify-content: flex-end; align-items: center; }
    .action-btn { 
        width: 32px; height: 32px; display: flex; align-items: center; justify-content: center;
        border: none; background: transparent; color: var(--text-secondary); cursor: pointer; border-radius: 6px;
    }
    .action-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
    
    .btn { display: inline-flex; align-items: center; gap: 0.5rem; padding: 0.6rem 1rem; border-radius: 8px; font-weight: 600; cursor: pointer; border: none; text-decoration: none; }
    .btn-sm { padding: 0.4rem 0.8rem; font-size: 0.85rem; }
    .btn-primary { background: var(--color-primary); color: white; }
    .btn-secondary { background: var(--bg-tertiary); color: var(--text-primary); }
</style>
