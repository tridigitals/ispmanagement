<script lang="ts">
    import { user, can } from "$lib/stores/auth";
    import { onMount } from "svelte";
    import {
        api,
        type TenantSubscriptionDetails,
        type Invoice,
    } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import Icon from "$lib/components/Icon.svelte";
    import { fade } from "svelte/transition";
    import { toast } from "svelte-sonner";
    import Table from "$lib/components/Table.svelte";

    let loading = $state(true);
    let subscription = $state<TenantSubscriptionDetails | null>(null);
    let availablePlans = $state<any[]>([]);
    let invoices = $state<Invoice[]>([]);
    let upgrading = $state(false);
    let activeTab = $state<"overview" | "plans" | "history">("overview");

    // Derived state for current plan details (price, description)
    let currentPlanInfo = $derived(
        availablePlans.find((p) => p.slug === subscription?.plan_slug),
    );

    onMount(async () => {
        try {
            const [subRes, plansRes, invoicesRes] = await Promise.all([
                api.plans.getSubscriptionDetails(),
                api.plans.list(),
                api.payment.listInvoices(),
            ]);
            subscription = subRes;
            availablePlans = plansRes.filter((p) => p.is_active);
            invoices = invoicesRes;
        } catch (e: any) {
            toast.error("Failed to load subscription details");
        } finally {
            loading = false;
        }
    });

    async function handleUpgrade(plan: any) {
        upgrading = true;
        try {
            const invoice = await api.payment.createInvoiceForPlan(
                plan.id,
                "monthly",
            );
            toast.success("Invoice created");
            goto(`/pay/${invoice.id}`);
        } catch (e: any) {
            toast.error(e.message || "Failed to create invoice");
            upgrading = false;
        }
    }

    function formatBytes(bytes: number) {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
    }

    function calculatePercent(used: number, limit: number | null) {
        if (!limit) return 0;
        return Math.min(100, (used / limit) * 100);
    }

    function formatCurrency(amount: number) {
        return new Intl.NumberFormat("id-ID", {
            style: "currency",
            currency: "IDR",
        }).format(amount);
    }

    // Helper to get feature highlights based on slug (Mocking feature list for UI)
    function getPlanFeatures(slug: string) {
        switch (slug) {
            case "free":
                return [
                    "Community Support",
                    "Basic Analytics",
                    "Subdomain Only",
                ];
            case "pro":
                return [
                    "Priority Support",
                    "Advanced Analytics",
                    "Custom Domain",
                    "Remove Branding",
                ];
            case "enterprise":
                return [
                    "24/7 Dedicated Support",
                    "Audit Logs",
                    "Custom Domain",
                    "SSO & Security",
                    "API Access",
                ];
            default:
                return [];
        }
    }

    const invoiceColumns = [
        { key: "invoice_number", label: "Invoice #", sortable: true },
        { key: "description", label: "Description", sortable: true },
        { key: "amount", label: "Amount", sortable: true },
        { key: "status", label: "Status", sortable: true },
        { key: "due_date", label: "Due Date", sortable: true },
        { key: "actions", label: "Actions", align: "right" },
    ];
</script>

<div class="subscription-page" in:fade>
    <div class="tabs">
        <button
            class="tab-btn"
            class:active={activeTab === "overview"}
            onclick={() => (activeTab = "overview")}
        >
            Overview
        </button>
        <button
            class="tab-btn"
            class:active={activeTab === "plans"}
            onclick={() => (activeTab = "plans")}
        >
            Available Plans
        </button>
        <button
            class="tab-btn"
            class:active={activeTab === "history"}
            onclick={() => (activeTab = "history")}
        >
            Payment History
        </button>
    </div>

    {#if loading}
        <div class="loading-state">
            <div class="spinner"></div>
            <p>Loading details...</p>
        </div>
    {:else if subscription}
        {#if activeTab === "overview"}
            <div class="content-grid fade-in">
                <!-- Detailed Current Plan Card -->
                <div class="card plan-detail-card">
                    <div class="detail-header">
                        <div class="plan-title-row">
                            <div class="icon-box">
                                <Icon name="credit-card" size={24} />
                            </div>
                            <div>
                                <h2>{subscription.plan_name} Plan</h2>
                                <p class="plan-desc">
                                    {currentPlanInfo?.description ||
                                        "Current active subscription"}
                                </p>
                            </div>
                        </div>
                        <div class="plan-meta">
                            {#if currentPlanInfo && currentPlanInfo.price_monthly > 0}
                                <div class="price-tag">
                                    <span class="currency">$</span>
                                    <span class="amount"
                                        >{currentPlanInfo.price_monthly}</span
                                    >
                                    <span class="period">/ month</span>
                                </div>
                            {:else}
                                <div class="price-tag free">Free</div>
                            {/if}
                            <span class="status-pill active"
                                >{subscription.status}</span
                            >
                        </div>
                    </div>

                    <div class="detail-body">
                        <!-- Left Column: Usage -->
                        <div class="usage-section">
                            <h3>Resource Usage</h3>

                            <div class="usage-item">
                                <div class="usage-label">
                                    <span class="u-title"
                                        ><Icon name="folder" size={14} /> Storage</span
                                    >
                                    <span class="u-val"
                                        >{formatBytes(
                                            subscription.storage_usage,
                                        )} / {subscription.storage_limit
                                            ? formatBytes(
                                                  subscription.storage_limit,
                                              )
                                            : "Unlimited"}</span
                                    >
                                </div>
                                <div class="progress-container">
                                    <div
                                        class="progress-bar"
                                        style="width: {calculatePercent(
                                            subscription.storage_usage,
                                            subscription.storage_limit,
                                        )}%"
                                        class:warning={calculatePercent(
                                            subscription.storage_usage,
                                            subscription.storage_limit,
                                        ) > 80}
                                        class:danger={calculatePercent(
                                            subscription.storage_usage,
                                            subscription.storage_limit,
                                        ) >= 100}
                                    ></div>
                                </div>
                            </div>

                            <div class="usage-item">
                                <div class="usage-label">
                                    <span class="u-title"
                                        ><Icon name="users" size={14} /> Team Members</span
                                    >
                                    <span class="u-val"
                                        >{subscription.member_usage} / {subscription.member_limit ||
                                            "Unlimited"}</span
                                    >
                                </div>
                                <div class="progress-container">
                                    <div
                                        class="progress-bar"
                                        style="width: {calculatePercent(
                                            subscription.member_usage,
                                            subscription.member_limit,
                                        )}%"
                                    ></div>
                                </div>
                            </div>
                        </div>

                        <div class="vertical-divider"></div>

                        <!-- Right Column: Info & Features -->
                        <div class="info-section">
                            <h3>Billing Details</h3>
                            <div class="info-grid">
                                <div class="info-item">
                                    <span class="info-label">Active Until</span>
                                    {#if subscription.current_period_end}
                                        <span
                                            >{new Date(
                                                subscription.current_period_end,
                                            ).toLocaleDateString()}</span
                                        >
                                    {:else}
                                        <span>Lifetime</span>
                                    {/if}
                                </div>
                                <div class="info-item">
                                    <span class="info-label">Billing Cycle</span
                                    >
                                    <span
                                        >{currentPlanInfo?.price_yearly > 0
                                            ? "Monthly / Yearly"
                                            : "Free Tier"}</span
                                    >
                                </div>
                            </div>

                            <h3 class="mt-4">Includes</h3>
                            <ul class="feature-list">
                                {#each getPlanFeatures(subscription.plan_slug) as feature}
                                    <li>
                                        <Icon
                                            name="check"
                                            size={14}
                                            class="check-icon"
                                        />
                                        {feature}
                                    </li>
                                {/each}
                            </ul>
                        </div>
                    </div>
                </div>
            </div>
        {:else if activeTab === "plans"}
            <div class="plans-comparison fade-in">
                <h3>Select a Plan</h3>
                <div class="plans-grid">
                    {#each availablePlans as plan}
                        <div
                            class="plan-option"
                            class:active={plan.slug === subscription.plan_slug}
                        >
                            <div class="option-header">
                                <h4>{plan.name}</h4>
                                <div class="price">
                                    <span class="currency">$</span>
                                    <span class="amount"
                                        >{plan.price_monthly}</span
                                    >
                                    <span class="period">/mo</span>
                                </div>
                            </div>
                            <p class="desc">{plan.description || ""}</p>

                            <ul class="mini-features">
                                {#each getPlanFeatures(plan.slug) as feat}
                                    <li>• {feat}</li>
                                {/each}
                            </ul>

                            {#if plan.slug === subscription.plan_slug}
                                <button
                                    class="btn btn-secondary w-full"
                                    disabled>Current Plan</button
                                >
                            {:else}
                                <button
                                    class="btn btn-outline w-full"
                                    onclick={() => handleUpgrade(plan)}
                                    disabled={upgrading}
                                >
                                    {upgrading ? "..." : "Upgrade"}
                                </button>
                            {/if}
                        </div>
                    {/each}
                </div>
            </div>
        {:else if activeTab === "history"}
            <div class="history-tab fade-in">
                <div class="card content-card">
                    <Table
                        {loading}
                        data={invoices}
                        columns={invoiceColumns}
                        searchable={true}
                        searchPlaceholder="Search invoices..."
                    >
                        {#snippet cell({ item, column })}
                            {#if column.key === "amount"}
                                {formatCurrency(item.amount)}
                            {:else if column.key === "status"}
                                <span class="status-pill {item.status}"
                                    >{item.status}</span
                                >
                            {:else if column.key === "due_date"}
                                {new Date(
                                    item[column.key],
                                ).toLocaleDateString()}
                            {:else if column.key === "actions"}
                                <div class="actions">
                                    {#if item.status === "pending"}
                                        <a
                                            href="/pay/{item.id}"
                                            class="btn btn-primary btn-sm"
                                        >
                                            Pay
                                        </a>
                                    {:else}
                                        <a
                                            href="/pay/{item.id}"
                                            class="action-btn"
                                            title="View Details"
                                        >
                                            <Icon name="eye" size={18} />
                                        </a>
                                    {/if}
                                </div>
                            {:else}
                                {item[column.key]}
                            {/if}
                        {/snippet}
                    </Table>
                </div>
            </div>
        {/if}
    {/if}
</div>

<style>
    .subscription-page {
        padding: 1.5rem;
        max-width: 1400px;
        margin: 0 auto;
        --glass: rgba(255, 255, 255, 0.04);
        --glass-border: rgba(255, 255, 255, 0.08);
        --accent-indigo: #6366f1;
        --accent-emerald: #10b981;
    }

    /* Tabs */
    .tabs {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 1.5rem;
        padding: 0.4rem;
        border-radius: 14px;
        border: 1px solid var(--glass-border);
        background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
        box-shadow: 0 10px 28px rgba(0, 0, 0, 0.25);
        flex-wrap: wrap;
    }
    .tab-btn {
        padding: 0.65rem 1rem;
        background: transparent;
        border: 1px solid transparent;
        border-radius: 12px;
        color: var(--text-secondary);
        font-weight: 700;
        cursor: pointer;
        transition: all 0.2s;
        white-space: nowrap;
    }
    .tab-btn:hover {
        color: var(--text-primary);
        background: rgba(99, 102, 241, 0.08);
    }
    .tab-btn.active {
        color: var(--text-primary);
        background: radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.16), transparent 60%);
        border-color: rgba(99, 102, 241, 0.35);
    }

    /* Plan Detail Card */
    .plan-detail-card {
        background: radial-gradient(circle at 15% 15%, rgba(99, 102, 241, 0.14), transparent 60%),
            linear-gradient(145deg, var(--bg-surface), #0b0c10);
        border: 1px solid var(--glass-border);
        border-radius: 18px;
        overflow: hidden;
        box-shadow: 0 12px 32px rgba(0, 0, 0, 0.3);
    }

    .detail-header {
        padding: 1.75rem;
        border-bottom: 1px solid var(--glass-border);
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: rgba(255, 255, 255, 0.015);
    }

    .plan-title-row {
        display: flex;
        align-items: center;
        gap: 1rem;
    }
    .icon-box {
        width: 48px;
        height: 48px;
        background: radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.22), transparent 60%),
            rgba(255, 255, 255, 0.04);
        color: var(--accent-indigo);
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: 1px solid rgba(99, 102, 241, 0.25);
    }
    .plan-title-row h2 {
        margin: 0;
        font-size: 1.4rem;
    }
    .plan-desc {
        margin: 0;
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .plan-meta {
        display: flex;
        align-items: center;
        gap: 1.5rem;
    }
    .price-tag {
        display: flex;
        align-items: baseline;
    }
    .price-tag .amount {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary);
    }
    .price-tag .currency {
        font-size: 1rem;
        margin-right: 2px;
    }
    .price-tag .period {
        font-size: 0.85rem;
        color: var(--text-secondary);
        margin-left: 4px;
    }
    .price-tag.free {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--color-success, #10b981);
    }

    .detail-body {
        display: flex;
        padding: 1.75rem;
        gap: 2.5rem;
    }

    .usage-section,
    .info-section {
        flex: 1;
    }
    .usage-section h3,
    .info-section h3 {
        font-size: 1rem;
        font-weight: 600;
        margin: 0 0 1.5rem 0;
        color: var(--text-primary);
    }
    .mt-4 {
        margin-top: 2rem !important;
    }

    .vertical-divider {
        width: 1px;
        background: var(--glass-border);
    }

    /* Usage Items */
    .usage-item {
        margin-bottom: 1.5rem;
    }
    .usage-label {
        display: flex;
        justify-content: space-between;
        font-size: 0.9rem;
        margin-bottom: 0.5rem;
    }
    .u-title {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-weight: 500;
    }
    .u-val {
        color: var(--text-secondary);
    }

    .progress-container {
        height: 8px;
        background: rgba(255, 255, 255, 0.06);
        border-radius: 4px;
        overflow: hidden;
    }
    .progress-bar {
        height: 100%;
        background: var(--color-primary);
        border-radius: 4px;
        transition: width 0.5s ease-out;
    }
    .progress-bar.warning {
        background: #f59e0b;
    }
    .progress-bar.danger {
        background: #ef4444;
    }

    /* Info Grid */
    .info-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1.5rem;
    }
    .info-item {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    .info-item .info-label {
        font-size: 0.8rem;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        font-weight: 600;
    }
    .info-item span {
        font-weight: 600;
        color: var(--text-primary);
    }

    /* Feature List */
    .feature-list {
        list-style: none;
        padding: 0;
        margin: 0;
        display: grid;
        grid-template-columns: 1fr;
        gap: 0.75rem;
    }
    .feature-list li {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        font-size: 0.95rem;
        color: var(--text-secondary);
    }
    /* Global styles might not support :global(.check-icon) without explicit scope, usually Icon component handles it */

    /* Plans Grid (Tab 2) */
    .plans-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: 1.5rem;
    }
    .plan-option {
        background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
        border: 1px solid var(--glass-border);
        border-radius: 16px;
        padding: 1.5rem;
        display: flex;
        flex-direction: column;
        transition: all 0.2s;
        box-shadow: 0 10px 28px rgba(0, 0, 0, 0.22);
    }
    .plan-option:hover {
        transform: translateY(-2px);
        border-color: rgba(99, 102, 241, 0.35);
        box-shadow: 0 14px 32px rgba(99, 102, 241, 0.18);
    }
    .plan-option.active {
        border-color: rgba(99, 102, 241, 0.35);
        background: radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.14), transparent 60%),
            linear-gradient(145deg, var(--bg-surface), #0b0c10);
    }
    .option-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 1rem;
    }
    .option-header h4 {
        margin: 0;
        font-size: 1.2rem;
    }
    .mini-features {
        list-style: none;
        padding: 0;
        margin: 0 0 1.5rem 0;
        font-size: 0.85rem;
        color: var(--text-secondary);
        line-height: 1.6;
    }

    /* Common */
    .status-pill {
        padding: 0.2rem 0.6rem;
        border-radius: 12px;
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
    }
    .status-pill.active {
        background: rgba(34, 197, 94, 0.1);
        color: #22c55e;
    }

    .btn {
        padding: 0.6rem 1rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        border: none;
        text-decoration: none;
        display: inline-flex;
        align-items: center;
        justify-content: center;
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
    .btn-outline {
        background: transparent;
        border: 1px solid var(--glass-border);
        color: var(--text-primary);
    }
    .btn-outline:hover {
        background: rgba(99, 102, 241, 0.12);
        border-color: rgba(99, 102, 241, 0.35);
    }
    .w-full {
        width: 100%;
    }
    .loading-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 4rem;
        background: var(--glass);
        border: 1px solid var(--glass-border);
        border-radius: 16px;
    }
    .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
        margin-bottom: 1rem;
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
        border: 1px solid var(--glass-border);
        background: rgba(255, 255, 255, 0.04);
        color: var(--text-secondary);
        cursor: pointer;
        border-radius: 10px;
        transition: all 0.2s;
    }
    .action-btn:hover {
        background: rgba(99, 102, 241, 0.12);
        color: var(--text-primary);
        border-color: rgba(99, 102, 241, 0.35);
    }
    .content-card {
        padding: 0;
        overflow: hidden;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    @media (max-width: 768px) {
        .subscription-page {
            padding: 1rem;
        }

        .detail-body {
            flex-direction: column;
            gap: 2rem;
        }
        .vertical-divider {
            width: 100%;
            height: 1px;
        }
    }

    /* Light theme adjustments */
    :global([data-theme="light"]) .tabs {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 10px 28px rgba(0, 0, 0, 0.08),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }
    :global([data-theme="light"]) .tab-btn:hover {
        background: rgba(99, 102, 241, 0.1);
    }
    :global([data-theme="light"]) .tab-btn.active {
        background: rgba(99, 102, 241, 0.12);
        border-color: rgba(99, 102, 241, 0.25);
        color: #111827;
    }
    :global([data-theme="light"]) .plan-detail-card {
        background: radial-gradient(circle at 15% 15%, rgba(99, 102, 241, 0.08), transparent 60%),
            linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 12px 32px rgba(0, 0, 0, 0.08),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }
    :global([data-theme="light"]) .detail-header {
        background: rgba(0, 0, 0, 0.02);
        border-bottom-color: rgba(0, 0, 0, 0.06);
    }
    :global([data-theme="light"]) .icon-box {
        background: rgba(99, 102, 241, 0.08);
        border-color: rgba(99, 102, 241, 0.18);
        color: #4f46e5;
    }
    :global([data-theme="light"]) .progress-container {
        background: rgba(0, 0, 0, 0.06);
    }
    :global([data-theme="light"]) .plan-option {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow: 0 10px 28px rgba(0, 0, 0, 0.08);
    }
    :global([data-theme="light"]) .plan-option.active {
        background: rgba(99, 102, 241, 0.08);
        border-color: rgba(99, 102, 241, 0.22);
    }
    :global([data-theme="light"]) .btn-outline {
        border-color: rgba(0, 0, 0, 0.1);
        color: #111827;
    }
    :global([data-theme="light"]) .action-btn {
        background: rgba(0, 0, 0, 0.02);
        border-color: rgba(0, 0, 0, 0.08);
        color: #475569;
    }
    :global([data-theme="light"]) .action-btn:hover {
        background: rgba(99, 102, 241, 0.12);
        border-color: rgba(99, 102, 241, 0.25);
        color: #111827;
    }
</style>
