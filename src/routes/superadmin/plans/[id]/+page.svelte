<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import { api } from "$lib/api/client";
    import Icon from "$lib/components/Icon.svelte";
    import { toast } from "$lib/stores/toast";
    import { appSettings } from "$lib/stores/settings";
    import { formatMoney } from "$lib/utils/money";
    import { superadminPlansCache } from "$lib/stores/superadminPlans";
    import { get } from "svelte/store";
    import { t } from "svelte-i18n";

    let id = $state("");
    let isNew = $state(true);
    let loading = $state(true);
    let saving = $state(false);

    // Data Models
    let planData = $state<any>({
        name: "",
        slug: "",
        description: "",
        price_monthly: 0,
        price_yearly: 0,
        is_active: true,
        is_default: false,
        sort_order: 0,
    });

    let features = $state<any[]>([]);
    let planFeatures = $state<Record<string, string>>({}); // feature_id -> value

    let activeTab = $state("general");

    $effect(() => {
        id = $page.params.id as string;
        isNew = id === "new";
    });

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

    function upsertPlanCache(savedPlan: any) {
        const cached = get(superadminPlansCache);
        const existing = cached?.plans || [];
        const next = existing.some((p) => p.id === savedPlan.id)
            ? existing.map((p) => (p.id === savedPlan.id ? savedPlan : p))
            : [savedPlan, ...existing];
        superadminPlansCache.set({ plans: next, fetchedAt: Date.now() });
    }

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

                upsertPlanCache(created);
                toast.success(
                    get(t)("superadmin.plans.editor.toasts.created") ||
                        "Plan created successfully",
                );
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

                upsertPlanCache({ ...planData, id });
                toast.success(
                    get(t)("superadmin.plans.editor.toasts.updated") ||
                        "Plan updated successfully",
                );
            }
        } catch (e: any) {
            toast.error(
                e.message ||
                    get(t)("superadmin.plans.editor.errors.save_failed") ||
                    "Failed to save plan",
            );
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

    function moneyStep() {
        const code = String($appSettings?.currency_code || "IDR").toUpperCase();
        return code === "IDR" || code === "JPY" || code === "KRW" ? "1" : "0.01";
    }

    const baseCurrencyCode = $derived.by(() =>
        String($appSettings?.currency_code || "IDR").toUpperCase(),
    );

    const baseLocale = $derived.by(() =>
        String($appSettings?.default_locale || "en-US"),
    );
</script>

<svelte:head>
    <title>{isNew ? ($t("superadmin.plans.editor.head.new") || "New Plan") : planData.name} | Superadmin</title>
</svelte:head>

<div class="plan-detail-page">
    <div class="page-header">
        <div class="header-content">
            <h1>
                {isNew
                    ? $t("superadmin.plans.editor.header.create_title") ||
                      "Create New Plan"
                    : $t("superadmin.plans.editor.header.edit_title", {
                          values: { name: planData.name },
                      }) || `Edit ${planData.name}`}
            </h1>
            <div
                class="actions"
                aria-label={$t("superadmin.plans.editor.aria.actions") ||
                    "Plan actions"}
            >
                <button
                    class="btn btn-secondary"
                    type="button"
                    onclick={() => goto("/superadmin/plans")}
                >
                    {$t("common.cancel") || "Cancel"}
                </button>
                <button
                    class="btn btn-primary"
                    onclick={savePlan}
                    disabled={saving}
                    type="button"
                >
                    {#if saving}
                        {$t("common.saving") || "Saving..."}
                    {:else}
                        {$t("superadmin.plans.editor.actions.save") || "Save Plan"}
                    {/if}
                </button>
            </div>
        </div>
    </div>

    {#if loading}
        <div class="loading">{$t("common.loading") || "Loading..."}</div>
    {:else}
        <div
            class="tabs"
            role="tablist"
            aria-label={$t("superadmin.plans.editor.aria.tabs") ||
                "Plan editor tabs"}
        >
            <button
                class="tab {activeTab === 'general' ? 'active' : ''}"
                onclick={() => (activeTab = "general")}
                type="button"
                role="tab"
                aria-selected={activeTab === "general"}
            >
                {$t("superadmin.plans.editor.tabs.general") || "General"}
            </button>
            <button
                class="tab {activeTab === 'features' ? 'active' : ''}"
                onclick={() => (activeTab = "features")}
                type="button"
                role="tab"
                aria-selected={activeTab === "features"}
            >
                {$t("superadmin.plans.editor.tabs.features") || "Features & Limits"}
            </button>
        </div>

        <div class="glass-card" role="tabpanel">
            {#if activeTab === "general"}
                <div class="form-grid fade-in">
                    <div class="form-group">
                        <label for="name">
                            {$t("superadmin.plans.editor.fields.name") || "Plan Name"}
                        </label>
                        <input
                            id="name"
                            type="text"
                            bind:value={planData.name}
                            oninput={onNameChange}
                            placeholder={$t(
                                "superadmin.plans.editor.placeholders.name",
                            ) || "e.g. Pro Plan"}
                        />
                    </div>

                    <div class="form-group">
                        <label for="slug">
                            {$t("superadmin.plans.editor.fields.slug") || "Slug (Code)"}
                        </label>
                        <input
                            id="slug"
                            type="text"
                            bind:value={planData.slug}
                            disabled={!isNew}
                            placeholder={$t(
                                "superadmin.plans.editor.placeholders.slug",
                            ) || "e.g. pro-plan"}
                        />
                        <small>
                            {$t("superadmin.plans.editor.help.slug") ||
                                "Unique identifier used in code/API."}
                        </small>
                    </div>

                    <div class="form-group full-width">
                        <label for="description">
                            {$t("superadmin.plans.editor.fields.description") ||
                                "Description"}
                        </label>
                        <textarea
                            id="description"
                            bind:value={planData.description}
                            rows="2"
                        ></textarea>
                    </div>

                    <div class="form-group">
                        <label for="price_m">
                            Monthly Price ({baseCurrencyCode})
                        </label>
                        <div class="money-input">
                            <input
                                id="price_m"
                                type="number"
                                step={moneyStep()}
                                bind:value={planData.price_monthly}
                            />
                            <span class="money-suffix">{baseCurrencyCode}</span>
                        </div>
                        <small class="hint"
                            >Preview: {formatMoney(planData.price_monthly, {
                                currency: baseCurrencyCode,
                                locale: baseLocale,
                            })}
                            /mo</small
                        >
                    </div>

                    <div class="form-group">
                        <label for="price_y">
                            Yearly Price ({baseCurrencyCode})
                        </label>
                        <div class="money-input">
                            <input
                                id="price_y"
                                type="number"
                                step={moneyStep()}
                                bind:value={planData.price_yearly}
                            />
                            <span class="money-suffix">{baseCurrencyCode}</span>
                        </div>
                        <small class="hint"
                            >Preview: {formatMoney(planData.price_yearly, {
                                currency: baseCurrencyCode,
                                locale: baseLocale,
                            })}
                            /yr</small
                        >
                    </div>

                    <div class="form-group">
                        <label for="sort_order"
                            >{$t("superadmin.plans.editor.fields.sort_order") ||
                                "Sort Order"}</label
                        >
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

                    <div class="form-group full-width note" role="note">
                        <Icon name="info" size={16} />
                        <span>
                            Plan prices are stored in the base currency (
                            {baseCurrencyCode}). If a tenant uses a
                            different currency, invoices will be auto-converted
                            using the configured FX provider.
                        </span>
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

    .header-content {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 1rem;
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

    .tabs {
        display: flex;
        gap: 1rem;
        margin-bottom: 1.5rem;
        border-bottom: 1px solid var(--border-color);
        overflow-x: auto;
        -webkit-overflow-scrolling: touch;
        scrollbar-width: none;
    }

    .tabs::-webkit-scrollbar {
        height: 0;
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
        white-space: nowrap;
    }

    .tab.active {
        color: var(--color-primary);
        border-bottom-color: var(--color-primary);
    }

    .glass-card {
        background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
        border-radius: 16px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        padding: 2rem;
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.28);
    }

    :global([data-theme="light"]) .glass-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 12px 32px rgba(0, 0, 0, 0.08),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }

    .form-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1.5rem;
    }

    .full-width {
        grid-column: 1 / -1;
    }

    .money-input {
        position: relative;
        display: flex;
        align-items: center;
    }

    .money-input input {
        width: 100%;
        padding-right: 5.25rem;
    }

    .money-suffix {
        position: absolute;
        right: 0.75rem;
        top: 50%;
        transform: translateY(-50%);
        font-size: 0.85rem;
        font-weight: 700;
        letter-spacing: 0.04em;
        color: var(--text-secondary);
        opacity: 0.9;
        pointer-events: none;
    }

    .hint {
        display: block;
        margin-top: 0.4rem;
        color: var(--text-secondary);
    }

    .note {
        display: flex;
        align-items: flex-start;
        gap: 0.6rem;
        padding: 0.9rem 1rem;
        border-radius: 12px;
        background: rgba(99, 102, 241, 0.08);
        border: 1px solid rgba(99, 102, 241, 0.22);
        color: var(--text-primary);
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
        width: 100%;
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
        gap: 1rem;
    }

    @media (max-width: 720px) {
        .plan-detail-page {
            padding: 1rem;
        }

        .page-header {
            margin-bottom: 1.25rem;
        }

        .header-content {
            flex-direction: column;
            align-items: flex-start;
        }

        h1 {
            font-size: 1.35rem;
        }

        .actions {
            width: 100%;
        }

        .actions :global(.btn) {
            flex: 1;
            width: 100%;
        }

        .glass-card {
            padding: 1.25rem;
            border-radius: 14px;
        }

        .form-grid {
            grid-template-columns: 1fr;
            gap: 1.1rem;
        }

        .toggle-group {
            flex-direction: column;
            align-items: flex-start;
            gap: 0.75rem;
            padding-top: 0.25rem;
        }

        .feature-item {
            flex-direction: column;
            align-items: stretch;
        }

        .feature-control {
            display: flex;
            justify-content: flex-end;
        }

        .input-sm {
            width: 100%;
        }
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
