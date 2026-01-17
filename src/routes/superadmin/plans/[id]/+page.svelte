<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import { api } from "$lib/api/client";
    import type { Plan, Feature } from "$lib/api/client";
    import { toast } from "$lib/stores/toast";

    let loading = true;
    let saving = false;
    let planId = $page.params.id;
    let isNew = planId === "new";

    let planForm = {
        name: "",
        slug: "",
        description: "",
        price_monthly: 0,
        price_yearly: 0,
        is_active: true,
        is_default: false,
    };

    let features: Feature[] = [];
    let planFeatures: { [key: string]: any } = {}; // Local state for feature values: { feature_id: value }
    let activeTab: "general" | "features" = "general";

    onMount(async () => {
        try {
            await loadData();
        } catch (e: any) {
            toast.error(e.message || "Failed to load data");
            goto("/superadmin/plans");
        } finally {
            loading = false;
        }
    });

    async function loadData() {
        // 1. Load Feature Definitions
        features = await api.plans.listFeatures();

        // 2. Load Plan Data if editing
        if (!isNew) {
            const result = await api.plans.get(planId);
            if (result) {
                // Determine structure: result might be Plan object directly or { plan: ..., features: ... }
                // Based on get_plan_with_features in backend, it returns { plan: Plan, features: PlanFeatureValue[] }
                const planData = result.plan || result;

                planForm = {
                    name: planData.name,
                    slug: planData.slug,
                    description: planData.description || "",
                    price_monthly: planData.price_monthly,
                    price_yearly: planData.price_yearly,
                    is_active: planData.is_active,
                    is_default: planData.is_default,
                };

                // Initialize planFeatures map
                const currentFeatures = result.features || [];
                currentFeatures.forEach((f: any) => {
                    planFeatures[f.feature_id] = f.value;
                });
            } else {
                throw new Error("Plan not found");
            }
        }
    }

    async function savePlan() {
        if (!planForm.name || !planForm.slug) {
            toast.error("Name and Slug are required");
            return;
        }

        saving = true;
        try {
            let savedPlanId = planId;

            // 1. Save Basic Plan Info
            if (isNew) {
                const newPlan = await api.plans.create(
                    planForm.name,
                    planForm.slug,
                    planForm.description || undefined,
                    planForm.price_monthly,
                    planForm.price_yearly,
                    planForm.is_active,
                    planForm.is_default,
                );
                savedPlanId = newPlan.id;
                toast.success("Plan created");
                // Redirect to edit mode to enable features
                if (isNew) {
                    await goto(`/superadmin/plans/${savedPlanId}`, {
                        replaceState: true,
                    });
                    isNew = false;
                    planId = savedPlanId;
                }
            } else {
                await api.plans.update(
                    planId,
                    planForm.name,
                    planForm.slug,
                    planForm.description || undefined,
                    planForm.price_monthly,
                    planForm.price_yearly,
                    planForm.is_active,
                    planForm.is_default,
                );
                toast.success("Plan updated");
            }

            // 2. Save Features (Parallel)
            // We only save features that have a value in planFeatures map
            const featurePromises = features.map(async (feature) => {
                const value = planFeatures[feature.id];
                // If value is undefined, it means it hasn't been touched, or we can use default.
                // But for "setPlanFeature", we generally only want to call it if we have a specific value to set.
                // However, user might want to set "false" explicitly.
                // Let's rely on what's in planFeatures.
                if (value !== undefined) {
                    await api.plans.setPlanFeature(
                        savedPlanId,
                        feature.id,
                        String(value),
                    );
                }
            });

            await Promise.all(featurePromises);

            // Reload to ensure consistency
            // await loadData();
            // Or just navigate back
            goto("/superadmin/plans");
        } catch (e: any) {
            toast.error(e.message || "Failed to save plan");
            saving = false;
        }
    }

    function updateFeatureValue(featureId: string, value: any) {
        planFeatures[featureId] = value;
    }

    function generateSlug(name: string) {
        return name
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, "-")
            .replace(/^-|-$/g, "");
    }

    function onNameChange(e: Event) {
        if (isNew) {
            const val = (e.target as HTMLInputElement).value;
            planForm.slug = generateSlug(val);
        }
    }
</script>

<div class="page-container">
    <!-- Header -->
    <header class="page-header">
        <div class="header-left">
            <button class="btn-back" on:click={() => goto("/superadmin/plans")}>
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"><path d="m15 18-6-6 6-6" /></svg
                >
                Back to Plans
            </button>
            <h1>{isNew ? "Create Plan" : "Edit Plan"}</h1>
        </div>
        <div class="header-actions">
            <button
                class="btn btn-secondary"
                on:click={() => goto("/superadmin/plans")}>Cancel</button
            >
            <button
                class="btn btn-primary"
                on:click={savePlan}
                disabled={saving}
            >
                {saving ? "Saving..." : "Save Plan"}
            </button>
        </div>
    </header>

    {#if loading}
        <div class="loading">Loading...</div>
    {:else}
        <div class="tabs-nav">
            <button
                class="tab-btn {activeTab === 'general' ? 'active' : ''}"
                on:click={() => (activeTab = "general")}
            >
                General
            </button>
            <button
                class="tab-btn {activeTab === 'features' ? 'active' : ''}"
                on:click={() => (activeTab = "features")}
            >
                Features
            </button>
        </div>

        <div class="tab-content">
            <!-- General Info Tab -->
            {#if activeTab === "general"}
                <div class="card general-section">
                    <div class="form-grid">
                        <div class="form-group full-width">
                            <label for="name">Plan Name</label>
                            <input
                                type="text"
                                id="name"
                                bind:value={planForm.name}
                                on:input={onNameChange}
                                placeholder="e.g. Pro Plan"
                            />
                        </div>

                        <div class="form-group full-width">
                            <label for="slug">Slug / Code</label>
                            <input
                                type="text"
                                id="slug"
                                bind:value={planForm.slug}
                                placeholder="pro_monthly"
                            />
                            <span class="hint"
                                >Unique identifier used in code (e.g. 'premium',
                                'basic')</span
                            >
                        </div>

                        <div class="form-group full-width">
                            <label for="desc">Description</label>
                            <textarea
                                id="desc"
                                bind:value={planForm.description}
                                placeholder="Plan description..."
                            ></textarea>
                        </div>

                        <div class="form-group">
                            <label for="monthly">Monthly Price ($)</label>
                            <input
                                type="number"
                                id="monthly"
                                bind:value={planForm.price_monthly}
                                step="0.01"
                                min="0"
                            />
                        </div>

                        <div class="form-group">
                            <label for="yearly">Yearly Price ($)</label>
                            <input
                                type="number"
                                id="yearly"
                                bind:value={planForm.price_yearly}
                                step="0.01"
                                min="0"
                            />
                        </div>
                    </div>

                    <div class="status-toggles">
                        <div class="toggle-item">
                            <label class="toggle">
                                <input
                                    type="checkbox"
                                    bind:checked={planForm.is_active}
                                />
                                <span class="slider"></span>
                            </label>
                            <span class="label">Active Status</span>
                        </div>

                        <div class="toggle-item">
                            <label class="toggle">
                                <input
                                    type="checkbox"
                                    bind:checked={planForm.is_default}
                                />
                                <span class="slider"></span>
                            </label>
                            <span class="label">Default Plan</span>
                        </div>
                    </div>
                </div>
            {/if}

            <!-- Features Tab -->
            {#if activeTab === "features"}
                <div class="card features-section">
                    {#if isNew}
                        <div class="info-banner">
                            <span class="icon">ℹ️</span>
                            <p>
                                Please save the plan first to configure
                                features.
                            </p>
                        </div>
                    {:else}
                        <div class="features-list">
                            {#each features as feature}
                                <div class="feature-row">
                                    <div class="feature-meta">
                                        <span class="feature-name"
                                            >{feature.name}</span
                                        >
                                        <span class="feature-code"
                                            >{feature.code}</span
                                        >
                                        <span class="feature-desc"
                                            >{feature.description || ""}</span
                                        >
                                    </div>
                                    <div class="feature-control">
                                        {#if feature.value_type === "boolean"}
                                            <label class="toggle">
                                                <input
                                                    type="checkbox"
                                                    checked={planFeatures[
                                                        feature.id
                                                    ] === "true"}
                                                    on:change={(e) =>
                                                        updateFeatureValue(
                                                            feature.id,
                                                            e.currentTarget
                                                                .checked
                                                                ? "true"
                                                                : "false",
                                                        )}
                                                />
                                                <span class="slider"></span>
                                            </label>
                                        {:else}
                                            <input
                                                type="text"
                                                class="input-sm"
                                                value={planFeatures[
                                                    feature.id
                                                ] || ""}
                                                on:input={(e) =>
                                                    updateFeatureValue(
                                                        feature.id,
                                                        e.currentTarget.value,
                                                    )}
                                                placeholder={feature.default_value}
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
    {/if}
</div>

<style>
    .page-container {
        padding: 2rem;
        max-width: 1200px;
        margin: 0 auto;
    }

    .page-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 2rem;
    }

    .header-left h1 {
        font-size: 1.75rem;
        font-weight: 700;
        margin: 0.5rem 0 0;
        color: var(--text-primary);
    }

    .btn-back {
        background: none;
        border: none;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--text-secondary);
        font-size: 0.9rem;
        cursor: pointer;
        padding: 0;
    }

    .btn-back:hover {
        color: var(--text-primary);
    }

    .header-actions {
        display: flex;
        gap: 1rem;
    }

    .header-actions button {
        min-width: 140px;
        justify-content: center;
    }

    /* TABS */
    .tabs-nav {
        display: flex;
        gap: 1.5rem;
        border-bottom: 2px solid var(--border-subtle);
        margin-bottom: 2rem;
    }

    .tab-btn {
        background: none;
        border: none;
        padding: 0.75rem 0.5rem;
        font-size: 1rem;
        font-weight: 500;
        color: var(--text-secondary);
        cursor: pointer;
        position: relative;
        transition: color 0.2s;
    }

    .tab-btn:hover {
        color: var(--text-primary);
    }

    .tab-btn.active {
        color: var(--color-primary);
        font-weight: 600;
    }

    .tab-btn.active::after {
        content: "";
        position: absolute;
        bottom: -2px;
        left: 0;
        width: 100%;
        height: 2px;
        background: var(--color-primary);
        border-radius: 2px 2px 0 0;
    }

    .tab-content {
        animation: fadeIn 0.2s ease-out;
    }

    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: translateY(5px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }

    /* Cards */
    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg);
        padding: 2rem; /* Increased padding */
        box-shadow: var(--shadow-sm);
        max-width: 800px; /* Limit width for cleaner look on wide screens */
    }

    /* Form Styles */
    .form-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1.5rem;
    }

    .full-width {
        grid-column: span 2;
    }

    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }

    label {
        font-size: 0.85rem;
        font-weight: 500;
        color: var(--text-secondary);
    }

    input[type="text"],
    input[type="number"],
    textarea {
        background: var(--bg-input);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        padding: 0.75rem;
        color: var(--text-primary);
        font-size: 0.95rem;
        transition: all 0.2s;
        width: 100%;
        box-sizing: border-box;
    }

    textarea {
        min-height: 100px;
        resize: vertical;
    }

    input:focus,
    textarea:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.1);
    }

    .hint {
        font-size: 0.75rem;
        color: var(--text-secondary);
        opacity: 0.8;
    }

    /* Features List */
    .features-list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .feature-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem;
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-md);
        background: rgba(255, 255, 255, 0.02);
    }

    .feature-meta {
        display: flex;
        flex-direction: column;
        gap: 0.1rem;
    }

    .feature-name {
        font-weight: 500;
        font-size: 1rem;
        color: var(--text-primary);
    }

    .feature-code {
        font-family: monospace;
        font-size: 0.8rem;
        color: var(--text-secondary);
        background: rgba(0, 0, 0, 0.2);
        padding: 0.1rem 0.3rem;
        border-radius: 4px;
        width: fit-content;
    }

    .feature-desc {
        font-size: 0.85rem;
        color: var(--text-secondary);
        margin-top: 0.2rem;
    }

    .input-sm {
        padding: 0.4rem 0.5rem !important;
        font-size: 0.9rem !important;
        width: 120px !important;
        text-align: right;
    }

    .info-banner {
        background: rgba(99, 102, 241, 0.1);
        border: 1px solid rgba(99, 102, 241, 0.2);
        color: var(--color-primary);
        padding: 1rem;
        border-radius: var(--radius-md);
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    /* Toggles */
    .status-toggles {
        margin-top: 1.5rem;
        padding-top: 1.5rem;
        border-top: 1px solid var(--border-subtle);
        display: flex;
        gap: 2rem;
    }

    .toggle-item {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 0.5rem;
    }

    /* Responsive */
    @media (max-width: 900px) {
        .form-grid {
            grid-template-columns: 1fr;
        }
    }
</style>
