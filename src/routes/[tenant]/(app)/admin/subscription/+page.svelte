<script lang="ts">
    import { user, can } from "$lib/stores/auth";
    import { onMount } from "svelte";
    import { api, type TenantSubscriptionDetails, type Invoice } from "$lib/api/client";
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
    let activeTab = $state<'overview' | 'plans' | 'history'>('overview');

    // Derived state for current plan details (price, description)
    let currentPlanInfo = $derived(
        availablePlans.find(p => p.slug === subscription?.plan_slug)
    );

    onMount(async () => {
        try {
            const [subRes, plansRes, invoicesRes] = await Promise.all([
                api.plans.getSubscriptionDetails(),
                api.plans.list(),
                api.payment.listInvoices()
            ]);
            subscription = subRes;
            availablePlans = plansRes.filter(p => p.is_active);
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
            const invoice = await api.payment.createInvoiceForPlan(plan.id, "monthly");
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
        return new Intl.NumberFormat('id-ID', {
            style: 'currency',
            currency: 'IDR'
        }).format(amount);
    }

    // Helper to get feature highlights based on slug (Mocking feature list for UI)
    function getPlanFeatures(slug: string) {
        switch (slug) {
            case 'free': return ['Community Support', 'Basic Analytics', 'Subdomain Only'];
            case 'pro': return ['Priority Support', 'Advanced Analytics', 'Custom Domain', 'Remove Branding'];
            case 'enterprise': return ['24/7 Dedicated Support', 'Audit Logs', 'Custom Domain', 'SSO & Security', 'API Access'];
            default: return [];
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
    <header class="page-header">
        <h1>Subscription & Billing</h1>
        <p class="subtitle">Manage your plan and monitor resource usage</p>
    </header>

    <div class="tabs">
        <button 
            class="tab-btn" 
            class:active={activeTab === 'overview'} 
            onclick={() => activeTab = 'overview'}
        >
            Overview
        </button>
        <button 
            class="tab-btn" 
            class:active={activeTab === 'plans'} 
            onclick={() => activeTab = 'plans'}
        >
            Available Plans
        </button>
        <button 
            class="tab-btn" 
            class:active={activeTab === 'history'} 
            onclick={() => activeTab = 'history'}
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
        {#if activeTab === 'overview'}
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
                                <p class="plan-desc">{currentPlanInfo?.description || 'Current active subscription'}</p>
                            </div>
                        </div>
                        <div class="plan-meta">
                            {#if currentPlanInfo && currentPlanInfo.price_monthly > 0}
                                <div class="price-tag">
                                    <span class="currency">$</span>
                                    <span class="amount">{currentPlanInfo.price_monthly}</span>
                                    <span class="period">/ month</span>
                                </div>
                            {:else}
                                <div class="price-tag free">Free</div>
                            {/if}
                            <span class="status-pill active">{subscription.status}</span>
                        </div>
                    </div>

                    <div class="detail-body">
                        <!-- Left Column: Usage -->
                        <div class="usage-section">
                            <h3>Resource Usage</h3>
                            
                            <div class="usage-item">
                                <div class="usage-label">
                                    <span class="u-title"><Icon name="folder" size={14}/> Storage</span>
                                    <span class="u-val">{formatBytes(subscription.storage_usage)} / {subscription.storage_limit ? formatBytes(subscription.storage_limit) : "Unlimited"}</span>
                                </div>
                                <div class="progress-container">
                                    <div 
                                        class="progress-bar" 
                                        style="width: {calculatePercent(subscription.storage_usage, subscription.storage_limit)}%"
                                        class:warning={calculatePercent(subscription.storage_usage, subscription.storage_limit) > 80}
                                        class:danger={calculatePercent(subscription.storage_usage, subscription.storage_limit) >= 100}
                                    ></div>
                                </div>
                            </div>

                            <div class="usage-item">
                                <div class="usage-label">
                                    <span class="u-title"><Icon name="users" size={14}/> Team Members</span>
                                    <span class="u-val">{subscription.member_usage} / {subscription.member_limit || "Unlimited"}</span>
                                </div>
                                <div class="progress-container">
                                    <div 
                                        class="progress-bar" 
                                        style="width: {calculatePercent(subscription.member_usage, subscription.member_limit)}%"
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
                                    <label>Active Until</label>
                                    {#if subscription.current_period_end}
                                        <span>{new Date(subscription.current_period_end).toLocaleDateString()}</span>
                                    {:else}
                                        <span>Lifetime</span>
                                    {/if}
                                </div>
                                <div class="info-item">
                                    <label>Billing Cycle</label>
                                    <span>{currentPlanInfo?.price_yearly > 0 ? "Monthly / Yearly" : "Free Tier"}</span>
                                </div>
                            </div>

                            <h3 class="mt-4">Includes</h3>
                            <ul class="feature-list">
                                {#each getPlanFeatures(subscription.plan_slug) as feature}
                                    <li><Icon name="check" size={14} class="check-icon"/> {feature}</li>
                                {/each}
                            </ul>
                        </div>
                    </div>
                </div>
            </div>
        {:else if activeTab === 'plans'}
            <div class="plans-comparison fade-in">
                <h3>Select a Plan</h3>
                <div class="plans-grid">
                    {#each availablePlans as plan}
                        <div class="plan-option" class:active={plan.slug === subscription.plan_slug}>
                            <div class="option-header">
                                <h4>{plan.name}</h4>
                                <div class="price">
                                    <span class="currency">$</span>
                                    <span class="amount">{plan.price_monthly}</span>
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
                                <button class="btn btn-secondary w-full" disabled>Current Plan</button>
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
        {:else if activeTab === 'history'}
            <div class="history-tab fade-in">
                <div class="card content-card">
                    <Table
                        {loading}
                        data={invoices}
                        columns={invoiceColumns}
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
                                            Pay
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
        {/if}
    {/if}
</div>

<style>
    .subscription-page {
        padding: 2rem;
        max-width: 1200px;
        margin: 0 auto;
    }

    .page-header { margin-bottom: 2rem; }
    h1 { font-size: 1.85rem; font-weight: 700; margin: 0 0 0.5rem 0; }
    .subtitle { color: var(--text-secondary); }

    /* Tabs */
    .tabs {
        display: flex;
        gap: 1rem;
        border-bottom: 1px solid var(--border-color);
        margin-bottom: 2rem;
    }
    .tab-btn {
        padding: 0.75rem 1rem;
        background: transparent;
        border: none;
        border-bottom: 2px solid transparent;
        color: var(--text-secondary);
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
    }
    .tab-btn:hover { color: var(--text-primary); }
    .tab-btn.active { color: var(--color-primary); border-bottom-color: var(--color-primary); }

    /* Plan Detail Card */
    .plan-detail-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 16px;
        overflow: hidden;
    }

    .detail-header {
        padding: 2rem;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: var(--bg-surface);
    }

    .plan-title-row { display: flex; align-items: center; gap: 1rem; }
    .icon-box {
        width: 48px; height: 48px;
        background: var(--color-primary-subtle); color: var(--color-primary);
        border-radius: 12px;
        display: flex; align-items: center; justify-content: center;
    }
    .plan-title-row h2 { margin: 0; font-size: 1.4rem; }
    .plan-desc { margin: 0; color: var(--text-secondary); font-size: 0.9rem; }

    .plan-meta { display: flex; align-items: center; gap: 1.5rem; }
    .price-tag { display: flex; align-items: baseline; }
    .price-tag .amount { font-size: 1.5rem; font-weight: 700; color: var(--text-primary); }
    .price-tag .currency { font-size: 1rem; margin-right: 2px; }
    .price-tag .period { font-size: 0.85rem; color: var(--text-secondary); margin-left: 4px; }
    .price-tag.free { font-size: 1.5rem; font-weight: 700; color: var(--color-success, #10b981); }

    .detail-body {
        display: flex;
        padding: 2rem;
        gap: 3rem;
    }

    .usage-section, .info-section { flex: 1; }
    .usage-section h3, .info-section h3 {
        font-size: 1rem; font-weight: 600; margin: 0 0 1.5rem 0; color: var(--text-primary);
    }
    .mt-4 { margin-top: 2rem !important; }

    .vertical-divider {
        width: 1px;
        background: var(--border-color);
    }

    /* Usage Items */
    .usage-item { margin-bottom: 1.5rem; }
    .usage-label { display: flex; justify-content: space-between; font-size: 0.9rem; margin-bottom: 0.5rem; }
    .u-title { display: flex; align-items: center; gap: 0.5rem; font-weight: 500; }
    .u-val { color: var(--text-secondary); }

    .progress-container {
        height: 8px; background: var(--bg-tertiary); border-radius: 4px; overflow: hidden;
    }
    .progress-bar {
        height: 100%; background: var(--color-primary); border-radius: 4px;
        transition: width 0.5s ease-out;
    }
    .progress-bar.warning { background: #f59e0b; }
    .progress-bar.danger { background: #ef4444; }

    /* Info Grid */
    .info-grid {
        display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem;
    }
    .info-item { display: flex; flex-direction: column; gap: 0.25rem; }
    .info-item label { font-size: 0.8rem; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600; }
    .info-item span { font-weight: 600; color: var(--text-primary); }

    /* Feature List */
    .feature-list { list-style: none; padding: 0; margin: 0; display: grid; grid-template-columns: 1fr; gap: 0.75rem; }
    .feature-list li { display: flex; align-items: center; gap: 0.75rem; font-size: 0.95rem; color: var(--text-secondary); }
    /* Global styles might not support :global(.check-icon) without explicit scope, usually Icon component handles it */
    
    /* Plans Grid (Tab 2) */
    .plans-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; }
    .plan-option {
        background: var(--bg-surface); border: 1px solid var(--border-color); border-radius: 12px; padding: 1.5rem;
        display: flex; flex-direction: column; transition: all 0.2s;
    }
    .plan-option.active { border-color: var(--color-primary); background: var(--color-primary-subtle); }
    .option-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1rem; }
    .option-header h4 { margin: 0; font-size: 1.2rem; }
    .mini-features { list-style: none; padding: 0; margin: 0 0 1.5rem 0; font-size: 0.85rem; color: var(--text-secondary); line-height: 1.6; }

    /* Common */
    .status-pill { padding: 0.2rem 0.6rem; border-radius: 12px; font-size: 0.75rem; font-weight: 700; text-transform: uppercase; }
    .status-pill.active { background: rgba(34, 197, 94, 0.1); color: #22c55e; }
    
    .btn { padding: 0.6rem 1rem; border-radius: 8px; font-weight: 600; cursor: pointer; border: none; text-decoration: none; display: inline-flex; align-items: center; justify-content: center; }
    .btn-sm { padding: 0.4rem 0.8rem; font-size: 0.85rem; }
    .btn-primary { background: var(--color-primary); color: white; }
    .btn-secondary { background: var(--bg-tertiary); color: var(--text-primary); }
    .btn-outline { background: transparent; border: 1px solid var(--border-color); color: var(--text-primary); }
    .btn-outline:hover { background: var(--bg-hover); border-color: var(--color-primary); }
    .w-full { width: 100%; }
    .loading-state { display: flex; flex-direction: column; align-items: center; padding: 4rem; }
    .spinner { width: 32px; height: 32px; border: 3px solid var(--border-color); border-top-color: var(--color-primary); border-radius: 50%; animation: spin 1s linear infinite; margin-bottom: 1rem; }
    .actions { display: flex; gap: 0.5rem; justify-content: flex-end; }
    .action-btn { width: 32px; height: 32px; display: flex; align-items: center; justify-content: center; border: none; background: transparent; color: var(--text-secondary); cursor: pointer; border-radius: 6px; }
    .action-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
    .content-card { padding: 0; overflow: hidden; }

    @keyframes spin { to { transform: rotate(360deg); } }
    
    @media (max-width: 768px) {
        .detail-body { flex-direction: column; gap: 2rem; }
        .vertical-divider { width: 100%; height: 1px; }
    }
</style>