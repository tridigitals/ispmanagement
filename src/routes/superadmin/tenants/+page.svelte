<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { fly } from "svelte/transition";
    import Icon from "$lib/components/ui/Icon.svelte";
    import TableToolbar from "$lib/components/ui/TableToolbar.svelte";
    import StatsCard from "$lib/components/dashboard/StatsCard.svelte";
    import { toast } from "$lib/stores/toast";
    import { formatMoney } from "$lib/utils/money";
    import { get } from "svelte/store";
    import { superadminTenantsCache } from "$lib/stores/superadminTenants";
    import { superadminPlansCache } from "$lib/stores/superadminPlans";
    import { t } from "svelte-i18n";

    import ConfirmDialog from "$lib/components/ui/ConfirmDialog.svelte";
    import TenantTable from "$lib/components/superadmin/tenants/TenantTable.svelte";
    import TenantFormModal from "$lib/components/superadmin/tenants/TenantFormModal.svelte";

    let tenants = $state<any[]>([]);
    let plans = $state<any[]>([]);
    let loading = $state(true);
    let isRefreshing = $state(false);
    let error = $state("");
    let isMobile = $state(false);
    let viewMode = $state<"cards" | "table">("table");

    // Modal state
    let isEditing = $state(false);
    let editingId = $state("");
    let showCreateModal = $state(false);
    let newTenant = $state({
        name: "",
        slug: "",
        customDomain: "",
        ownerEmail: "",
        ownerPassword: "",
        isActive: true,
        planId: "",
    });
    let creating = $state(false);

    // Confirm Dialog State
    let showConfirm = $state(false);
    let confirmLoading = $state(false);
    let pendingDeleteId = $state("");

    // Activate/Deactivate Tenant dialog
    let showToggleConfirm = $state(false);
    let toggleLoading = $state(false);
    let pendingToggleTenant = $state<any | null>(null);

    let searchQuery = $state("");
    let statusFilter = $state<"all" | "active" | "inactive">("all");

    let stats = $derived({
        total: tenants.length,
        active: tenants.filter((t) => t.is_active).length,
        inactive: tenants.filter((t) => !t.is_active).length,
    });

    let filteredTenants = $derived(
        tenants.filter((t) => {
            const q = searchQuery.trim().toLowerCase();
            const matchesSearch =
                !q ||
                String(t.name || "")
                    .toLowerCase()
                    .includes(q) ||
                String(t.slug || "")
                    .toLowerCase()
                    .includes(q) ||
                String(t.custom_domain || "")
                    .toLowerCase()
                    .includes(q);

            const matchesStatus =
                statusFilter === "all" ||
                (statusFilter === "active" ? t.is_active : !t.is_active);

            return matchesSearch && matchesStatus;
        }),
    );

    // Table columns
    const columns = [
        { key: "name", label: "Tenant Name", sortable: true },
        { key: "slug", label: "Slug", sortable: true },
        { key: "custom_domain", label: "Custom Domain", sortable: true },
        { key: "is_active", label: "Status", sortable: true },
        { key: "created_at", label: "Created At", sortable: true },
        { key: "actions", label: "Actions", align: "right" },
    ];

    onMount(() => {
        let cleanup: (() => void) | undefined;

        if (typeof window !== "undefined") {
            const mq = window.matchMedia("(max-width: 899px)");
            const sync = () => {
                isMobile = mq.matches;
                if (mq.matches) viewMode = "cards";
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

        const cachedTenants = get(superadminTenantsCache);
        if (cachedTenants?.fetchedAt && cachedTenants.tenants?.length) {
            tenants = cachedTenants.tenants as any[];
            loading = false;
            void loadData({ silent: true });
        } else {
            void loadData();
        }

        return cleanup;
    });

    $effect(() => {
        if (isMobile && viewMode === "table") viewMode = "cards";
    });

    function mapPlansToSelect(plansRes: any[]) {
        plans = (plansRes || [])
            .filter((p) => p.is_active)
            .map((p) => ({
                label: `${p.name} - ${p.price_monthly > 0 ? `${formatMoney(p.price_monthly)}/mo` : "Free"}`,
                value: p.id,
            }));

        const defaultPlan = (plansRes || []).find((p) => p.is_default);
        if (defaultPlan) {
            newTenant.planId = defaultPlan.id;
        } else if (!newTenant.planId && plans.length > 0) {
            newTenant.planId = plans[0].value;
        }
    }

    async function loadData(opts: { silent?: boolean } = {}) {
        if (opts.silent) isRefreshing = true;
        else loading = true;
        try {
            const cachedPlans = get(superadminPlansCache);
            if (cachedPlans?.fetchedAt && cachedPlans.plans?.length) {
                mapPlansToSelect(cachedPlans.plans as any[]);
            }

            const [tenantsRes, plansRes] = await Promise.all([
                api.superadmin.listTenants(),
                api.plans.list().catch(() => null),
            ]);

            if (Array.isArray(tenantsRes)) {
                tenants = tenantsRes;
            } else if (tenantsRes && Array.isArray(tenantsRes.data)) {
                tenants = tenantsRes.data;
            } else {
                tenants = [];
            }

            superadminTenantsCache.set({ tenants, fetchedAt: Date.now() });

            if (plansRes) {
                mapPlansToSelect(plansRes as any[]);
                superadminPlansCache.set({
                    plans: plansRes as any[],
                    fetchedAt: Date.now(),
                });
            }
        } catch (e: any) {
            console.error("Load data error:", e);
            error = e.toString();
            if (e.toString().includes("Unauthorized")) {
                goto("/dashboard");
            }
        } finally {
            loading = false;
            isRefreshing = false;
        }
    }

    async function loadTenants() {
        try {
            const res: any = await api.superadmin.listTenants();
            if (Array.isArray(res)) {
                tenants = res;
            } else if (res && Array.isArray(res.data)) {
                tenants = res.data;
            }
            superadminTenantsCache.set({ tenants, fetchedAt: Date.now() });
        } catch (e) {
            console.error("Reload error", e);
        }
    }

    function openCreateModal() {
        isEditing = false;
        editingId = "";

        Object.assign(newTenant, {
            name: "",
            slug: "",
            customDomain: "",
            ownerEmail: "",
            ownerPassword: "",
            isActive: true,
            planId: plans.length > 0 ? plans[0].value : "",
        });
        showCreateModal = true;
    }

    function openEditModal(tenant: any) {
        isEditing = true;
        editingId = tenant.id;
        Object.assign(newTenant, {
            name: tenant.name,
            slug: tenant.slug,
            customDomain: tenant.custom_domain || "",
            ownerEmail: "---", // Email cannot be changed here easily in this view
            ownerPassword: "", // Password not needed for update
            isActive: tenant.is_active,
            planId: "", // Plan cannot be changed here for now (use subscription page)
        });
        showCreateModal = true;
    }

    async function handleSubmit() {
        if (isEditing) {
            await updateTenant();
        } else {
            await createTenant();
        }
    }

    async function updateTenant() {
        if (!newTenant.name || !newTenant.slug) return;
        creating = true;
        try {
            await api.superadmin.updateTenant(
                editingId,
                newTenant.name,
                newTenant.slug,
                newTenant.customDomain || null,
                newTenant.isActive,
            );
            showCreateModal = false;
            toast.success(
                get(t)("superadmin.tenants.toasts.updated") ||
                    "Tenant updated successfully",
            );
            await loadTenants();
        } catch (e: any) {
            toast.error(
                get(t)("superadmin.tenants.toasts.update_failed", {
                    values: { message: e?.message || e },
                }) || "Failed to update tenant: " + e,
            );
        } finally {
            creating = false;
        }
    }

    async function createTenant() {
        if (
            !newTenant.name ||
            !newTenant.slug ||
            !newTenant.ownerEmail ||
            !newTenant.ownerPassword
        )
            return;
        creating = true;
        try {
            await api.superadmin.createTenant(
                newTenant.name,
                newTenant.slug,
                newTenant.customDomain || null,
                newTenant.ownerEmail,
                newTenant.ownerPassword,
                newTenant.planId || undefined, // Pass planId
            );

            showCreateModal = false;
            toast.success(
                get(t)("superadmin.tenants.toasts.created") ||
                    "Tenant created successfully",
            );
            await loadTenants();
        } catch (e: any) {
            toast.error(
                get(t)("superadmin.tenants.toasts.create_failed", {
                    values: { message: e?.message || e },
                }) || "Failed to create tenant: " + e,
            );
        } finally {
            creating = false;
        }
    }

    function generateSlug() {
        if (!newTenant.name) return;
        newTenant.slug = newTenant.name
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, "-")
            .replace(/(^-|-$)/g, "");
    }

    function confirmDelete(id: string) {
        pendingDeleteId = id;
        showConfirm = true;
    }

    function confirmToggleTenant(tenant: any) {
        pendingToggleTenant = tenant;
        showToggleConfirm = true;
    }

    async function handleDelete() {
        if (!pendingDeleteId) return;
        confirmLoading = true;
        try {
            await api.superadmin.deleteTenant(pendingDeleteId);
            toast.success(
                get(t)("superadmin.tenants.toasts.deleted") ||
                    "Tenant deleted successfully",
            );
            showConfirm = false;
            await loadTenants();
        } catch (e: any) {
            toast.error(
                get(t)("superadmin.tenants.toasts.delete_failed", {
                    values: { message: e?.message || e },
                }) || "Failed to delete tenant: " + e,
            );
        } finally {
            confirmLoading = false;
            pendingDeleteId = "";
        }
    }

    let toggleKeyword = $derived.by(() =>
        pendingToggleTenant?.is_active ? "DEACTIVATE" : "ACTIVATE",
    );

    let toggleTitle = $derived.by(() =>
        pendingToggleTenant?.is_active
            ? $t("superadmin.tenants.toggle.deactivate_title") ||
              "Deactivate Tenant"
            : $t("superadmin.tenants.toggle.activate_title") ||
              "Activate Tenant",
    );

    let toggleType = $derived.by((): "danger" | "warning" | "info" =>
        pendingToggleTenant?.is_active ? "danger" : "info",
    );

    let toggleMessage = $derived.by(() => {
        const name =
            pendingToggleTenant?.name ||
            $t("superadmin.tenants.toggle.this_tenant") ||
            "this tenant";
        if (pendingToggleTenant?.is_active) {
            return (
                $t("superadmin.tenants.toggle.deactivate_message", {
                    values: { name },
                }) ||
                `Deactivate ${name}? Users in this tenant will be blocked from accessing the app.`
            );
        }
        return (
            $t("superadmin.tenants.toggle.activate_message", {
                values: { name },
            }) || `Activate ${name}? Users in this tenant will regain access.`
        );
    });

    let toggleConfirmText = $derived.by(() =>
        pendingToggleTenant?.is_active
            ? $t("superadmin.tenants.actions.deactivate") || "Deactivate"
            : $t("superadmin.tenants.actions.activate") || "Activate",
    );

    async function handleToggleTenant() {
        if (!pendingToggleTenant) return;
        toggleLoading = true;
        try {
            await api.superadmin.updateTenant(
                pendingToggleTenant.id,
                pendingToggleTenant.name,
                pendingToggleTenant.slug,
                pendingToggleTenant.custom_domain || null,
                !pendingToggleTenant.is_active,
            );
            toast.success(
                pendingToggleTenant.is_active
                    ? get(t)("superadmin.tenants.toasts.deactivated") ||
                          "Tenant deactivated"
                    : get(t)("superadmin.tenants.toasts.activated") ||
                          "Tenant activated",
            );
            showToggleConfirm = false;
            pendingToggleTenant = null;
            await loadTenants();
        } catch (e: any) {
            toast.error(
                get(t)("superadmin.tenants.toasts.update_failed", {
                    values: { message: e?.message || e },
                }) || "Failed to update tenant: " + e,
            );
        } finally {
            toggleLoading = false;
        }
    }
</script>

<div class="superadmin-content fade-in">
    <div
        class="stats-row"
        aria-label={$t("superadmin.tenants.aria.stats") || "Tenant stats"}
    >
        <button
            class="stat-btn"
            class:active={statusFilter === "all"}
            onclick={() => (statusFilter = "all")}
            aria-label={$t("superadmin.tenants.stats.show_all") ||
                "Show all tenants"}
            title={$t("superadmin.tenants.stats.show_all") ||
                "Show all tenants"}
            type="button"
        >
            <StatsCard
                title={$t("superadmin.tenants.stats.all_title") ||
                    "All Tenants"}
                value={stats.total}
                icon="database"
                color="primary"
            />
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "active"}
            onclick={() => (statusFilter = "active")}
            aria-label={$t("superadmin.tenants.stats.show_active") ||
                "Show active tenants"}
            title={$t("superadmin.tenants.stats.show_active") ||
                "Show active tenants"}
            type="button"
        >
            <StatsCard
                title={$t("superadmin.tenants.stats.active_title") ||
                    "Active Tenants"}
                value={stats.active}
                icon="check-circle"
                color="success"
            />
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "inactive"}
            onclick={() => (statusFilter = "inactive")}
            aria-label={$t("superadmin.tenants.stats.show_inactive") ||
                "Show inactive tenants"}
            title={$t("superadmin.tenants.stats.show_inactive") ||
                "Show inactive tenants"}
            type="button"
        >
            <StatsCard
                title={$t("superadmin.tenants.stats.inactive_title") ||
                    "Inactive Tenants"}
                value={stats.inactive}
                icon="slash"
                color="warning"
            />
        </button>
    </div>

    <div class="glass-card" in:fly={{ y: 20, delay: 80 }}>
        <div class="card-header glass">
            <div>
                <h3>{$t("superadmin.tenants.title") || "Tenants"}</h3>
                <span class="muted"
                    >{$t("superadmin.tenants.subtitle") ||
                        "Manage all organizations in the platform"}</span
                >
            </div>
            <div class="header-actions">
                {#if isRefreshing}
                    <span
                        class="refresh-pill"
                        title={$t("superadmin.tenants.refreshing_title") ||
                            "Refreshing..."}
                    >
                        <span class="spinner-xs"></span>
                        {$t("superadmin.tenants.refreshing") || "Refreshing"}
                    </span>
                {/if}
                <span class="count-badge"
                    >{$t("superadmin.tenants.count", {
                        values: { count: stats.total },
                    }) || `${stats.total} tenants`}</span
                >
            </div>
        </div>

        <div class="toolbar-wrapper">
            <TableToolbar
                bind:searchQuery
                placeholder={$t("superadmin.tenants.search") ||
                    "Search tenants..."}
            >
                {#snippet filters()}
                    <div class="status-filter">
                        <button
                            type="button"
                            class="filter-chip"
                            class:active={statusFilter === "all"}
                            onclick={() => (statusFilter = "all")}
                        >
                            {$t("superadmin.tenants.filters.all") ||
                                $t("common.all") ||
                                "All"}
                        </button>
                        <button
                            type="button"
                            class="filter-chip"
                            class:active={statusFilter === "active"}
                            onclick={() => (statusFilter = "active")}
                        >
                            {$t("superadmin.tenants.filters.active") ||
                                $t("common.active") ||
                                "Active"}
                        </button>
                        <button
                            type="button"
                            class="filter-chip"
                            class:active={statusFilter === "inactive"}
                            onclick={() => (statusFilter = "inactive")}
                        >
                            {$t("superadmin.tenants.filters.inactive") ||
                                $t("common.inactive") ||
                                "Inactive"}
                        </button>
                    </div>

                    {#if !isMobile}
                        <button
                            type="button"
                            class="btn-icon view-btn"
                            class:active={viewMode === "table"}
                            title={$t("superadmin.tenants.view.table") ||
                                "Table view"}
                            onclick={() => (viewMode = "table")}
                        >
                            <Icon name="list" size={18} />
                        </button>
                        <button
                            type="button"
                            class="btn-icon view-btn"
                            class:active={viewMode === "cards"}
                            title={$t("superadmin.tenants.view.cards") ||
                                "Cards view"}
                            onclick={() => (viewMode = "cards")}
                        >
                            <Icon name="grid" size={18} />
                        </button>
                    {/if}
                {/snippet}
                {#snippet actions()}
                    <button class="btn btn-primary" onclick={openCreateModal}>
                        <Icon name="plus" size={18} />
                        <span>
                            {$t("superadmin.tenants.actions.new") ||
                                "New Tenant"}
                        </span>
                    </button>
                {/snippet}
            </TableToolbar>
        </div>

        {#if error}
            <div class="error-state">
                <Icon name="alert-circle" size={48} color="#ef4444" />
                <p>{error}</p>
                <button class="btn btn-secondary" onclick={() => loadData()}>
                    {$t("common.retry") || "Retry"}
                </button>
            </div>
        {:else}
            <TenantTable
                tenants={filteredTenants}
                {loading}
                {viewMode}
                {isMobile}
                {columns}
                onEdit={openEditModal}
                onDelete={(id: string) => confirmDelete(id)}
                onToggleStatus={confirmToggleTenant}
            />
        {/if}
    </div>
</div>

<TenantFormModal
    bind:show={showCreateModal}
    {isEditing}
    bind:newTenant
    {plans}
    loading={creating}
    onSubmit={handleSubmit}
    onGenerateSlug={generateSlug}
/>

<ConfirmDialog
    bind:show={showConfirm}
    title={$t("superadmin.tenants.delete.title") || "Delete Tenant"}
    message={$t("superadmin.tenants.delete.message") ||
        "Are you sure you want to delete this tenant? This action cannot be undone and will remove all associated data."}
    confirmText={$t("superadmin.tenants.delete.confirm") ||
        "Delete Permanently"}
    confirmationKeyword="DELETE"
    type="danger"
    loading={confirmLoading}
    onconfirm={handleDelete}
/>

<ConfirmDialog
    bind:show={showToggleConfirm}
    title={toggleTitle}
    message={toggleMessage}
    confirmText={toggleConfirmText}
    confirmationKeyword={toggleKeyword}
    type={toggleType}
    loading={toggleLoading}
    onconfirm={handleToggleTenant}
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
        grid-template-columns: repeat(3, minmax(0, 1fr));
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

    .stat-btn:hover {
        transform: translateY(-1px);
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

    .header-actions {
        display: inline-flex;
        align-items: center;
        gap: 0.75rem;
        flex-wrap: wrap;
        justify-content: flex-end;
    }

    .refresh-pill {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 0.75rem;
        border-radius: 999px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        background: rgba(255, 255, 255, 0.04);
        color: var(--text-secondary);
        font-weight: 750;
        font-size: 0.85rem;
        user-select: none;
    }

    .spinner-xs {
        width: 14px;
        height: 14px;
        border-radius: 999px;
        border: 2px solid rgba(255, 255, 255, 0.14);
        border-top-color: rgba(99, 102, 241, 0.95);
        animation: spin 0.9s linear infinite;
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

    .count-badge {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-primary);
        padding: 0.35rem 0.75rem;
        border-radius: 999px;
        font-size: 0.85rem;
        font-weight: 650;
        white-space: nowrap;
        align-self: flex-start;
    }

    :global([data-theme="light"]) .count-badge {
        background: rgba(0, 0, 0, 0.03);
        border-color: rgba(0, 0, 0, 0.06);
    }

    .toolbar-wrapper {
        padding: 1rem 1.25rem 0.25rem 1.25rem;
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

    :global(.btn-icon.view-btn.active) {
        background: rgba(99, 102, 241, 0.14);
        border-color: rgba(99, 102, 241, 0.35);
        color: var(--text-primary);
    }

    .error-state {
        padding: 2rem 1.25rem;
        text-align: center;
        color: var(--text-secondary);
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.5rem;
    }

    .btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.25rem;
        border-radius: var(--radius-md);
        font-weight: 600;
        font-size: 0.9rem;
        border: none;
        cursor: pointer;
        transition: all 0.2s;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover {
        background: var(--color-primary-hover);
    }

    .btn-secondary {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .btn-secondary:hover {
        background: var(--bg-hover);
    }

    .btn-icon {
        width: 36px;
        height: 36px;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.02);
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
    }

    :global([data-theme="light"]) .btn-icon {
        border-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.02);
        color: var(--text-secondary);
    }

    @media (max-width: 768px) {
        .stats-row {
            grid-template-columns: 1fr;
            gap: 0.75rem;
        }

        .toolbar-wrapper {
            padding: 0.9rem 1rem 0 1rem;
        }

        .btn {
            justify-content: center;
        }
    }
</style>
