<script lang="ts">
    import { isAdmin } from "$lib/stores/auth";
    import { api } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import { t } from "svelte-i18n";
    import type { Role, Permission } from "$lib/api/client";

    let roles: Role[] = [];
    let permissions: Permission[] = [];
    let loading = true;
    let error = "";

    // Modal state
    let showModal = false;
    let editingRole: Role | null = null;
    let roleName = "";
    let roleDescription = "";
    let selectedPermissions: Set<string> = new Set();
    let saving = false;

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
        if (!$isAdmin) {
            goto("/dashboard");
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
        selectedPermissions = new Set();
        showModal = true;
    }

    function openEditModal(role: Role) {
        editingRole = role;
        roleName = role.name;
        roleDescription = role.description || "";
        selectedPermissions = new Set(role.permissions || []);
        showModal = true;
    }

    function togglePermission(permId: string) {
        // Find permission object to match what API expects
        // But API expects "resource:action" strings for creation/updation
        // OR does it expect IDs?
        // Checking roles.rs: create_role receives Vec<String> which are "resource:action" strings

        // Wait, my Permission interface has id, resource, action.
        // My Role interface has permissions?: string[]. These strings are "resource:action"
        // based on `get_role_permissions` in role_service.rs which returns format!("{}:{}", r, a)

        // So I should work with "resource:action" strings

        if (selectedPermissions.has(permId)) {
            selectedPermissions.delete(permId);
        } else {
            selectedPermissions.add(permId);
        }
        selectedPermissions = selectedPermissions; // trigger reactivity
    }

    async function saveRole() {
        if (!roleName) return;
        saving = true;
        try {
            const permsList = Array.from(selectedPermissions);

            if (editingRole) {
                await api.roles.update(
                    editingRole.id,
                    roleName,
                    roleDescription,
                    permsList,
                );
            } else {
                await api.roles.create(roleName, roleDescription, permsList);
            }

            await loadData();
            showModal = false;
        } catch (e: any) {
            alert("Failed to save role: " + e.message);
        } finally {
            saving = false;
        }
    }

    async function deleteRole(role: Role) {
        if (role.is_system) return; // Cannot delete system roles
        if (
            !confirm(`Are you sure you want to delete the role "${role.name}"?`)
        )
            return;

        try {
            await api.roles.delete(role.id);
            roles = roles.filter((r) => r.id !== role.id);
        } catch (e: any) {
            alert("Failed to delete role: " + e.message);
        }
    }
</script>

<div class="page-content fade-in">
    <div class="content-card" in:fly={{ y: 20, delay: 100 }}>
        <div class="card-header">
            <div class="header-left">
                <h2>Roles & Permissions</h2>
                <span class="count-badge">{roles.length} Roles</span>
            </div>
            <button class="btn btn-primary" on:click={openCreateModal}>
                <Icon name="plus" size={18} />
                Create Role
            </button>
        </div>

        {#if showModal}
            <div
                class="modal-backdrop"
                on:click={() => (showModal = false)}
                transition:fade={{ duration: 200 }}
            >
                <div
                    class="modal-card wide"
                    on:click|stopPropagation
                    transition:fly={{ y: 20, duration: 300 }}
                >
                    <h3>{editingRole ? "Edit Role" : "Create New Role"}</h3>
                    <form on:submit|preventDefault={saveRole}>
                        <div class="form-row">
                            <div class="form-group flex-1">
                                <label>Role Name</label>
                                <input
                                    type="text"
                                    bind:value={roleName}
                                    placeholder="e.g. Editor"
                                    required
                                    disabled={editingRole?.is_system}
                                />
                                {#if editingRole?.is_system}
                                    <small class="text-muted"
                                        >System role names cannot be changed</small
                                    >
                                {/if}
                            </div>
                            <div class="form-group flex-2">
                                <label>Description</label>
                                <input
                                    type="text"
                                    bind:value={roleDescription}
                                    placeholder="Description of what this role can do"
                                />
                            </div>
                        </div>

                        <div class="permissions-section">
                            <h4>Permissions</h4>
                            <div class="permissions-grid">
                                {#each Object.entries(permissionGroups) as [resource, groupPerms]}
                                    <div class="permission-group">
                                        <div class="group-header">
                                            <span class="resource-name"
                                                >{resource}</span
                                            >
                                        </div>
                                        <div class="group-actions">
                                            {#each groupPerms as perm}
                                                {@const permKey = `${perm.resource}:${perm.action}`}
                                                <label class="permission-check">
                                                    <input
                                                        type="checkbox"
                                                        checked={selectedPermissions.has(
                                                            permKey,
                                                        )}
                                                        on:change={() =>
                                                            togglePermission(
                                                                permKey,
                                                            )}
                                                    />
                                                    <span class="perm-label">
                                                        {perm.action}
                                                        {#if perm.description}
                                                            <span
                                                                class="perm-desc"
                                                                title={perm.description}
                                                            >
                                                                <Icon
                                                                    name="help-circle"
                                                                    size={12}
                                                                />
                                                            </span>
                                                        {/if}
                                                    </span>
                                                </label>
                                            {/each}
                                        </div>
                                    </div>
                                {/each}
                            </div>
                        </div>

                        <div class="modal-actions">
                            <button
                                type="button"
                                class="btn btn-glass"
                                on:click={() => (showModal = false)}
                                >Cancel</button
                            >
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled={saving}
                            >
                                {saving ? "Saving..." : "Save Role"}
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        {/if}

        {#if loading}
            <div class="loading-state">
                <div class="spinner"></div>
                <p>Loading roles...</p>
            </div>
        {:else if error}
            <div class="error-state">
                <Icon name="alert-circle" size={48} color="#ef4444" />
                <p>{error}</p>
                <button class="btn btn-glass" on:click={loadData}>Retry</button>
            </div>
        {:else}
            <div class="roles-grid">
                {#each roles as role}
                    <div class="role-card">
                        <div class="role-header">
                            <div>
                                <h3>{role.name}</h3>
                                {#if role.description}
                                    <p class="role-desc">{role.description}</p>
                                {/if}
                            </div>
                            {#if role.is_system}
                                <span class="badge system">System</span>
                            {:else}
                                <span class="badge custom">Custom</span>
                            {/if}
                        </div>

                        <div class="role-permissions-preview">
                            <p>
                                <strong>{role.permissions?.length || 0}</strong>
                                permissions
                            </p>
                        </div>

                        <div class="role-actions">
                            <button
                                class="btn btn-sm btn-glass"
                                on:click={() => openEditModal(role)}
                            >
                                Edit
                            </button>
                            {#if !role.is_system}
                                <button
                                    class="btn btn-sm btn-danger-glass"
                                    on:click={() => deleteRole(role)}
                                >
                                    Delete
                                </button>
                            {/if}
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
</div>

<style>
    .page-content {
        padding: 1.5rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .content-card {
        background: var(--bg-surface, #1e293b);
        border-radius: 16px;
        border: 1px solid var(--border-color, rgba(255, 255, 255, 0.05));
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
        min-height: 400px;
    }

    .card-header {
        padding: 1.5rem 2rem;
        border-bottom: 1px solid var(--border-color, rgba(255, 255, 255, 0.05));
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .header-left {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .header-left h2 {
        margin: 0;
        font-size: 1.25rem;
        color: var(--text-primary, #fff);
    }

    .count-badge {
        background: rgba(255, 255, 255, 0.1);
        color: var(--text-secondary, #94a3b8);
        padding: 0.2rem 0.6rem;
        border-radius: 12px;
        font-size: 0.8rem;
        font-weight: 600;
    }

    /* Roles Grid */
    .roles-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
        gap: 1.5rem;
        padding: 1.5rem;
    }

    .role-card {
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 12px;
        padding: 1.5rem;
        transition: all 0.2s;
    }

    .role-card:hover {
        background: rgba(255, 255, 255, 0.05);
        border-color: rgba(255, 255, 255, 0.1);
        transform: translateY(-2px);
    }

    .role-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 1rem;
    }

    .role-header h3 {
        margin: 0 0 0.25rem 0;
        font-size: 1.1rem;
        color: var(--text-primary, #fff);
    }

    .role-desc {
        margin: 0;
        font-size: 0.85rem;
        color: var(--text-secondary, #94a3b8);
        line-height: 1.4;
    }

    .badge {
        font-size: 0.7rem;
        padding: 0.2rem 0.5rem;
        border-radius: 4px;
        text-transform: uppercase;
        font-weight: 700;
        letter-spacing: 0.05em;
    }

    .badge.system {
        background: rgba(99, 102, 241, 0.2);
        color: #818cf8;
    }

    .badge.custom {
        background: rgba(16, 185, 129, 0.2);
        color: #34d399;
    }

    .role-permissions-preview {
        margin-bottom: 1.5rem;
        font-size: 0.9rem;
        color: var(--text-secondary, #94a3b8);
    }

    .role-actions {
        display: flex;
        gap: 0.5rem;
    }

    /* Modal */
    .modal-backdrop {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.7);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
    }

    .modal-card {
        background: var(--bg-surface, #1e293b);
        padding: 2rem;
        border-radius: 16px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        width: 100%;
        max-width: 500px;
        box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
        max-height: 90vh;
        overflow-y: auto;
    }

    .modal-card.wide {
        max-width: 800px;
    }

    .modal-card h3 {
        margin: 0 0 1.5rem 0;
        font-size: 1.25rem;
        color: var(--text-primary, white);
    }

    .form-row {
        display: flex;
        gap: 1.5rem;
        margin-bottom: 1.5rem;
    }

    .flex-1 {
        flex: 1;
    }
    .flex-2 {
        flex: 2;
    }

    .form-group label {
        display: block;
        margin-bottom: 0.5rem;
        font-size: 0.9rem;
        color: var(--text-secondary, #94a3b8);
    }

    .form-group input {
        width: 100%;
        background: var(--bg-primary, #0f172a);
        border: 1px solid rgba(255, 255, 255, 0.1);
        padding: 0.75rem 1rem;
        border-radius: 8px;
        color: var(--text-primary, white);
        font-size: 1rem;
    }

    .form-group input:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .text-muted {
        color: var(--text-secondary, #64748b);
        font-size: 0.8rem;
    }

    /* Permissions Matrix */
    .permissions-section {
        background: rgba(0, 0, 0, 0.2);
        border-radius: 8px;
        padding: 1.5rem;
        margin-bottom: 2rem;
    }

    .permissions-section h4 {
        margin: 0 0 1rem 0;
        font-size: 1rem;
        color: var(--text-primary, #fff);
    }

    .permissions-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 1.5rem;
    }

    .permission-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .group-header {
        font-weight: 600;
        color: var(--text-primary, #fff);
        text-transform: capitalize;
        border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        padding-bottom: 0.3rem;
        margin-bottom: 0.3rem;
        font-size: 0.9rem;
    }

    .permission-check {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: pointer;
        padding: 0.2rem 0;
    }

    .permission-check:hover .perm-label {
        color: var(--color-primary, #6366f1);
    }

    .permission-check input[type="checkbox"] {
        accent-color: var(--color-primary, #6366f1);
        width: 16px;
        height: 16px;
    }

    .perm-label {
        color: var(--text-secondary, #cbd5e1);
        font-size: 0.9rem;
        text-transform: capitalize;
        display: flex;
        align-items: center;
        gap: 0.3rem;
    }

    .perm-desc {
        opacity: 0.5;
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
    }

    /* Buttons */
    .btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.6rem 1rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        border: none;
        font-size: 0.9rem;
        transition: all 0.2s;
    }

    .btn-sm {
        padding: 0.4rem 0.8rem;
        font-size: 0.8rem;
    }

    .btn-primary {
        background: var(--color-primary, #6366f1);
        color: white;
    }

    .btn-glass {
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-secondary, #cbd5e1);
        border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .btn-glass:hover {
        background: rgba(255, 255, 255, 0.1);
        color: var(--text-primary, #fff);
    }

    .btn-danger-glass {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
        border: 1px solid rgba(239, 68, 68, 0.2);
    }

    .btn-danger-glass:hover {
        background: rgba(239, 68, 68, 0.2);
    }

    .loading-state,
    .error-state {
        text-align: center;
        padding: 3rem;
        color: var(--text-secondary, #94a3b8);
    }

    .spinner {
        width: 24px;
        height: 24px;
        border: 3px solid rgba(255, 255, 255, 0.1);
        border-top-color: var(--color-primary, #6366f1);
        border-radius: 50%;
        margin: 0 auto 1rem auto;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }
</style>
