<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { fly } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import TableToolbar from "$lib/components/TableToolbar.svelte";
    import StatsCard from "$lib/components/StatsCard.svelte";
    import Modal from "$lib/components/Modal.svelte";
    import Input from "$lib/components/Input.svelte";
    import Select from "$lib/components/Select.svelte";
    import { toast } from "$lib/stores/toast";
    import { formatMoney } from "$lib/utils/money";
    import { get } from "svelte/store";
    import { superadminTenantsCache } from "$lib/stores/superadminTenants";
    import { superadminPlansCache } from "$lib/stores/superadminPlans";

    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";

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
    let showPassword = $state(false);

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
            const mq = window.matchMedia("(max-width: 720px)");
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
            toast.success("Tenant updated successfully");
            await loadTenants();
        } catch (e: any) {
            toast.error("Failed to update tenant: " + e);
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
            toast.success("Tenant created successfully");
            await loadTenants();
        } catch (e: any) {
            toast.error("Failed to create tenant: " + e);
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
            toast.success("Tenant deleted successfully");
            showConfirm = false;
            await loadTenants();
        } catch (e: any) {
            toast.error("Failed to delete tenant: " + e);
        } finally {
            confirmLoading = false;
            pendingDeleteId = "";
        }
    }

    let toggleKeyword = $derived.by(() =>
        pendingToggleTenant?.is_active ? "DEACTIVATE" : "ACTIVATE",
    );

    let toggleTitle = $derived.by(() =>
        pendingToggleTenant?.is_active ? "Deactivate Tenant" : "Activate Tenant",
    );

    let toggleType = $derived.by(
        (): "danger" | "warning" | "info" =>
            pendingToggleTenant?.is_active ? "danger" : "info",
    );

    let toggleMessage = $derived.by(() => {
        const name = pendingToggleTenant?.name || "this tenant";
        if (pendingToggleTenant?.is_active) {
            return `Deactivate ${name}? Users in this tenant will be blocked from accessing the app. Type ${toggleKeyword} to confirm.`;
        }
        return `Activate ${name}? Users in this tenant will regain access. Type ${toggleKeyword} to confirm.`;
    });

    let toggleConfirmText = $derived.by(() =>
        pendingToggleTenant?.is_active ? "Deactivate" : "Activate",
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
                    ? "Tenant deactivated"
                    : "Tenant activated",
            );
            showToggleConfirm = false;
            pendingToggleTenant = null;
            await loadTenants();
        } catch (e: any) {
            toast.error("Failed to update tenant: " + e);
        } finally {
            toggleLoading = false;
        }
    }
</script>

<div class="superadmin-content fade-in">
    <div class="stats-row" aria-label="Tenant stats">
        <button
            class="stat-btn"
            class:active={statusFilter === "all"}
            onclick={() => (statusFilter = "all")}
            aria-label="Show all tenants"
            title="Show all tenants"
            type="button"
        >
            <StatsCard
                title="All Tenants"
                value={stats.total}
                icon="database"
                color="primary"
            />
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "active"}
            onclick={() => (statusFilter = "active")}
            aria-label="Show active tenants"
            title="Show active tenants"
            type="button"
        >
            <StatsCard
                title="Active Tenants"
                value={stats.active}
                icon="check-circle"
                color="success"
            />
        </button>
        <button
            class="stat-btn"
            class:active={statusFilter === "inactive"}
            onclick={() => (statusFilter = "inactive")}
            aria-label="Show inactive tenants"
            title="Show inactive tenants"
            type="button"
        >
            <StatsCard
                title="Inactive Tenants"
                value={stats.inactive}
                icon="slash"
                color="warning"
            />
        </button>
    </div>

    <div class="glass-card" in:fly={{ y: 20, delay: 80 }}>
        <div class="card-header glass">
            <div>
                <h3>Tenants</h3>
                <span class="muted"
                    >Manage all organizations in the platform</span
                >
            </div>
            <div class="header-actions">
                {#if isRefreshing}
                    <span class="refresh-pill" title="Refreshing...">
                        <span class="spinner-xs"></span>
                        Refreshing
                    </span>
                {/if}
                <span class="count-badge">{stats.total} tenants</span>
            </div>
        </div>

        <div class="toolbar-wrapper">
            <TableToolbar bind:searchQuery placeholder="Search tenants...">
                {#snippet filters()}
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
                            class:active={viewMode === "table"}
                            title="Table view"
                            onclick={() => (viewMode = "table")}
                        >
                            <Icon name="list" size={18} />
                        </button>
                        <button
                            type="button"
                            class="btn-icon view-btn"
                            class:active={viewMode === "cards"}
                            title="Cards view"
                            onclick={() => (viewMode = "cards")}
                        >
                            <Icon name="grid" size={18} />
                        </button>
                    {/if}
                {/snippet}
                {#snippet actions()}
                    <button class="btn btn-primary" onclick={openCreateModal}>
                        <Icon name="plus" size={18} />
                        <span>New Tenant</span>
                    </button>
                {/snippet}
            </TableToolbar>
        </div>

        {#if error}
            <div class="error-state">
                <Icon name="alert-circle" size={48} color="#ef4444" />
                <p>{error}</p>
                <button class="btn btn-secondary" onclick={() => loadData()}>
                    Retry
                </button>
            </div>
        {:else}
            {#if viewMode === "cards" || isMobile}
                <div class="tenants-grid" aria-label="Tenant cards">
                    {#each filteredTenants as tenant (tenant.id)}
                        <div class="tenant-card" in:fly={{ y: 6, duration: 150 }}>
                            <div class="tenant-top">
                                <div>
                                    <div class="tenant-name">{tenant.name}</div>
                                    <div class="tenant-sub">
                                        <span class="tenant-slug">{tenant.slug}</span>
                                        {#if tenant.custom_domain}
                                            <span class="dot">•</span>
                                            <span class="tenant-domain mono">
                                                {tenant.custom_domain}
                                            </span>
                                        {/if}
                                    </div>
                                </div>
                                <span
                                    class="status-badge {tenant.is_active
                                        ? 'success'
                                        : 'error'}"
                                >
                                    {tenant.is_active ? "Active" : "Inactive"}
                                </span>
                            </div>

                            <div class="tenant-meta">
                                <span class="meta-label">Created</span>
                                <span class="meta-value">
                                    {tenant.created_at
                                        ? new Date(tenant.created_at).toLocaleDateString()
                                        : "—"}
                                </span>
                            </div>

                            <div class="tenant-actions">
                                <button
                                    class="btn-icon {tenant.is_active ? 'warn' : 'success'}"
                                    title={tenant.is_active ? "Deactivate" : "Activate"}
                                    type="button"
                                    onclick={() => confirmToggleTenant(tenant)}
                                >
                                    <Icon
                                        name={tenant.is_active ? "ban" : "check-circle"}
                                        size={18}
                                    />
                                </button>
                                <button
                                    class="btn-icon"
                                    title="Edit"
                                    type="button"
                                    onclick={() => openEditModal(tenant)}
                                >
                                    <Icon name="edit" size={18} />
                                </button>
                                <button
                                    class="btn-icon danger"
                                    title="Delete"
                                    type="button"
                                    onclick={() => confirmDelete(tenant.id)}
                                >
                                    <Icon name="trash" size={18} />
                                </button>
                            </div>
                        </div>
                    {/each}

                    {#if filteredTenants.length === 0}
                        <div class="empty-state-container">
                            <div class="empty-icon">
                                <Icon name="database" size={64} />
                            </div>
                            <h3>No tenants found</h3>
                            <p>Try adjusting your search or filters.</p>
                        </div>
                    {/if}
                </div>
            {:else if viewMode === "table" && !isMobile}
                <div class="table-wrapper">
                    <Table
                        pagination={true}
                        {loading}
                        data={filteredTenants}
                        {columns}
                        emptyText="No tenants found"
                        mobileView="scroll"
                    >
                        {#snippet empty()}
                            <div class="empty-state-container">
                                <div class="empty-icon">
                                    <Icon name="database" size={64} />
                                </div>
                                <h3>No tenants found</h3>
                                <p>Try adjusting your search or filters.</p>
                            </div>
                        {/snippet}

                        {#snippet cell({ item, key })}
                            {#if key === "custom_domain"}
                                {#if item.custom_domain}
                                    <code class="domain-badge"
                                        >{item.custom_domain}</code
                                    >
                                {:else}
                                    <span class="text-muted">-</span>
                                {/if}
                            {:else if key === "is_active"}
                                <span
                                    class="status-badge {item.is_active
                                        ? 'success'
                                        : 'error'}"
                                >
                                    {item.is_active ? "Active" : "Inactive"}
                                </span>
                            {:else if key === "created_at"}
                                {new Date(item.created_at).toLocaleDateString()}
                            {:else if key === "actions"}
                                <div class="actions">
                                    <button
                                        class="btn-icon {item.is_active
                                            ? 'warn'
                                            : 'success'}"
                                        title={item.is_active
                                            ? "Deactivate"
                                            : "Activate"}
                                        type="button"
                                        onclick={() => confirmToggleTenant(item)}
                                    >
                                        <Icon
                                            name={item.is_active
                                                ? "ban"
                                                : "check-circle"}
                                            size={18}
                                        />
                                    </button>
                                    <button
                                        class="btn-icon"
                                        title="Edit"
                                        type="button"
                                        onclick={() => openEditModal(item)}
                                    >
                                        <Icon name="edit" size={18} />
                                    </button>
                                    <button
                                        class="btn-icon danger"
                                        title="Delete"
                                        type="button"
                                        onclick={() => confirmDelete(item.id)}
                                    >
                                        <Icon name="trash" size={18} />
                                    </button>
                                </div>
                            {:else}
                                {item[key]}
                            {/if}
                        {/snippet}
                    </Table>
                </div>
            {/if}
        {/if}
    </div>
</div>

<Modal
    bind:show={showCreateModal}
    title={isEditing ? "Edit Tenant" : "Create New Tenant"}
>
    <div class="modal-form">
        <Input
            label="Tenant Name"
            bind:value={newTenant.name}
            oninput={generateSlug}
            placeholder="e.g. Acme Corp"
        />

        <Input
            label="Slug (URL)"
            bind:value={newTenant.slug}
            placeholder="e.g. acme-corp"
            disabled={isEditing}
        />

        <Input
            label="Custom Domain (Optional)"
            bind:value={newTenant.customDomain}
            placeholder="e.g. app.acme.com"
        />

        {#if !isEditing}
            <div class="divider">
                <span>Initial Subscription</span>
            </div>

            <Select
                label="Subscription Plan"
                options={plans}
                bind:value={newTenant.planId}
                placeholder="Select a plan"
            />

            <div class="divider">
                <span>Initial Admin User</span>
            </div>

            <Input
                label="Owner Email"
                type="email"
                bind:value={newTenant.ownerEmail}
                placeholder="admin@acme.com"
            />

            <div class="password-group">
                <Input
                    label="Owner Password"
                    type={showPassword ? "text" : "password"}
                    bind:value={newTenant.ownerPassword}
                    placeholder="Strong password"
                />
                <button
                    class="toggle-password"
                    onclick={() => (showPassword = !showPassword)}
                    type="button"
                >
                    <Icon name={showPassword ? "eye-off" : "eye"} size={18} />
                </button>
            </div>
        {/if}

        <div class="form-group toggle-row">
            <label>
                <input type="checkbox" bind:checked={newTenant.isActive} />
                <span class="toggle-label">Active Status</span>
            </label>
        </div>

        <div class="modal-actions">
            <button
                class="btn btn-secondary"
                onclick={() => (showCreateModal = false)}
                disabled={creating}
            >
                Cancel
            </button>
            <button
                class="btn btn-primary"
                onclick={handleSubmit}
                disabled={creating}
            >
                {#if creating}
                    <div class="spinner-sm"></div>
                {/if}
                {isEditing ? "Update Tenant" : "Create Tenant"}
            </button>
        </div>
    </div>
</Modal>

<ConfirmDialog
    bind:show={showConfirm}
    title="Delete Tenant"
    message="Are you sure you want to delete this tenant? This action cannot be undone and will remove all associated data. Type DELETE to confirm."
    confirmText="Delete Permanently"
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

    .table-wrapper {
        padding: 0 1.25rem 1rem 1.25rem;
    }

    .mono {
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
            "Liberation Mono", "Courier New", monospace;
    }

    .tenants-grid {
        padding: 0 1.25rem 1.25rem 1.25rem;
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
        gap: 1rem;
    }

    .tenant-card {
        background: linear-gradient(
            145deg,
            rgba(255, 255, 255, 0.06),
            rgba(255, 255, 255, 0.02)
        );
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 18px;
        padding: 1rem;
        box-shadow: 0 14px 36px rgba(0, 0, 0, 0.25);
    }

    :global([data-theme="light"]) .tenant-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 12px 28px rgba(0, 0, 0, 0.06),
            0 0 0 1px rgba(255, 255, 255, 0.85);
    }

    .tenant-top {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 0.75rem;
    }

    .tenant-name {
        font-weight: 900;
        color: var(--text-primary);
        letter-spacing: -0.02em;
        line-height: 1.15;
    }

    .tenant-sub {
        margin-top: 0.35rem;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        flex-wrap: wrap;
        color: var(--text-secondary);
        font-weight: 650;
        font-size: 0.9rem;
    }

    .tenant-slug {
        padding: 0.15rem 0.5rem;
        border-radius: 999px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        background: rgba(255, 255, 255, 0.03);
    }

    .dot {
        opacity: 0.6;
    }

    .tenant-domain {
        opacity: 0.9;
    }

    .tenant-meta {
        margin-top: 0.9rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding-top: 0.75rem;
        border-top: 1px solid rgba(255, 255, 255, 0.06);
        color: var(--text-secondary);
    }

    :global([data-theme="light"]) .tenant-meta {
        border-top-color: rgba(0, 0, 0, 0.06);
    }

    .meta-label {
        font-size: 0.8rem;
        font-weight: 750;
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }

    .meta-value {
        font-weight: 750;
        color: var(--text-primary);
    }

    .tenant-actions {
        margin-top: 0.9rem;
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
    }

    .status-badge {
        padding: 0.25rem 0.75rem;
        border-radius: 20px;
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .status-badge.success {
        background: rgba(16, 185, 129, 0.1);
        color: #10b981;
    }

    .status-badge.error {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
    }

    .domain-badge {
        background: var(--bg-app);
        padding: 0.2rem 0.5rem;
        border-radius: 4px;
        font-family: monospace;
        font-size: 0.85rem;
        color: var(--color-primary);
    }

    .text-muted {
        color: var(--text-secondary);
        font-style: italic;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
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

    /* Modal Form Styles */
    .modal-form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .divider {
        display: flex;
        align-items: center;
        gap: 1rem;
        margin: 1rem 0;
        color: var(--text-secondary);
        font-size: 0.8rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .divider::before,
    .divider::after {
        content: "";
        flex: 1;
        height: 1px;
        background: var(--border-color);
    }

    .password-group {
        position: relative;
    }

    .toggle-password {
        position: absolute;
        right: 0;
        top: 28px; /* Approximate alignment with input content */
        background: transparent;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0.5rem;
    }

    .toggle-row {
        margin-top: 0.5rem;
    }

    .toggle-row label {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        cursor: pointer;
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.75rem;
        margin-top: 1.5rem;
    }

    .spinner-sm {
        width: 16px;
        height: 16px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    @media (max-width: 768px) {
        .stats-row {
            grid-template-columns: 1fr;
            gap: 0.75rem;
        }

        .toolbar-wrapper {
            padding: 0.9rem 1rem 0 1rem;
        }

        .table-wrapper {
            padding: 0 1rem 1rem 1rem;
        }

        .tenants-grid {
            padding: 0 1rem 1rem 1rem;
            grid-template-columns: 1fr;
        }

        .btn {
            justify-content: center;
        }
    }
</style>
