<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { api } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import TableToolbar from "$lib/components/TableToolbar.svelte";
    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
    import StatsCard from "$lib/components/StatsCard.svelte";
    import { toast } from "$lib/stores/toast";
    import { formatMoney } from "$lib/utils/money";

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
    let confirmLoading = $state(false);
    let confirmTitle = $state("");
    let confirmMessage = $state("");
    let confirmAction = $state<(() => Promise<void>) | null>(null);
    let confirmType = $state<"danger" | "warning" | "info">("danger");
    let confirmKeyword = $state("");

    // Table configuration
    const planColumns = [
        { key: "name", label: "Plan", width: "30%" },
        { key: "pricing", label: "Pricing", width: "20%" },
        { key: "status", label: "Status", width: "18%" },
        { key: "sort_order", label: "Order", width: "10%" },
        {
            key: "actions",
            label: "Actions",
            width: "22%",
            align: "right" as const,
        },
    ];

    let planSearch = $state("");
    let statusFilter = $state<"all" | "active" | "inactive">("all");
    let viewMode = $state<"cards" | "table">("cards");
    let isMobile = $state(false);

    let stats = $derived({
        total: plans.length,
        active: plans.filter((p) => p.is_active).length,
        inactive: plans.filter((p) => !p.is_active).length,
        defaultPlan: plans.find((p) => p.is_default) || null,
    });

    let filteredPlans = $derived(
        plans
            .filter((p) => {
                const q = planSearch.trim().toLowerCase();
                const matchesSearch =
                    !q ||
                    p.name.toLowerCase().includes(q) ||
                    p.slug.toLowerCase().includes(q);

                const matchesStatus =
                    statusFilter === "all" ||
                    (statusFilter === "active" ? p.is_active : !p.is_active);

                return matchesSearch && matchesStatus;
            })
            .sort((a, b) => (a.sort_order ?? 0) - (b.sort_order ?? 0)),
    );

    onMount(async () => {
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

        await loadData();
        return cleanup;
    });

    $effect(() => {
        if (isMobile && viewMode === "table") viewMode = "cards";
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
        confirmLoading = false;
        showConfirm = true;
    }

    async function handleConfirm() {
        if (!confirmAction) return;
        confirmLoading = true;
        try {
            await confirmAction();
            showConfirm = false;
        } finally {
            confirmLoading = false;
            confirmAction = null;
        }
    }

    function handleCancelConfirm() {
        showConfirm = false;
        confirmLoading = false;
        confirmAction = null;
        confirmKeyword = "";
    }

    function confirmDeletePlan(plan: Plan) {
        openConfirmDialog(
            "Delete Plan",
            `Delete "${plan.name}"? This action cannot be undone. Type DELETE to confirm.`,
            async () => {
                await api.plans.delete(plan.id);
                toast.success("Plan deleted");
                await loadData();
            },
            "danger",
            "DELETE",
        );
    }

    function confirmToggleActive(plan: Plan) {
        if (plan.is_default && plan.is_active) {
            toast.error(
                "Default plan cannot be deactivated. Set another default first.",
            );
            return;
        }

        const next = !plan.is_active;
        openConfirmDialog(
            next ? "Activate Plan" : "Deactivate Plan",
            next
                ? `Activate "${plan.name}"? Tenants can be assigned to it again. Type ACTIVATE to confirm.`
                : `Deactivate "${plan.name}"? Tenants can no longer be assigned to it. Type DEACTIVATE to confirm.`,
            async () => {
                await api.plans.update(
                    plan.id,
                    plan.name,
                    plan.slug,
                    plan.description ?? undefined,
                    plan.price_monthly,
                    plan.price_yearly,
                    next,
                    plan.is_default,
                    plan.sort_order,
                );
                toast.success(next ? "Plan activated" : "Plan deactivated");
                await loadData();
            },
            next ? "info" : "warning",
            next ? "ACTIVATE" : "DEACTIVATE",
        );
    }

    function confirmSetDefault(plan: Plan) {
        if (plan.is_default) return;

        openConfirmDialog(
            "Set Default Plan",
            `Make "${plan.name}" the default plan for new tenants? Type DEFAULT to confirm.`,
            async () => {
                const currentDefault = plans.find((p) => p.is_default);
                if (currentDefault && currentDefault.id !== plan.id) {
                    await api.plans.update(
                        currentDefault.id,
                        currentDefault.name,
                        currentDefault.slug,
                        currentDefault.description ?? undefined,
                        currentDefault.price_monthly,
                        currentDefault.price_yearly,
                        currentDefault.is_active,
                        false,
                        currentDefault.sort_order,
                    );
                }

                await api.plans.update(
                    plan.id,
                    plan.name,
                    plan.slug,
                    plan.description ?? undefined,
                    plan.price_monthly,
                    plan.price_yearly,
                    true,
                    true,
                    plan.sort_order,
                );

                toast.success("Default plan updated");
                await loadData();
            },
            "info",
            "DEFAULT",
        );
    }

    function formatPrice(price: number): string {
        if (!price || price <= 0) return "Free";
        return formatMoney(price);
    }
</script>

<svelte:head>
    <title>Plans | Superadmin</title>
</svelte:head>

<div class="superadmin-content fade-in">
    <div class="stats-row" aria-label="Plan stats">
        <button
            class="stat-btn"
            class:active={statusFilter === "all"}
            onclick={() => (statusFilter = "all")}
            type="button"
            title="Show all plans"
        >
            <StatsCard
                title="All Plans"
                value={stats.total}
                icon="credit-card"
                color="primary"
            />
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "active"}
            onclick={() => (statusFilter = "active")}
            type="button"
            title="Show active plans"
        >
            <StatsCard
                title="Active"
                value={stats.active}
                icon="check-circle"
                color="success"
            />
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "inactive"}
            onclick={() => (statusFilter = "inactive")}
            type="button"
            title="Show inactive plans"
        >
            <StatsCard
                title="Inactive"
                value={stats.inactive}
                icon="ban"
                color="warning"
            />
        </button>
        <button
            class="stat-btn"
            type="button"
            title="Current default plan"
            disabled={!stats.defaultPlan}
        >
            <StatsCard
                title="Default"
                value={stats.defaultPlan?.name || "—"}
                icon="star"
                color="primary"
            />
        </button>
    </div>

    <div class="glass-card">
        <div class="card-header glass">
            <div>
                <h3>Subscription Plans</h3>
                <span class="muted">Manage pricing tiers and plan status</span>
            </div>
            <button class="btn btn-primary" onclick={createPlan} type="button">
                <Icon name="plus" size={18} />
                <span>New Plan</span>
            </button>
        </div>

        {#if loading}
            <div class="loading-state">
                <div class="spinner"></div>
                <p>Loading plans...</p>
            </div>
        {:else}
            <div class="toolbar-wrapper">
                <TableToolbar
                    bind:searchQuery={planSearch}
                    placeholder="Search plans..."
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
                                    All
                                </button>
                                <button
                                    type="button"
                                    class="filter-chip"
                                    class:active={statusFilter === "active"}
                                    onclick={() => (statusFilter = "active")}
                                >
                                    Active
                                </button>
                                <button
                                    type="button"
                                    class="filter-chip"
                                    class:active={statusFilter === "inactive"}
                                    onclick={() => (statusFilter = "inactive")}
                                >
                                    Inactive
                                </button>
                            </div>

                            {#if !isMobile}
                                <button
                                    type="button"
                                    class="btn-icon view-btn"
                                    class:active={viewMode === "cards"}
                                    title="Cards view"
                                    onclick={() => (viewMode = "cards")}
                                >
                                    <Icon name="grid" size={18} />
                                </button>
                                <button
                                    type="button"
                                    class="btn-icon view-btn"
                                    class:active={viewMode === "table"}
                                    title="Table view"
                                    onclick={() => (viewMode = "table")}
                                >
                                    <Icon name="list" size={18} />
                                </button>
                            {/if}
                        </div>
                    {/snippet}
                </TableToolbar>
            </div>

            {#if viewMode === "cards" || isMobile}
                <div class="plans-grid" aria-label="Plan cards">
                    {#each filteredPlans as plan (plan.id)}
                        <div
                            class="plan-card"
                            onclick={() => editPlan(plan)}
                            onkeydown={(e) =>
                                e.key === "Enter" && editPlan(plan)}
                            role="button"
                            tabindex="0"
                        >
                            <div class="plan-top">
                                <div>
                                    <div class="plan-name">
                                        <span>{plan.name}</span>
                                        {#if plan.is_default}
                                            <span
                                                class="pill default"
                                                title="Default plan"
                                                >Default</span
                                            >
                                        {/if}
                                        <span
                                            class="pill {plan.is_active
                                                ? 'active'
                                                : 'inactive'}"
                                            title={plan.is_active
                                                ? "Active"
                                                : "Inactive"}
                                        >
                                            {plan.is_active
                                                ? "Active"
                                                : "Inactive"}
                                        </span>
                                    </div>
                                    <div class="plan-code">{plan.slug}</div>
                                </div>
                                <div class="plan-price">
                                    <div class="price-main">
                                        {formatPrice(plan.price_monthly)}<span
                                            class="unit"
                                            >/mo</span
                                        >
                                    </div>
                                    <div class="price-sub">
                                        {formatPrice(plan.price_yearly)}<span
                                            class="unit"
                                            >/yr</span
                                        >
                                    </div>
                                </div>
                            </div>

                            {#if plan.description}
                                <div class="plan-desc">{plan.description}</div>
                            {:else}
                                <div class="plan-desc muted-text">
                                    No description
                                </div>
                            {/if}

                            <div class="plan-actions">
                                <button
                                    class="btn-icon"
                                    title="Edit"
                                    type="button"
                                    onclick={(e) => {
                                        e.stopPropagation();
                                        editPlan(plan);
                                    }}
                                >
                                    <Icon name="edit" size={18} />
                                </button>
                                <button
                                    class="btn-icon"
                                    title={plan.is_default
                                        ? "Already default"
                                        : "Set as default"}
                                    type="button"
                                    disabled={plan.is_default}
                                    onclick={(e) => {
                                        e.stopPropagation();
                                        confirmSetDefault(plan);
                                    }}
                                >
                                    <Icon name="star" size={18} />
                                </button>
                                <button
                                    class="btn-icon {plan.is_active
                                        ? 'warn'
                                        : 'success'}"
                                    title={plan.is_active
                                        ? "Deactivate"
                                        : "Activate"}
                                    type="button"
                                    onclick={(e) => {
                                        e.stopPropagation();
                                        confirmToggleActive(plan);
                                    }}
                                >
                                    <Icon
                                        name={plan.is_active
                                            ? "ban"
                                            : "check-circle"}
                                        size={18}
                                    />
                                </button>
                                <button
                                    class="btn-icon danger"
                                    title="Delete"
                                    type="button"
                                    onclick={(e) => {
                                        e.stopPropagation();
                                        confirmDeletePlan(plan);
                                    }}
                                >
                                    <Icon name="trash" size={18} />
                                </button>
                            </div>
                        </div>
                    {/each}

                    {#if filteredPlans.length === 0}
                        <div class="empty-grid">
                            <div class="empty-icon">
                                <Icon name="credit-card" size={56} />
                            </div>
                            <h4>No plans found</h4>
                            <p>Try adjusting your search or filters.</p>
                            <button
                                class="btn btn-primary"
                                type="button"
                                onclick={createPlan}
                            >
                                <Icon name="plus" size={18} />
                                <span>Create plan</span>
                            </button>
                        </div>
                    {/if}
                </div>
            {:else}
                <div class="table-wrapper" aria-label="Plans table">
                    <Table
                        columns={planColumns}
                        data={filteredPlans}
                        loading={false}
                        keyField="id"
                        pagination={true}
                        pageSize={10}
                        mobileView="scroll"
                    >
                        {#snippet cell({ item, column, key })}
                            {#if key === "name"}
                                <div class="table-plan">
                                    <div class="table-plan-name">
                                        {item.name}
                                    </div>
                                    <div class="table-plan-sub">
                                        {item.slug}
                                    </div>
                                </div>
                            {:else if key === "pricing"}
                                <div class="table-pricing">
                                    <span class="mono"
                                        >{formatPrice(item.price_monthly)}</span
                                    >
                                    <span class="sep">/</span>
                                    <span class="mono"
                                        >{formatPrice(item.price_yearly)}</span
                                    >
                                </div>
                            {:else if key === "status"}
                                <div class="status-badges">
                                    <span
                                        class="badge {item.is_active
                                            ? 'success'
                                            : 'warning'}"
                                    >
                                        {item.is_active
                                            ? "Active"
                                            : "Inactive"}
                                    </span>
                                    {#if item.is_default}
                                        <span class="badge primary"
                                            >Default</span
                                        >
                                    {/if}
                                </div>
                            {:else if key === "actions"}
                                <div class="table-actions">
                                    <button
                                        class="btn-icon"
                                        title="Edit"
                                        type="button"
                                        onclick={() => editPlan(item)}
                                    >
                                        <Icon name="edit" size={18} />
                                    </button>
                                    <button
                                        class="btn-icon"
                                        title={item.is_default
                                            ? "Already default"
                                            : "Set as default"}
                                        type="button"
                                        disabled={item.is_default}
                                        onclick={() => confirmSetDefault(item)}
                                    >
                                        <Icon name="star" size={18} />
                                    </button>
                                    <button
                                        class="btn-icon {item.is_active
                                            ? 'warn'
                                            : 'success'}"
                                        title={item.is_active
                                            ? "Deactivate"
                                            : "Activate"}
                                        type="button"
                                        onclick={() =>
                                            confirmToggleActive(item)}
                                    >
                                        <Icon
                                            name={item.is_active
                                                ? "ban"
                                                : "check-circle"}
                                            size={18}
                                        />
                                    </button>
                                    <button
                                        class="btn-icon danger"
                                        title="Delete"
                                        type="button"
                                        onclick={() => confirmDeletePlan(item)}
                                    >
                                        <Icon name="trash" size={18} />
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

<ConfirmDialog
    bind:show={showConfirm}
    title={confirmTitle}
    message={confirmMessage}
    type={confirmType}
    confirmationKeyword={confirmKeyword}
    loading={confirmLoading}
    onconfirm={handleConfirm}
    oncancel={handleCancelConfirm}
/>

<style>
    .superadmin-content {
        padding: clamp(16px, 3vw, 32px);
        max-width: 1400px;
        margin: 0 auto;
        color: var(--text-primary);
        --glass: rgba(255, 255, 255, 0.04);
        --glass-border: rgba(255, 255, 255, 0.08);
    }

    .stats-row {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 1rem;
        margin-bottom: 1.25rem;
    }

    .stat-btn {
        border: none;
        padding: 0;
        background: transparent;
        cursor: pointer;
        text-align: left;
        border-radius: 18px;
        transition: transform 0.15s ease;
    }

    .stat-btn:hover:not(:disabled) {
        transform: translateY(-1px);
    }

    .stat-btn:disabled {
        opacity: 0.7;
        cursor: default;
    }

    .stat-btn.active :global(.stats-card) {
        border-color: rgba(99, 102, 241, 0.35);
        box-shadow: 0 0 0 1px rgba(99, 102, 241, 0.25);
    }

    .glass-card {
        background: var(--glass);
        border: 1px solid var(--glass-border);
        border-radius: var(--radius-lg);
        overflow: hidden;
        box-shadow: 0 18px 45px rgba(0, 0, 0, 0.35);
        backdrop-filter: blur(12px);
    }

    :global([data-theme="light"]) .glass-card {
        background: rgba(255, 255, 255, 0.75);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 12px 28px rgba(0, 0, 0, 0.06),
            0 0 0 1px rgba(255, 255, 255, 0.85);
    }

    .card-header {
        padding: 1.25rem 1.25rem 1rem 1.25rem;
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 1rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    }

    :global([data-theme="light"]) .card-header {
        border-bottom-color: rgba(0, 0, 0, 0.06);
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

    .btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.25rem;
        border-radius: var(--radius-md);
        font-weight: 650;
        font-size: 0.9rem;
        border: none;
        cursor: pointer;
        transition: all 0.2s;
        white-space: nowrap;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover {
        background: var(--color-primary-hover);
    }

    .toolbar-wrapper {
        padding: 1rem 1.25rem 0.5rem 1.25rem;
    }

    .status-filter {
        display: inline-flex;
        align-items: center;
        gap: 0.35rem;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 12px;
        padding: 0.35rem;
    }

    :global([data-theme="light"]) .status-filter {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .filter-chip {
        border: none;
        background: transparent;
        color: var(--text-secondary);
        padding: 0.45rem 0.75rem;
        border-radius: 10px;
        cursor: pointer;
        font-weight: 650;
        font-size: 0.85rem;
        transition: all 0.2s;
    }

    .filter-chip:hover {
        color: var(--text-primary);
        background: rgba(255, 255, 255, 0.05);
    }

    :global([data-theme="light"]) .filter-chip:hover {
        background: rgba(0, 0, 0, 0.04);
    }

    .filter-chip.active {
        background: rgba(99, 102, 241, 0.18);
        border: 1px solid rgba(99, 102, 241, 0.25);
        color: var(--text-primary);
    }

    .filter-row {
        display: inline-flex;
        align-items: center;
        gap: 0.6rem;
        flex-wrap: wrap;
    }

    .view-btn {
        width: 38px;
        height: 38px;
    }

    .view-btn.active {
        background: rgba(99, 102, 241, 0.12);
        border-color: rgba(99, 102, 241, 0.35);
        color: var(--text-primary);
    }

    .plans-grid {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 1rem;
        padding: 0.75rem 1.25rem 1rem 1.25rem;
    }

    .plan-card {
        border-radius: 18px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.03);
        box-shadow: 0 12px 30px rgba(0, 0, 0, 0.25);
        padding: 1rem;
        cursor: pointer;
        transition:
            transform 0.15s ease,
            border-color 0.15s ease,
            background 0.15s ease;
    }

    .plan-card:hover {
        transform: translateY(-1px);
        border-color: rgba(99, 102, 241, 0.25);
        background: rgba(255, 255, 255, 0.04);
    }

    :global([data-theme="light"]) .plan-card {
        background: rgba(255, 255, 255, 0.85);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow: 0 10px 24px rgba(0, 0, 0, 0.06);
    }

    .plan-top {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 0.75rem;
    }

    .plan-name {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 0.5rem;
        font-weight: 800;
        letter-spacing: -0.01em;
        color: var(--text-primary);
        line-height: 1.2;
    }

    .plan-code {
        margin-top: 0.25rem;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        color: var(--text-secondary);
        font-size: 0.82rem;
    }

    .plan-price {
        text-align: right;
        white-space: nowrap;
    }

    .price-main {
        font-weight: 900;
        font-size: 1.15rem;
        color: var(--text-primary);
    }

    .price-sub {
        margin-top: 0.1rem;
        font-weight: 650;
        font-size: 0.9rem;
        color: var(--text-secondary);
    }

    .unit {
        font-weight: 650;
        font-size: 0.85rem;
        color: var(--text-secondary);
        margin-left: 0.15rem;
    }

    .plan-desc {
        margin-top: 0.85rem;
        color: var(--text-secondary);
        font-size: 0.92rem;
        line-height: 1.5;
        min-height: 2.8em;
    }

    .muted-text {
        opacity: 0.75;
    }

    .plan-actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
        margin-top: 0.9rem;
    }

    .pill {
        display: inline-flex;
        align-items: center;
        height: 22px;
        padding: 0 0.55rem;
        border-radius: 999px;
        font-size: 0.72rem;
        font-weight: 750;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        border: 1px solid rgba(255, 255, 255, 0.12);
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-primary);
    }

    :global([data-theme="light"]) .pill {
        border-color: rgba(0, 0, 0, 0.08);
        background: rgba(0, 0, 0, 0.02);
    }

    .pill.default {
        border-color: rgba(99, 102, 241, 0.35);
        background: rgba(99, 102, 241, 0.12);
    }

    .pill.active {
        border-color: rgba(16, 185, 129, 0.35);
        background: rgba(16, 185, 129, 0.12);
    }

    .pill.inactive {
        border-color: rgba(245, 158, 11, 0.35);
        background: rgba(245, 158, 11, 0.12);
    }

    .table-wrapper {
        padding: 0 1.25rem 1rem 1.25rem;
    }

    .table-actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
    }

    .table-plan-name {
        font-weight: 750;
        color: var(--text-primary);
    }

    .table-plan-sub {
        margin-top: 0.15rem;
        color: var(--text-secondary);
        font-size: 0.82rem;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    }

    .table-pricing {
        display: inline-flex;
        align-items: center;
        gap: 0.35rem;
        white-space: nowrap;
        color: var(--text-primary);
        font-weight: 650;
    }

    .mono {
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        font-size: 0.92rem;
    }

    .sep {
        color: var(--text-secondary);
    }

    .status-badges {
        display: inline-flex;
        gap: 0.45rem;
        flex-wrap: wrap;
    }

    .badge {
        display: inline-flex;
        align-items: center;
        padding: 0.22rem 0.6rem;
        border-radius: 999px;
        font-size: 0.72rem;
        font-weight: 700;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        border: 1px solid rgba(255, 255, 255, 0.12);
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-primary);
    }

    :global([data-theme="light"]) .badge {
        border-color: rgba(0, 0, 0, 0.08);
        background: rgba(0, 0, 0, 0.02);
    }

    .badge.success {
        border-color: rgba(16, 185, 129, 0.35);
        background: rgba(16, 185, 129, 0.12);
        color: #10b981;
    }

    .badge.warning {
        border-color: rgba(245, 158, 11, 0.35);
        background: rgba(245, 158, 11, 0.12);
        color: #f59e0b;
    }

    .badge.primary {
        border-color: rgba(99, 102, 241, 0.35);
        background: rgba(99, 102, 241, 0.12);
        color: #818cf8;
    }

    .loading-state {
        padding: 3rem 1.25rem;
        display: grid;
        place-items: center;
        gap: 0.75rem;
        color: var(--text-secondary);
    }

    .spinner {
        width: 28px;
        height: 28px;
        border-radius: 999px;
        border: 3px solid rgba(255, 255, 255, 0.12);
        border-top-color: rgba(99, 102, 241, 0.8);
        animation: spin 0.9s linear infinite;
    }

    :global([data-theme="light"]) .spinner {
        border-color: rgba(0, 0, 0, 0.08);
        border-top-color: rgba(99, 102, 241, 0.9);
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .empty-grid {
        grid-column: 1 / -1;
        border-radius: 18px;
        border: 1px dashed rgba(255, 255, 255, 0.18);
        background: rgba(255, 255, 255, 0.02);
        padding: 2rem 1.25rem;
        text-align: center;
        color: var(--text-secondary);
        display: grid;
        place-items: center;
        gap: 0.5rem;
    }

    :global([data-theme="light"]) .empty-grid {
        border-color: rgba(0, 0, 0, 0.12);
        background: rgba(0, 0, 0, 0.01);
    }

    .empty-grid h4 {
        margin: 0.35rem 0 0 0;
        color: var(--text-primary);
        font-size: 1rem;
        font-weight: 800;
    }

    .empty-grid p {
        margin: 0;
        max-width: 46ch;
    }

    .empty-icon {
        color: var(--text-secondary);
        opacity: 0.9;
    }

    :global(.btn-icon.danger:hover:not(:disabled)) {
        background: rgba(239, 68, 68, 0.1);
        border-color: rgba(239, 68, 68, 0.35);
        color: #ef4444;
    }

    :global(.btn-icon.warn:hover:not(:disabled)) {
        background: rgba(245, 158, 11, 0.12);
        border-color: rgba(245, 158, 11, 0.35);
        color: #f59e0b;
    }

    :global(.btn-icon.success:hover:not(:disabled)) {
        background: rgba(16, 185, 129, 0.12);
        border-color: rgba(16, 185, 129, 0.35);
        color: #10b981;
    }

    @media (max-width: 1024px) {
        .stats-row {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }

        .plans-grid {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
    }

    @media (max-width: 720px) {
        .plans-grid {
            grid-template-columns: 1fr;
            padding-top: 0.25rem;
        }

        .card-header {
            flex-direction: column;
            align-items: stretch;
        }

        .btn-primary {
            width: 100%;
            justify-content: center;
        }

        .plan-top {
            flex-direction: column;
            align-items: flex-start;
        }

        .plan-price {
            text-align: left;
        }

        .filter-row {
            width: 100%;
            justify-content: space-between;
        }
    }
</style>
