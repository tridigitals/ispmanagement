<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { api } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import TableToolbar from "$lib/components/TableToolbar.svelte";
    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
    import { toast } from "$lib/stores/toast";

    interface Plan {
        id: string;
        name: string;
        slug: string;
        description: string | null;
        price_monthly: number;
        price_yearly: number;
        is_active: boolean;
        is_default: boolean;
        sort_order: number;
    }

    let plans = $state<Plan[]>([]);
    let loading = $state(true);

    // Confirm Dialog State
    let showConfirm = $state(false);
    let confirmTitle = $state("");
    let confirmMessage = $state("");
    let confirmAction = $state<() => Promise<void>>(async () => {});
    let confirmType = $state<"danger" | "warning" | "info">("danger");
    let confirmKeyword = $state("");

    // Table configuration
    const planColumns = [
        { key: "name", label: "Plan Name", width: "25%" },
        { key: "slug", label: "Code", width: "15%" },
        { key: "price_monthly", label: "Monthly", width: "15%" },
        { key: "price_yearly", label: "Yearly", width: "15%" },
        { key: "status", label: "Status", width: "15%" },
        {
            key: "actions",
            label: "Actions",
            width: "15%",
            align: "right" as const,
        },
    ];

    let planSearch = $state("");

    let filteredPlans = $derived(plans.filter(
        (p) =>
            p.name.toLowerCase().includes(planSearch.toLowerCase()) ||
            p.slug.toLowerCase().includes(planSearch.toLowerCase()),
    ));

    onMount(async () => {
        await loadData();
    });

    async function loadData() {
        loading = true;
        try {
            plans = await api.plans.list();
        } catch (e: any) {
            toast.error(e.message || "Failed to load data");
        }
        loading = false;
    }

    function createPlan() {
        goto("/superadmin/plans/new");
    }

    function editPlan(plan: Plan) {
        goto(`/superadmin/plans/${plan.id}`);
    }

    // Confirm Dialog Logic
    function openConfirmDialog(
        title: string,
        message: string,
        action: () => Promise<void>,
        type: "danger" | "warning" | "info" = "danger",
        keyword = "",
    ) {
        confirmTitle = title;
        confirmMessage = message;
        confirmAction = action;
        confirmType = type;
        confirmKeyword = keyword;
        showConfirm = true;
    }

    async function handleConfirm() {
        await confirmAction();
        showConfirm = false;
    }

    function confirmDeletePlan(plan: Plan) {
        openConfirmDialog(
            "Delete Plan",
            `Are you sure you want to delete the plan "${plan.name}"? This action cannot be undone.`,
            async () => {
                try {
                    await api.plans.delete(plan.id);
                    toast.success("Plan deleted");
                    await loadData();
                } catch (e: any) {
                    toast.error(e.message || "Failed to delete plan");
                }
            },
            "danger",
            plan.name,
        );
    }

    function formatPrice(price: number): string {
        return new Intl.NumberFormat("en-US", {
            style: "currency",
            currency: "USD",
        }).format(price);
    }
</script>

<svelte:head>
    <title>Plans | Superadmin</title>
</svelte:head>

<div class="plans-page">
    <header class="page-header">
        <div class="header-left">
            <h1>Subscription Plans</h1>
            <p class="subtitle">Manage plans and features for your tenants</p>
        </div>
        <div class="header-actions">
            <button class="btn btn-primary" onclick={createPlan}>
                <Icon name="plus" size={16} />
                Add Plan
            </button>
        </div>
    </header>

    {#if loading}
        <div class="loading">Loading...</div>
    {:else}
        <div class="content-wrapper">
            <TableToolbar
                placeholder="Search plans..."
                bind:searchQuery={planSearch}
            >
                <div slot="actions">
                    <!-- Additional actions if needed -->
                </div>
            </TableToolbar>

            <Table
                columns={planColumns}
                data={filteredPlans}
                loading={false}
                keyField="id"
                pagination={true}
                pageSize={10}
            >
                <div slot="cell" let:item let:column>
                    {#if column.key === "status"}
                        <div class="status-badges">
                            {#if item.is_active}
                                <span class="badge badge-success">Active</span>
                            {:else}
                                <span class="badge badge-warning">Inactive</span
                                >
                            {/if}
                            {#if item.is_default}
                                <span class="badge badge-primary">Default</span>
                            {/if}
                        </div>
                    {:else if column.key === "price_monthly" || column.key === "price_yearly"}
                        {formatPrice(item[column.key])}
                    {:else if column.key === "actions"}
                        <div class="table-actions">
                            <button
                                class="btn btn-sm btn-icon"
                                title="Edit"
                                onclick={() => editPlan(item)}
                            >
                                <Icon name="edit" size={16} />
                            </button>
                            <button
                                class="btn btn-sm btn-danger btn-icon"
                                title="Delete"
                                onclick={() => confirmDeletePlan(item)}
                            >
                                <Icon name="trash" size={16} />
                            </button>
                        </div>
                    {:else}
                        {item[column.key]}
                    {/if}
                </div>
            </Table>
        </div>
    {/if}
</div>

<ConfirmDialog
    bind:show={showConfirm}
    title={confirmTitle}
    message={confirmMessage}
    type={confirmType}
    confirmationKeyword={confirmKeyword}
    onconfirm={handleConfirm}
/>

<style>
    /* =========================================
       PREMIUM STYLES
       ========================================= */

    .plans-page {
        padding: 2rem;
        max-width: 1400px;
        margin: 0 auto;
        min-height: 80vh;
    }

    /* Page Header */
    .page-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
        margin-bottom: 2.5rem;
        padding-bottom: 1.5rem;
        border-bottom: 1px solid var(--border-subtle);
    }

    .page-header h1 {
        margin: 0;
        font-size: 2rem;
        font-weight: 700;
        letter-spacing: -0.02em;
        background: linear-gradient(to right, #fff, #94a3b8);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        background-clip: text;
    }

    .subtitle {
        color: var(--text-secondary);
        margin: 0.5rem 0 0;
        font-size: 1rem;
    }

    .header-actions {
        display: flex;
        gap: 1rem;
    }

    .header-actions .btn {
        min-width: 140px;
        box-shadow:
            0 4px 6px -1px rgba(0, 0, 0, 0.1),
            0 2px 4px -1px rgba(0, 0, 0, 0.06);
    }

    /* Content Layout */
    .content-wrapper {
        display: block; /* Full width layout */
        max-width: 1200px;
        margin: 0 auto 3rem auto;
    }

    /* Status Badges */
    .status-badges {
        display: flex;
        gap: 0.5rem;
    }

    .badge {
        display: inline-flex;
        align-items: center;
        padding: 0.2rem 0.6rem;
        border-radius: 9999px;
        font-size: 0.7rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .badge-success {
        background: rgba(16, 185, 129, 0.15);
        color: #34d399;
        border: 1px solid rgba(16, 185, 129, 0.2);
    }

    .badge-warning {
        background: rgba(245, 158, 11, 0.15);
        color: #fbbf24;
        border: 1px solid rgba(245, 158, 11, 0.2);
    }

    .badge-primary {
        background: rgba(99, 102, 241, 0.15);
        color: #818cf8;
        border: 1px solid rgba(99, 102, 241, 0.2);
    }

    .table-actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
    }

    .btn-icon {
        padding: 0.4rem;
        border-radius: var(--radius-md);
        background: transparent;
        color: var(--text-secondary);
        border: 1px solid transparent;
        transition: all 0.2s;
        cursor: pointer;
    }

    .btn-icon:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
        border-color: var(--border-color);
    }

    .btn-danger:hover {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
        border-color: rgba(239, 68, 68, 0.2);
    }
</style>
