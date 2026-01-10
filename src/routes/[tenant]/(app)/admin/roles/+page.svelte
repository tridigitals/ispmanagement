<script lang="ts">
    import { isAdmin, checkAuth, can } from "$lib/stores/auth";
    import { api } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import Table from "$lib/components/Table.svelte";
    import TableToolbar from "$lib/components/TableToolbar.svelte";
    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
    import { t } from "svelte-i18n";
    import type { Role, Permission } from "$lib/api/client";
    import { toast } from "$lib/stores/toast";

    let roles: Role[] = [];
    let permissions: Permission[] = [];
    let loading = true;

    let error = "";

    // Search
    let searchQuery = "";

    $: filteredRoles = roles.filter(
        (role) =>
            role.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            (role.description || "")
                .toLowerCase()
                .includes(searchQuery.toLowerCase()),
    );

    // Table Columns

    // Table Columns
    const columns = [
        { key: "name", label: "Role Name" },
        { key: "type", label: "Type", width: "120px" },
        { key: "level", label: "Level", width: "80px" },
        { key: "permissions", label: "Permissions" },
        { key: "actions", label: "", align: "right" as const, width: "140px" },
    ];

    // Modal state
    let showModal = false;
    let editingRole: Role | null = null;
    let roleName = "";
    let roleDescription = "";
    let roleLevel = 0;
    let selectedPermissions: Set<string> = new Set();
    let saving = false;

    // Confirm Dialog State
    let showConfirm = false;
    let deletingRole: Role | null = null;
    let deleting = false;

    // Group permissions by resource for better UI
    $: permissionGroups = groupPermissions(permissions);

    function groupPermissions(perms: Permission[]) {
        const groups: Record<string, Permission[]> = {};
        for (const p of perms) {
            if (!groups[p.resource]) {
                groups[p.resource] = [];
            }
            groups[p.resource].push(p);
        }
        return groups;
    }

    onMount(async () => {
        if (!$isAdmin || !$can("read", "roles")) {
            goto("/unauthorized");
            return;
        }
        await loadData();
    });

    async function loadData() {
        loading = true;
        try {
            const [rolesRes, permsRes] = await Promise.all([
                api.roles.list(),
                api.roles.getPermissions(),
            ]);
            roles = rolesRes;
            permissions = permsRes;
        } catch (e: any) {
            error = e.toString();
        } finally {
            loading = false;
        }
    }

    function openCreateModal() {
        editingRole = null;
        roleName = "";
        roleDescription = "";
        roleLevel = 0;
        selectedPermissions = new Set();
        showModal = true;
    }

    function openEditModal(role: Role) {
        editingRole = role;
        roleName = role.name;
        roleDescription = role.description || "";
        roleLevel = role.level || 0;
        selectedPermissions = new Set(role.permissions || []);
        showModal = true;
    }

    function togglePermission(permId: string) {
        if (selectedPermissions.has(permId)) {
            selectedPermissions.delete(permId);
        } else {
            selectedPermissions.add(permId);
        }
        selectedPermissions = selectedPermissions;
    }

    function toggleGroup(resource: string, groupPerms: Permission[]) {
        const allKeys = groupPerms.map((p) => `${p.resource}:${p.action}`);
        const allSelected = allKeys.every((k) => selectedPermissions.has(k));

        if (allSelected) {
            allKeys.forEach((k) => selectedPermissions.delete(k));
        } else {
            allKeys.forEach((k) => selectedPermissions.add(k));
        }
        selectedPermissions = selectedPermissions;
    }

    async function saveRole() {
        if (!roleName) return;
        saving = true;
        try {
            const permsArray = Array.from(selectedPermissions);

            if (editingRole) {
                await api.roles.update(
                    editingRole.id,
                    roleName,
                    roleDescription,
                    roleLevel,
                    permsArray,
                );
                toast.success("Role updated successfully");
            } else {
                await api.roles.create(
                    roleName,
                    roleDescription,
                    roleLevel,
                    permsArray,
                );
                toast.success("Role created successfully");
            }

            await loadData();
            showModal = false;
        } catch (e: any) {
            toast.error(e.message || "Failed to save role");
            console.error(e);
        } finally {
            saving = false;
        }
    }

    function confirmDelete(role: Role) {
        deletingRole = role;
        showConfirm = true;
    }

    async function executeDelete() {
        if (!deletingRole) return;
        deleting = true;
        try {
            await api.roles.delete(deletingRole.id);
            toast.success("Role deleted successfully");
            await loadData();
            showConfirm = false;
        } catch (e: any) {
            toast.error(e.message || "Failed to delete role");
        } finally {
            deleting = false;
            deletingRole = null;
        }
    }
</script>

<div class="page-content fade-in">
    <div class="header-section">
        <div class="header-left">
            <h2>Roles & Permissions</h2>
            <p>Manage user roles and access control</p>
        </div>
    </div>
    <div class="content-card">
        <div class="card-header">
            <h3>All Roles</h3>
            <span class="count-badge">{filteredRoles.length} roles</span>
        </div>

        <div class="toolbar-wrapper">
            <TableToolbar bind:searchQuery placeholder="Search roles...">
                <div slot="actions">
                    {#if $can("create", "roles")}
                        <button
                            class="btn btn-primary"
                            on:click={openCreateModal}
                        >
                            <Icon name="plus" size={18} />
                            Create Role
                        </button>
                    {/if}
                </div>
            </TableToolbar>
        </div>

        <div class="table-wrapper">
            <Table
                pagination={true}
                {columns}
                data={filteredRoles}
                {loading}
                emptyText="No roles found"
            >
                <svelte:fragment slot="cell" let:item let:key>
                    {#if key === "name"}
                        <div class="role-info">
                            <span class="role-name">{item.name}</span>
                            {#if item.description}
                                <span class="role-desc">{item.description}</span
                                >
                            {/if}
                        </div>
                    {:else if key === "type"}
                        {#if item.is_system}
                            <span class="badge system">System</span>
                        {:else}
                            <span class="badge custom">Custom</span>
                        {/if}
                    {:else if key === "level"}
                        <span class="level-badge">Lvl {item.level}</span>
                    {:else if key === "permissions"}
                        <span class="perm-count">
                            <Icon name="lock" size={14} />
                            {item.permissions?.length || 0} permissions
                        </span>
                    {:else if key === "actions"}
                        <div class="action-buttons-cell">
                            {#if !item.is_system && $can("delete", "roles")}
                                <button
                                    class="btn-icon danger"
                                    title="Delete Role"
                                    on:click={() => confirmDelete(item)}
                                >
                                    <Icon name="trash" size={18} />
                                    <span class="btn-text">Delete</span>
                                </button>
                            {/if}
                            {#if $can("update", "roles")}
                                <button
                                    class="btn-icon primary"
                                    title="Edit Role"
                                    on:click={() => openEditModal(item)}
                                >
                                    <Icon name="edit" size={18} />
                                    <span class="btn-text">Edit Role</span>
                                </button>
                            {/if}
                        </div>
                    {/if}
                </svelte:fragment>
            </Table>
        </div>
    </div>
</div>

{#if showModal}
    <div
        class="modal-backdrop"
        role="button"
        tabindex="0"
        on:click={() => (showModal = false)}
        on:keydown={(e) => e.key === "Escape" && (showModal = false)}
        transition:fade={{ duration: 200 }}
    >
        <div
            class="modal-card wide"
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            on:click|stopPropagation
            on:keydown|stopPropagation
            transition:fly={{ y: 20, duration: 300 }}
        >
            <div class="modal-header">
                <h3>{editingRole ? "Edit Role" : "Create New Role"}</h3>
                <button class="close-btn" on:click={() => (showModal = false)}>
                    <Icon name="x" size={20} />
                </button>
            </div>

            <div class="modal-body-scroll">
                <form on:submit|preventDefault={saveRole} id="roleForm">
                    <div class="form-row">
                        <div class="form-group flex-1">
                            <label for="role-name">Role Name</label>
                            <input
                                id="role-name"
                                type="text"
                                bind:value={roleName}
                                required
                                disabled={editingRole?.is_system}
                                placeholder="e.g. Editor"
                            />
                            {#if editingRole?.is_system}
                                <small class="text-muted"
                                    >System role names cannot be changed</small
                                >
                            {/if}
                        </div>
                        <div class="form-group flex-2">
                            <label for="role-desc">Description</label>
                            <input
                                id="role-desc"
                                type="text"
                                bind:value={roleDescription}
                                placeholder="Role description"
                            />
                        </div>
                        <div class="form-group" style="flex: 0 0 100px;">
                            <label for="role-level">Level</label>
                            <input
                                id="role-level"
                                type="number"
                                min="0"
                                max="100"
                                bind:value={roleLevel}
                            />
                        </div>
                    </div>

                    <div class="permissions-section">
                        <h4>Permissions Settings</h4>
                        <div class="permissions-container">
                            {#each Object.entries(permissionGroups) as [resource, groupPerms]}
                                {@const allSelected = groupPerms.every((p) =>
                                    selectedPermissions.has(
                                        `${p.resource}:${p.action}`,
                                    ),
                                )}
                                <div class="resource-row">
                                    <div class="resource-header">
                                        <div class="resource-info">
                                            <span class="resource-name"
                                                >{resource}</span
                                            >
                                            <span class="resource-hint"
                                                >Manage permissions for {resource.toLowerCase()}</span
                                            >
                                        </div>
                                        <label class="select-all-toggle">
                                            <input
                                                type="checkbox"
                                                checked={allSelected}
                                                on:change={() =>
                                                    toggleGroup(
                                                        resource,
                                                        groupPerms,
                                                    )}
                                            />
                                            <span class="select-all-text"
                                                >Select All</span
                                            >
                                        </label>
                                    </div>
                                    <div class="perms-list">
                                        {#each groupPerms as perm}
                                            {@const permKey = `${perm.resource}:${perm.action}`}
                                            {@const isChecked =
                                                selectedPermissions.has(
                                                    permKey,
                                                )}
                                            <label
                                                class="perm-toggle {isChecked
                                                    ? 'checked'
                                                    : ''}"
                                            >
                                                <input
                                                    type="checkbox"
                                                    checked={isChecked}
                                                    on:change={() =>
                                                        togglePermission(
                                                            permKey,
                                                        )}
                                                />
                                                <div class="toggle-track">
                                                    <div
                                                        class="toggle-thumb"
                                                    ></div>
                                                </div>
                                                <span class="perm-action"
                                                    >{perm.action}</span
                                                >
                                            </label>
                                        {/each}
                                    </div>
                                </div>
                            {/each}
                        </div>
                    </div>
                </form>
            </div>

            <div class="modal-actions">
                <button
                    type="button"
                    class="btn btn-glass"
                    on:click={() => (showModal = false)}>Cancel</button
                >
                <button
                    type="submit"
                    form="roleForm"
                    class="btn btn-primary"
                    disabled={saving}
                >
                    {#if saving}
                        <div class="spinner-sm"></div>
                        Saving...
                    {:else}
                        Save Role
                    {/if}
                </button>
            </div>
        </div>
    </div>
{/if}

<ConfirmDialog
    bind:show={showConfirm}
    title="Delete Role"
    message={`Are you sure you want to delete role "${deletingRole?.name}"? This action cannot be undone.`}
    confirmText="Delete Role"
    cancelText="Cancel"
    loading={deleting}
    on:confirm={executeDelete}
/>

<style>
    .page-content {
        padding: 1.5rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .header-section {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 2rem;
    }

    .header-left h2 {
        margin: 0;
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary);
    }

    .header-left p {
        margin: 0.25rem 0 0;
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .content-card {
        background: var(--bg-surface);
        border-radius: 16px;
        border: 1px solid var(--border-color);
        box-shadow: var(--shadow-sm);
        overflow: hidden;
    }

    .card-header {
        padding: 1rem 1.5rem;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: rgba(0, 0, 0, 0.2);
    }

    .card-header h3 {
        margin: 0;
        font-size: 1rem;
        font-weight: 600;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .count-badge {
        background: var(--bg-tertiary);
        color: var(--text-secondary);
        padding: 0.2rem 0.6rem;
        border-radius: 12px;
        font-size: 0.75rem;
        font-weight: 600;
    }

    /* Table Content Styles */
    .role-info {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    .role-name {
        font-weight: 600;
        color: var(--text-primary);
        font-size: 0.95rem;
    }
    .role-desc {
        font-size: 0.8rem;
        color: var(--text-secondary);
    }

    .badge {
        font-size: 0.75rem;
        padding: 0.25rem 0.75rem;
        border-radius: 8px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }
    .badge.system {
        background: rgba(99, 102, 241, 0.15);
        color: #818cf8;
        border: 1px solid rgba(99, 102, 241, 0.2);
    }
    .badge.custom {
        background: rgba(16, 185, 129, 0.15);
        color: #34d399;
        border: 1px solid rgba(16, 185, 129, 0.2);
    }

    .perm-count {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .action-buttons-cell {
        display: flex;
        gap: 0.5rem;
        justify-content: flex-end;
        width: 100%;
    }
    .btn-icon.primary:hover {
        background: rgba(99, 102, 241, 0.1);
        color: #6366f1;
    }
    .btn-icon {
        width: 32px;
        height: 32px;
        border-radius: 6px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: none;
        background: transparent;
        color: var(--text-secondary);
        cursor: pointer;
        transition: all 0.2s;
    }
    .btn-icon:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }
    .btn-icon.danger:hover {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
    }

    .btn-text {
        display: none;
    }

    @media (max-width: 768px) {
        .btn-text {
            display: inline-block;
            font-size: 0.85rem;
            font-weight: 600;
            margin-left: 0.5rem;
        }

        .action-buttons-cell .btn-icon {
            width: auto;
            height: 36px;
            padding: 0 1rem;
            background: var(--bg-surface-alt);
            border: 1px solid var(--border-color);
        }

        .action-buttons-cell .btn-icon.primary {
            background: var(--color-primary);
            color: white;
            border: none;
        }

        .action-buttons-cell .btn-icon.primary:hover {
            background: var(--color-primary-hover);
            color: white;
        }

        .action-buttons-cell .btn-icon.danger {
            background: transparent;
            border: 1px solid rgba(239, 68, 68, 0.3);
            color: #ef4444;
        }
    }

    /* Modal Styles */
    .modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.7);
        z-index: 100;
        display: flex;
        align-items: center;
        justify-content: center;
        backdrop-filter: blur(4px);
    }
    .modal-card {
        background: var(--bg-surface);
        width: 100%;
        max-width: 500px;
        max-height: 90vh;
        border-radius: 16px;
        display: flex;
        flex-direction: column;
        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
        border: 1px solid var(--border-color);
    }
    .modal-card.wide {
        max-width: 900px;
    }
    .modal-header {
        padding: 1.25rem 1.5rem;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    .modal-header h3 {
        margin: 0;
        font-size: 1.25rem;
        font-weight: 600;
    }
    .modal-body-scroll {
        padding: 1.5rem;
        overflow-y: auto;
        flex: 1;
    }
    .modal-actions {
        padding: 1.25rem 1.5rem;
        border-top: 1px solid var(--border-color);
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
        background: rgba(0, 0, 0, 0.2);
    }

    .close-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0.25rem;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 4px;
    }
    .close-btn:hover {
        background: rgba(255, 255, 255, 0.1);
        color: var(--text-primary);
    }

    .form-row {
        display: flex;
        gap: 1.5rem;
        margin-bottom: 1.5rem;
    }
    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .flex-1 {
        flex: 1;
    }
    .flex-2 {
        flex: 2;
    }

    label {
        font-size: 0.85rem;
        color: var(--text-secondary);
        font-weight: 500;
    }
    input[type="text"],
    input[type="number"] {
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        padding: 0.6rem 1rem;
        border-radius: 8px;
        color: var(--text-primary);
        transition: border-color 0.2s;
    }
    input[type="text"]:focus,
    input[type="number"]:focus {
        outline: none;
        border-color: var(--color-primary);
    }
    input[type="text"]:disabled,
    input[type="number"]:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .permissions-section h4 {
        margin: 0 0 1rem 0;
        font-size: 1rem;
        color: var(--text-primary);
    }
    .permissions-container {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .resource-row {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.25rem;
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .resource-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding-bottom: 0.75rem;
        border-bottom: 1px dashed var(--border-color);
    }

    .resource-info {
        display: flex;
        flex-direction: column;
        gap: 0.2rem;
    }

    .select-all-toggle {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: pointer;
        font-size: 0.85rem;
        color: var(--color-primary);
        font-weight: 600;
        user-select: none;
    }

    .select-all-toggle input {
        accent-color: var(--color-primary);
        width: 16px;
        height: 16px;
        cursor: pointer;
    }

    .resource-name {
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        font-size: 0.85rem;
        color: var(--text-primary);
    }
    .resource-hint {
        font-size: 0.8rem;
        color: var(--text-secondary);
    }

    .perms-list {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
    }

    .perm-toggle {
        display: inline-flex;
        align-items: center;
        gap: 0.6rem;
        cursor: pointer;
        user-select: none;
        padding: 0.4rem 0.8rem 0.4rem 0.4rem;
        border-radius: 20px;
        background: var(--bg-active);
        border: 1px solid transparent;
        transition: all 0.2s;
    }

    .perm-toggle:hover {
        background: rgba(255, 255, 255, 0.05);
        border-color: var(--border-color);
    }

    .perm-toggle.checked {
        background: rgba(99, 102, 241, 0.1);
        border-color: rgba(99, 102, 241, 0.3);
    }

    .perm-toggle input {
        display: none;
    }

    .toggle-track {
        width: 36px;
        height: 20px;
        background: var(--bg-primary);
        border-radius: 20px;
        position: relative;
        transition: background 0.2s;
        border: 1px solid var(--border-color);
    }

    .toggle-thumb {
        position: absolute;
        width: 16px;
        height: 16px;
        background: var(--text-secondary);
        border-radius: 50%;
        top: 1px;
        left: 1px;
        transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .perm-toggle.checked .toggle-track {
        background: var(--color-primary);
        border-color: var(--color-primary);
    }

    .perm-toggle.checked .toggle-thumb {
        transform: translateX(16px);
        background: white;
    }

    .perm-action {
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--text-secondary);
        text-transform: capitalize;
    }

    .perm-toggle.checked .perm-action {
        color: var(--text-primary);
    }

    .btn {
        padding: 0.6rem 1.2rem;
        border-radius: 8px;
        font-weight: 500;
        cursor: pointer;
        border: none;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 0.9rem;
        transition: all 0.2s;
    }
    .btn:disabled {
        opacity: 0.7;
        cursor: wait;
    }
    .btn-primary {
        background: var(--color-primary);
        color: white;
    }
    .btn-glass {
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-secondary);
        border: 1px solid var(--border-color);
    }
    .btn-glass:hover {
        background: rgba(255, 255, 255, 0.1);
        color: var(--text-primary);
    }

    .spinner-sm {
        width: 14px;
        height: 14px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @media (max-width: 640px) {
        .page-content {
            padding: 1rem;
        }
        .header-section {
            flex-direction: column;
            align-items: stretch;
            gap: 1rem;
            margin-bottom: 1.5rem;
        }
        .header-left {
            text-align: center;
        }

        .form-row {
            flex-direction: column;
            gap: 1rem;
        }

        .modal-card.wide {
            height: 100%;
            border-radius: 0;
            max-height: 100vh;
        }
    }
</style>
