<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import { api } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import { toast } from "$lib/stores/toast";

    let id = $page.params.id as string;
    let isNew = id === "new";
    let loading = true;
    let saving = false;

    // Data Models
    let planData: any = {
        name: "",
        slug: "",
        description: "",
        price_monthly: 0,
        price_yearly: 0,
        is_active: true,
        is_default: false,
        sort_order: 0,
    };

    let features: any[] = [];
    let planFeatures: Record<string, string> = {}; // feature_id -> value

    let activeTab = "general";

    onMount(async () => {
        try {
            // Load all available features definition
            features = await api.plans.listFeatures();

            if (!isNew) {
                const plan = await api.plans.get(id);
                planData = plan;

                // Map plan features to simple key-value
                if (plan.features) {
                    plan.features.forEach((f: any) => {
                        planFeatures[f.feature_id] = f.value;
                    });
                }
            } else {
                // Set defaults for new plan
                features.forEach((f) => {
                    planFeatures[f.id] = f.default_value;
                });
            }
        } catch (e: any) {
            toast.error(e.message || "Failed to load plan data");
            goto("/superadmin/plans");
        } finally {
            loading = false;
        }
    });

    async function savePlan() {
        saving = true;
        try {
            if (isNew) {
                // Create Plan
                const created = await api.plans.create(
                    planData.name,
                    planData.slug,
                    planData.description,
                    planData.price_monthly,
                    planData.price_yearly,
                    planData.is_active,
                    planData.is_default,
                    planData.sort_order,
                );
                id = created.id;
                isNew = false;

                // Save features
                await saveFeatures(created.id);

                toast.success("Plan created successfully");
                goto("/superadmin/plans");
            } else {
                // Update Plan
                await api.plans.update(
                    id,
                    planData.name,
                    planData.slug,
                    planData.description,
                    planData.price_monthly,
                    planData.price_yearly,
                    planData.is_active,
                    planData.is_default,
                    planData.sort_order,
                );
                await saveFeatures(id);

                toast.success("Plan updated successfully");
            }
        } catch (e: any) {
            toast.error(e.message || "Failed to save plan");
        } finally {
            saving = false;
        }
    }

    async function saveFeatures(planId: string) {
        const promises = Object.entries(planFeatures).map(
            ([featureId, value]) => {
                return api.plans.setPlanFeature(
                    planId,
                    featureId,
                    String(value),
                );
            },
        );
        await Promise.all(promises);
    }

    // Helper to generate slug from name
    function onNameChange() {
        if (isNew && planData.name) {
            planData.slug = planData.name
                .toLowerCase()
                .replace(/[^a-z0-9]+/g, "-")
                .replace(/(^-|-$)/g, "");
        }
    }
</script>

<svelte:head>
    <title>{isNew ? "New Plan" : planData.name} | Superadmin</title>
</svelte:head>

<div class="plan-detail-page">
    <div class="page-header">
        <button class="btn-back" onclick={() => goto("/superadmin/plans")}>
            <Icon name="arrow-left" size={20} />
            Back
        </button>
        <div class="header-content">
            <h1>{isNew ? "Create New Plan" : `Edit ${planData.name}`}</h1>
            <div class="actions">
                <button
                    class="btn btn-secondary"
                    onclick={() => goto("/superadmin/plans")}>Cancel</button
                >
                <button
                    class="btn btn-primary"
                    onclick={savePlan}
                    disabled={saving}
                >
                    {#if saving}Saving...{:else}Save Plan{/if}
                </button>
            </div>
        </div>
    </div>

    {#if loading}
        <div class="loading">Loading...</div>
    {:else}
        <div class="tabs">
            <button
                class="tab {activeTab === 'general' ? 'active' : ''}"
                onclick={() => (activeTab = "general")}
            >
                General
            </button>
            <button
                class="tab {activeTab === 'features' ? 'active' : ''}"
                onclick={() => (activeTab = "features")}
            >
                Features & Limits
            </button>
        </div>

        <div class="content-card">
            {#if activeTab === "general"}
                <div class="form-grid fade-in">
                    <div class="form-group">
                        <label for="name">Plan Name</label>
                        <input
                            id="name"
                            type="text"
                            bind:value={planData.name}
                            oninput={onNameChange}
                            placeholder="e.g. Pro Plan"
                        />
                    </div>

                    <div class="form-group">
                        <label for="slug">Slug (Code)</label>
                        <input
                            id="slug"
                            type="text"
                            bind:value={planData.slug}
                            disabled={!isNew}
                            placeholder="e.g. pro-plan"
                        />
                        <small>Unique identifier used in code/API.</small>
                    </div>

                    <div class="form-group full-width">
                        <label for="description">Description</label>
                        <textarea
                            id="description"
                            bind:value={planData.description}
                            rows="2"
                        ></textarea>
                    </div>

                    <div class="form-group">
                        <label for="price_m">Monthly Price ($)</label>
                        <input
                            id="price_m"
                            type="number"
                            step="0.01"
                            bind:value={planData.price_monthly}
                        />
                    </div>

                    <div class="form-group">
                        <label for="price_y">Yearly Price ($)</label>
                        <input
                            id="price_y"
                            type="number"
                            step="0.01"
                            bind:value={planData.price_yearly}
                        />
                    </div>

                    <div class="form-group">
                        <label for="sort_order">Sort Order</label>
                        <input
                            id="sort_order"
                            type="number"
                            bind:value={planData.sort_order}
                        />
                    </div>

                    <div class="form-group toggle-group">
                        <label class="toggle-label">
                            <input
                                type="checkbox"
                                bind:checked={planData.is_active}
                            />
                            Active (Visible to users)
                        </label>
                        <label class="toggle-label">
                            <input
                                type="checkbox"
                                bind:checked={planData.is_default}
                            />
                            Default Plan (for new tenants)
                        </label>
                    </div>
                </div>
            {:else if activeTab === "features"}
                <div class="features-list fade-in">
                    {#each features as feature}
                        <div class="feature-item">
                            <div class="feature-info">
                                <strong>{feature.name}</strong>
                                <p>{feature.description}</p>
                                <small class="code">{feature.code}</small>
                            </div>
                            <div class="feature-control">
                                {#if feature.value_type === "boolean"}
                                    <label class="toggle-switch">
                                        <input
                                            type="checkbox"
                                            checked={planFeatures[
                                                feature.id
                                            ] === "true"}
                                            onchange={(e) =>
                                                (planFeatures[feature.id] = e
                                                    .currentTarget.checked
                                                    ? "true"
                                                    : "false")}
                                        />
                                        <span class="slider"></span>
                                    </label>
                                {:else if feature.value_type === "number"}
                                    <input
                                        type="number"
                                        value={planFeatures[feature.id]}
                                        oninput={(e) =>
                                            (planFeatures[feature.id] =
                                                e.currentTarget.value)}
                                        class="input-sm"
                                    />
                                {:else}
                                    <input
                                        type="text"
                                        value={planFeatures[feature.id]}
                                        oninput={(e) =>
                                            (planFeatures[feature.id] =
                                                e.currentTarget.value)}
                                        class="input-sm"
                                    />
                                {/if}
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .plan-detail-page {
        padding: 2rem;
        max-width: 1000px;
        margin: 0 auto;
    }

    .page-header {
        margin-bottom: 2rem;
    }

    .btn-back {
        background: none;
        border: none;
        color: var(--text-secondary);
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: pointer;
        margin-bottom: 1rem;
        padding: 0;
        font-size: 0.9rem;
    }

    .btn-back:hover {
        color: var(--text-primary);
    }

    .header-content {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    h1 {
        font-size: 1.8rem;
        font-weight: 700;
        margin: 0;
        color: var(--text-primary);
    }

    .actions {
        display: flex;
        gap: 1rem;
    }

    .btn {
        padding: 0.6rem 1.2rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        border: none;
        font-size: 0.9rem;
        transition: all 0.2s;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }
    .btn-primary:hover {
        filter: brightness(1.1);
    }
    .btn-primary:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    .btn-secondary {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
    }
    .btn-secondary:hover {
        background: var(--bg-hover);
    }

    .tabs {
        display: flex;
        gap: 1rem;
        margin-bottom: 1.5rem;
        border-bottom: 1px solid var(--border-color);
    }

    .tab {
        background: none;
        border: none;
        padding: 0.8rem 1rem;
        color: var(--text-secondary);
        font-weight: 500;
        cursor: pointer;
        border-bottom: 2px solid transparent;
        transition: all 0.2s;
    }

    .tab.active {
        color: var(--color-primary);
        border-bottom-color: var(--color-primary);
    }

    .content-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 2rem;
        box-shadow: var(--shadow-sm);
    }

    .form-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1.5rem;
    }

    .full-width {
        grid-column: 1 / -1;
    }

    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    label {
        font-size: 0.9rem;
        font-weight: 500;
        color: var(--text-secondary);
    }

    input[type="text"],
    input[type="number"],
    textarea {
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 0.7rem;
        color: var(--text-primary);
        font-size: 0.95rem;
    }

    input:focus,
    textarea:focus {
        outline: none;
        border-color: var(--color-primary);
    }

    small {
        color: var(--text-secondary);
        font-size: 0.8rem;
    }

    .toggle-group {
        flex-direction: row;
        gap: 2rem;
        align-items: center;
        padding-top: 1.5rem;
    }

    .toggle-label {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: pointer;
        color: var(--text-primary);
    }

    /* Features List */
    .features-list {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
    }

    .feature-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem;
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        border-radius: 8px;
    }

    .feature-info {
        display: flex;
        flex-direction: column;
    }

    .feature-info strong {
        color: var(--text-primary);
    }
    .feature-info p {
        margin: 0.2rem 0;
        font-size: 0.85rem;
        color: var(--text-secondary);
    }
    .code {
        font-family: monospace;
        font-size: 0.75rem;
        background: rgba(0, 0, 0, 0.1);
        padding: 2px 4px;
        border-radius: 4px;
        width: fit-content;
    }

    .input-sm {
        width: 120px;
        padding: 0.5rem;
    }

    /* Toggle Switch */
    .toggle-switch {
        position: relative;
        display: inline-block;
        width: 48px;
        height: 24px;
    }
    .toggle-switch input {
        opacity: 0;
        width: 0;
        height: 0;
    }
    .slider {
        position: absolute;
        cursor: pointer;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: #ccc;
        transition: 0.4s;
        border-radius: 34px;
    }
    .slider:before {
        position: absolute;
        content: "";
        height: 18px;
        width: 18px;
        left: 3px;
        bottom: 3px;
        background-color: white;
        transition: 0.4s;
        border-radius: 50%;
    }
    input:checked + .slider {
        background-color: var(--color-primary);
    }
    input:checked + .slider:before {
        transform: translateX(24px);
    }

    .fade-in {
        animation: fadeIn 0.3s ease-out;
    }
    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
</style>
