<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { user } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import { fade } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import Modal from "$lib/components/Modal.svelte";
    import Input from "$lib/components/Input.svelte";
    import Select from "$lib/components/Select.svelte";
    import { toast } from "$lib/stores/toast";

    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";

    let tenants: any[] = [];
    let plans: any[] = [];
    let loading = true;
    let error = "";

    // Modal state
    let isEditing = false;
    let editingId = "";
    let showCreateModal = false;
    let newTenant = {
        name: "",
        slug: "",
        customDomain: "",
        ownerEmail: "",
        ownerPassword: "",
        isActive: true,
        planId: "",
    };
    let creating = false;
    let showPassword = false;

    // Confirm Dialog State
    let showConfirm = false;
    let confirmLoading = false;
    let pendingDeleteId = "";

    // Table columns
    const columns = [
        { key: "name", label: "Tenant Name", sortable: true },
        { key: "slug", label: "Slug", sortable: true },
        { key: "custom_domain", label: "Custom Domain", sortable: true },
        { key: "is_active", label: "Status", sortable: true },
        { key: "created_at", label: "Created At", sortable: true },
        { key: "actions", label: "Actions", align: "right" },
    ];

    onMount(async () => {
        loadData();
    });

    async function loadData() {
        loading = true;
        try {
            const [tenantsRes, plansRes] = await Promise.all([
                api.superadmin.listTenants(),
                api.plans.list()
            ]);

            if (Array.isArray(tenantsRes)) {
                tenants = tenantsRes;
            } else if (tenantsRes && Array.isArray(tenantsRes.data)) {
                tenants = tenantsRes.data;
            } else {
                tenants = [];
            }

            // Map plans for Select component
            plans = plansRes
                .filter(p => p.is_active)
                .map(p => ({
                    label: `${p.name} - ${p.price_monthly > 0 ? '$' + p.price_monthly + '/mo' : 'Free'}`,
                    value: p.id
                }));
            
            // Set default plan if needed
            const defaultPlan = plansRes.find(p => p.is_default);
            if (defaultPlan) {
                newTenant.planId = defaultPlan.id;
            }

        } catch (e: any) {
            console.error("Load data error:", e);
            error = e.toString();
            if (e.toString().includes("Unauthorized")) {
                goto("/dashboard");
            }
        } finally {
            loading = false;
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
        } catch (e) {
            console.error("Reload error", e);
        }
    }

    function openCreateModal() {
        isEditing = false;
        editingId = "";
        
        // Find default plan again to reset
        // We need to keep plans state
        
        newTenant = {
            name: "",
            slug: "",
            customDomain: "",
            ownerEmail: "",
            ownerPassword: "",
            isActive: true,
            planId: plans.length > 0 ? plans[0].value : "",
        };
        showCreateModal = true;
    }

    function openEditModal(tenant: any) {
        isEditing = true;
        editingId = tenant.id;
        newTenant = {
            name: tenant.name,
            slug: tenant.slug,
            customDomain: tenant.custom_domain || "",
            ownerEmail: "---", // Email cannot be changed here easily in this view
            ownerPassword: "", // Password not needed for update
            isActive: tenant.is_active,
            planId: "", // Plan cannot be changed here for now (use subscription page)
        };
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
                newTenant.planId || undefined // Pass planId
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
</script>

<div class="page-container fade-in">
    <div class="page-header">
        <div class="header-content">
            <h1>Tenants</h1>
            <p class="subtitle">Manage all organizations in the platform</p>
        </div>
        <button class="btn btn-primary" on:click={openCreateModal}>
            <Icon name="plus" size={20} />
            <span>New Tenant</span>
        </button>
    </div>

    <div class="card content-card">
        {#if error}
            <div class="alert alert-error">
                {error}
            </div>
        {/if}

        <Table
            {loading}
            data={tenants}
            {columns}
            searchable={true}
            searchPlaceholder="Search tenants..."
        >
            {#snippet cell({ item, column })}
                {#if column.key === "custom_domain"}
                    {#if item.custom_domain}
                        <code class="domain-badge">{item.custom_domain}</code>
                    {:else}
                        <span class="text-muted">-</span>
                    {/if}
                {:else if column.key === "is_active"}
                    <span
                        class="status-badge {item.is_active
                            ? 'success'
                            : 'error'}"
                    >
                        {item.is_active ? "Active" : "Inactive"}
                    </span>
                {:else if column.key === "created_at"}
                    {new Date(item.created_at).toLocaleDateString()}
                {:else if column.key === "actions"}
                    <div class="actions">
                        <button
                            class="action-btn"
                            title="Edit"
                            on:click={() => openEditModal(item)}
                        >
                            <Icon name="edit" size={18} />
                        </button>
                        <button
                            class="action-btn delete"
                            title="Delete"
                            on:click={() => confirmDelete(item.id)}
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
</div>

<Modal
    bind:show={showCreateModal}
    title={isEditing ? "Edit Tenant" : "Create New Tenant"}
>
    <div class="modal-form">
        <Input
            label="Tenant Name"
            bind:value={newTenant.name}
            on:input={generateSlug}
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
                    on:click={() => (showPassword = !showPassword)}
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
                on:click={() => (showCreateModal = false)}
                disabled={creating}
            >
                Cancel
            </button>
            <button
                class="btn btn-primary"
                on:click={handleSubmit}
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

<style>
    .page-container {
        padding: 2rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .page-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 2rem;
    }

    .header-content h1 {
        font-size: 1.8rem;
        font-weight: 700;
        margin: 0 0 0.5rem 0;
        color: var(--text-primary);
    }

    .subtitle {
        color: var(--text-secondary);
        font-size: 0.95rem;
        margin: 0;
    }

    .content-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg);
        overflow: hidden;
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

    .action-btn {
        width: 32px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: none;
        background: transparent;
        color: var(--text-secondary);
        border-radius: var(--radius-md);
        cursor: pointer;
        transition: all 0.2s;
    }

    .action-btn:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .action-btn.delete:hover {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
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
        .page-container {
            padding: 1rem;
        }

        .page-header {
            flex-direction: column;
            gap: 1rem;
            align-items: stretch;
        }

        .btn {
            justify-content: center;
        }
    }
</style>