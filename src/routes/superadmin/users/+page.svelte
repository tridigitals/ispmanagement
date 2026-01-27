<script lang="ts">
    import { isSuperAdmin } from "$lib/stores/auth";
    import { api } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import type { User } from "$lib/api/client";

    let allUsers: User[] = [];
    let loading = true;
    let searchQuery = "";

    $: filteredUsers = searchQuery
        ? allUsers.filter(
              (u) =>
                  u.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                  u.email.toLowerCase().includes(searchQuery.toLowerCase()),
          )
        : allUsers;

    onMount(async () => {
        if (!$isSuperAdmin) {
            goto("/dashboard");
            return;
        }

        try {
            const usersRes = await api.users.list(1, 200);
            allUsers = usersRes.data;
        } catch (err) {
            console.error("Failed to load users:", err);
        } finally {
            loading = false;
        }
    });

    async function reset2FA(u: User) {
        if (
            !confirm(
                `Are you sure you want to reset 2FA for ${u.name}? They will be able to login without a secondary code.`,
            )
        )
            return;

        try {
            await api.auth.resetUser2FA(u.id);
            // Update local state
            allUsers = allUsers.map((user) =>
                user.id === u.id ? { ...user, two_factor_enabled: false } : user,
            );
            alert("Two-factor authentication has been reset.");
        } catch (err: any) {
            alert("Failed to reset 2FA: " + err.message);
        }
    }

    const getInitials = (name: string) => name.substring(0, 2).toUpperCase();
</script>

<div class="content-card" in:fly={{ y: 20, delay: 100 }}>
    <div class="card-header">
        <div class="header-left">
            <h2>All Users</h2>
            <span class="count-badge">{allUsers.length} Total</span>
        </div>
        <div class="search-box">
            <Icon name="search" size={18} />
            <input
                type="text"
                placeholder="Search users..."
                bind:value={searchQuery}
            />
        </div>
    </div>

    {#if loading}
        <div class="loading-state">
            <div class="spinner"></div>
            <p>Loading users...</p>
        </div>
    {:else}
        <div class="table-responsive">
            <table class="data-table">
                <thead>
                    <tr>
                        <th>User</th>
                        <th>Email</th>
                        <th>Role</th>
                        <th>Tenant ID</th>
                        <th>Status</th>
                        <th>Joined</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {#each filteredUsers as u}
                        <tr class="fade-in-row">
                            <td>
                                <div class="user-info">
                                    <div class="avatar">
                                        {getInitials(u.name)}
                                    </div>
                                    <span class="user-name">{u.name}</span>
                                </div>
                            </td>
                            <td>{u.email}</td>
                            <td>
                                {#if u.is_super_admin}
                                    <span class="role-pill superadmin"
                                        >Super Admin</span
                                    >
                                {:else if u.tenant_role}
                                    <span
                                        class="role-pill {u.tenant_role.toLowerCase()}"
                                        >{u.tenant_role}</span
                                    >
                                    {#if u.role !== "user" && u.role !== "admin" && u.role !== u.tenant_role.toLowerCase()}
                                        <span class="text-xs text-muted"
                                            >({u.role})</span
                                        >
                                    {/if}
                                {:else}
                                    <span class="role-pill {u.role}"
                                        >{u.role}</span
                                    >
                                {/if}
                            </td>
                            <td class="text-mono">
                                {#if u.tenant_slug}
                                    {u.tenant_slug}
                                {:else}
                                    <span class="text-muted">—</span>
                                {/if}
                            </td>
                            <td>
                                {#if u.is_active}
                                    <span class="status-pill active">
                                        <span class="dot"></span> Active
                                    </span>
                                {:else}
                                    <span class="status-pill inactive">
                                        <span class="dot"></span> Inactive
                                    </span>
                                {/if}
                            </td>
                            <td class="text-muted"
                                >{new Date(
                                    u.created_at,
                                ).toLocaleDateString()}</td
                            >
                            <td>
                                <div class="actions">
                                    {#if u.two_factor_enabled}
                                        <button
                                            class="btn-icon warning"
                                            onclick={() => reset2FA(u)}
                                            title="Reset 2FA"
                                        >
                                            <Icon name="shield-off" size={16} />
                                        </button>
                                    {/if}
                                </div>
                            </td>
                        </tr>
                    {:else}
                        <tr>
                            <td colspan="6" class="empty-state">
                                <Icon name="users" size={48} />
                                <h3>No Users Found</h3>
                                <p>No users match your search criteria.</p>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</div>

<style>
    .content-card {
        background: var(--bg-surface);
        border-radius: var(--radius-lg);
        border: 1px solid var(--border-color);
        box-shadow: var(--shadow-sm);
    }

    .card-header {
        padding: 1.5rem 2rem;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        justify-content: space-between;
        align-items: center;
        flex-wrap: wrap;
        gap: 1rem;
    }

    @media (max-width: 640px) {
        .card-header {
            padding: 1rem;
            flex-direction: column;
            align-items: flex-start;
        }

        .header-left {
            width: 100%;
            justify-content: space-between;
        }

        .search-box {
            width: 100%;
        }

        .search-box input {
            width: 100%;
        }
    }

    .header-left {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .card-header h2 {
        font-size: 1.25rem;
        font-weight: 700;
        margin: 0;
        color: var(--text-primary);
    }

    .count-badge {
        background: var(--bg-active);
        color: var(--text-secondary);
        padding: 0.2rem 0.6rem;
        border-radius: 12px;
        font-size: 0.8rem;
        font-weight: 600;
    }

    .search-box {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        background: var(--bg-app);
        padding: 0.6rem 1rem;
        border-radius: 8px;
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
        transition: border-color 0.2s;
    }

    .search-box:focus-within {
        border-color: var(--color-primary);
        color: var(--text-primary);
    }

    .search-box input {
        background: transparent;
        border: none;
        color: var(--text-primary);
        outline: none;
        width: 200px;
    }

    .table-responsive {
        width: 100%;
        overflow-x: auto;
    }

    .data-table {
        width: 100%;
        border-collapse: collapse;
        min-width: 900px; /* Ensure scroll on mobile */
    }

    .data-table th {
        text-align: left;
        padding: 1rem 2rem;
        font-size: 0.8rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--text-secondary);
        font-weight: 600;
        border-bottom: 1px solid var(--border-color);
        background: var(--bg-hover);
    }

    .data-table td {
        padding: 1.25rem 2rem;
        border-bottom: 1px solid var(--border-subtle);
        vertical-align: middle;
        color: var(--text-primary);
        font-size: 0.95rem;
    }

    .data-table tr:hover {
        background: var(--bg-hover);
    }

    .user-info {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .avatar {
        width: 40px;
        height: 40px;
        background: linear-gradient(135deg, #475569, #334155);
        border-radius: 10px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 700;
        color: white;
        font-size: 0.9rem;
    }

    .user-name {
        font-weight: 600;
        color: var(--text-primary);
    }

    .role-pill {
        padding: 0.3rem 0.8rem;
        border-radius: 20px;
        font-size: 0.8rem;
        font-weight: 600;
        text-transform: capitalize;
    }

    .role-pill.admin {
        background: rgba(99, 102, 241, 0.2);
        color: #818cf8;
    }

    .role-pill.superadmin {
        background: rgba(139, 92, 246, 0.2);
        color: #a78bfa;
        border: 1px solid rgba(139, 92, 246, 0.3);
    }

    .role-pill.user {
        background: rgba(16, 185, 129, 0.2);
        color: #34d399;
    }

    .status-pill {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.35rem 0.8rem;
        border-radius: 20px;
        font-size: 0.8rem;
        font-weight: 600;
    }

    .status-pill.active {
        background: rgba(16, 185, 129, 0.15);
        color: var(--color-success);
        border: 1px solid rgba(16, 185, 129, 0.2);
    }

    .status-pill.inactive {
        background: rgba(239, 68, 68, 0.15);
        color: var(--color-danger);
        border: 1px solid rgba(239, 68, 68, 0.2);
    }

    .dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: currentColor;
    }

    .text-muted {
        color: var(--text-muted);
    }
    .text-mono {
        font-family: monospace;
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .loading-state {
        padding: 4rem;
        text-align: center;
        color: var(--text-secondary);
    }

    .spinner {
        width: 24px;
        height: 24px;
        border: 3px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        margin: 0 auto 1rem auto;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .btn-icon:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .btn-icon.warning:hover {
        background: rgba(245, 158, 11, 0.15);
        color: #f59e0b;
    }

    .actions {
        display: flex;
        gap: 0.5rem;
    }

    .empty-state {
        text-align: center;
        padding: 4rem;
        color: var(--text-secondary);
    }

    .empty-state h3 {
        color: var(--text-primary);
        margin: 1rem 0 0.5rem 0;
    }
</style>
