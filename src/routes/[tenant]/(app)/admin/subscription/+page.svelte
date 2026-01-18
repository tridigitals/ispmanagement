<script lang="ts">
    import { user, can } from "$lib/stores/auth";
    import { onMount } from "svelte";
    import { api, type TenantSubscriptionDetails } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import { fade, fly } from "svelte/transition";
    import { toast } from "svelte-sonner";

    let loading = $state(true);
    let subscription = $state<TenantSubscriptionDetails | null>(null);
    let availablePlans = $state<any[]>([]);

    onMount(async () => {
        try {
            const [subRes, plansRes] = await Promise.all([
                api.plans.getSubscriptionDetails(),
                api.plans.list() // Load all plans for comparison/upgrade
            ]);
            subscription = subRes;
            availablePlans = plansRes.filter(p => p.is_active);
        } catch (e: any) {
            toast.error("Failed to load subscription details");
        } finally {
            loading = false;
        }
    });

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
</script>

<div class="subscription-page" in:fade>
    <header class="page-header">
        <h1>Subscription & Billing</h1>
        <p class="subtitle">Manage your plan and monitor resource usage</p>
    </header>

    {#if loading}
        <div class="loading-state">
            <div class="spinner"></div>
            <p>Loading details...</p>
        </div>
    {:else if subscription}
        <div class="content-grid">
            <!-- Current Plan Card -->
            <div class="card plan-card featured">
                <div class="card-badge">Current Plan</div>
                <div class="plan-info">
                    <div class="icon-circle">
                        <Icon name="credit-card" size={32} />
                    </div>
                    <div class="text">
                        <h2>{subscription.plan_name}</h2>
                        <span class="status-pill active">{subscription.status}</span>
                    </div>
                </div>
                
                <div class="plan-details">
                    {#if subscription.current_period_end}
                        <div class="detail-row">
                            <span>Next Billing Date</span>
                            <strong>{new Date(subscription.current_period_end).toLocaleDateString()}</strong>
                        </div>
                    {/if}
                </div>

                <div class="usage-stats">
                    <div class="usage-item">
                        <div class="usage-label">
                            <span>Storage Usage</span>
                            <span>{formatBytes(subscription.storage_usage)} / {subscription.storage_limit ? formatBytes(subscription.storage_limit) : "Unlimited"}</span>
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
                            <span>Team Members</span>
                            <span>{subscription.member_usage} / {subscription.member_limit || "Unlimited"}</span>
                        </div>
                        <div class="progress-container">
                            <div 
                                class="progress-bar" 
                                style="width: {calculatePercent(subscription.member_usage, subscription.member_limit)}%"
                            ></div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Upgrade/Available Plans -->
            <div class="plans-comparison">
                <h3>Available Plans</h3>
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
                            {#if plan.slug === subscription.plan_slug}
                                <button class="btn btn-secondary w-full" disabled>Your Current Plan</button>
                            {:else}
                                <button class="btn btn-outline w-full">Change to {plan.name}</button>
                            {/if}
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    .subscription-page {
        padding: 2rem;
        max-width: 1200px;
        margin: 0 auto;
    }

    .page-header {
        margin-bottom: 2rem;
    }

    h1 {
        font-size: 1.85rem;
        font-weight: 700;
        margin: 0 0 0.5rem 0;
    }

    .subtitle {
        color: var(--text-secondary);
    }

    .content-grid {
        display: grid;
        grid-template-columns: 1fr;
        gap: 2rem;
    }

    /* Plan Card */
    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 16px;
        padding: 2rem;
        position: relative;
        overflow: hidden;
    }

    .plan-card.featured {
        border-color: var(--color-primary);
        box-shadow: 0 10px 30px rgba(99, 102, 241, 0.1);
    }

    .card-badge {
        position: absolute;
        top: 1rem;
        right: 1rem;
        background: var(--color-primary);
        color: white;
        padding: 0.25rem 0.75rem;
        border-radius: 20px;
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
    }

    .plan-info {
        display: flex;
        align-items: center;
        gap: 1.5rem;
        margin-bottom: 2rem;
    }

    .icon-circle {
        width: 64px;
        height: 64px;
        background: var(--color-primary-subtle);
        color: var(--color-primary);
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .plan-info h2 {
        margin: 0 0 0.25rem 0;
        font-size: 1.5rem;
    }

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

    .usage-stats {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 2rem;
        padding-top: 2rem;
        border-top: 1px solid var(--border-color);
    }

    .usage-item {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .usage-label {
        display: flex;
        justify-content: space-between;
        font-size: 0.9rem;
        font-weight: 500;
    }

    .progress-container {
        height: 8px;
        background: var(--bg-tertiary);
        border-radius: 4px;
        overflow: hidden;
    }

    .progress-bar {
        height: 100%;
        background: var(--color-primary);
        border-radius: 4px;
        transition: width 0.5s ease-out;
    }

    .progress-bar.warning { background: #f59e0b; }
    .progress-bar.danger { background: #ef4444; }

    /* Plans Comparison */
    .plans-comparison {
        margin-top: 1rem;
    }

    .plans-comparison h3 {
        margin-bottom: 1.5rem;
    }

    .plans-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
        gap: 1.5rem;
    }

    .plan-option {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.5rem;
        display: flex;
        flex-direction: column;
        transition: all 0.2s;
    }

    .plan-option.active {
        border-color: var(--color-primary);
        background: var(--color-primary-subtle);
    }

    .option-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 1rem;
    }

    .option-header h4 {
        margin: 0;
        font-size: 1.1rem;
    }

    .price {
        display: flex;
        align-items: baseline;
    }

    .amount {
        font-size: 1.5rem;
        font-weight: 700;
    }

    .period {
        font-size: 0.8rem;
        color: var(--text-secondary);
    }

    .desc {
        font-size: 0.85rem;
        color: var(--text-secondary);
        margin-bottom: 1.5rem;
        flex: 1;
    }

    .btn {
        padding: 0.6rem 1rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
    }

    .btn-secondary { background: var(--bg-tertiary); color: var(--text-primary); }
    .btn-outline { background: transparent; border: 1px solid var(--border-color); color: var(--text-primary); }
    .btn-outline:hover { background: var(--bg-hover); border-color: var(--color-primary); }

    .w-full { width: 100%; }

    .loading-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 4rem;
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

    @keyframes spin { to { transform: rotate(360deg); } }
</style>
