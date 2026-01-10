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
                                <span class="role-pill {u.role}">{u.role}</span>
                            </td>
                            <td class="text-mono">
                                {#if u.tenant_id}
                                    {u.tenant_id.substring(0, 8)}...
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
        background: #1e293b;
        border-radius: 16px;
        border: 1px solid rgba(255, 255, 255, 0.05);
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
    }

    .card-header {
        padding: 1.5rem 2rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        display: flex;
        justify-content: space-between;
        align-items: center;
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
        color: #fff;
    }

    .count-badge {
        background: rgba(255, 255, 255, 0.1);
        color: #94a3b8;
        padding: 0.2rem 0.6rem;
        border-radius: 12px;
        font-size: 0.8rem;
        font-weight: 600;
    }

    .search-box {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        background: #0f172a;
        padding: 0.6rem 1rem;
        border-radius: 8px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        color: #64748b;
    }

    .search-box input {
        background: transparent;
        border: none;
        color: #fff;
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
    }

    .data-table th {
        text-align: left;
        padding: 1rem 2rem;
        font-size: 0.8rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: #64748b;
        font-weight: 600;
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        background: rgba(0, 0, 0, 0.1);
    }

    .data-table td {
        padding: 1.25rem 2rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.03);
        vertical-align: middle;
        color: #e2e8f0;
        font-size: 0.95rem;
    }

    .data-table tr:hover {
        background: rgba(255, 255, 255, 0.02);
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
        color: #fff;
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
        color: #34d399;
        border: 1px solid rgba(16, 185, 129, 0.2);
    }

    .status-pill.inactive {
        background: rgba(239, 68, 68, 0.15);
        color: #f87171;
        border: 1px solid rgba(239, 68, 68, 0.2);
    }

    .dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: currentColor;
    }

    .text-muted {
        color: #64748b;
    }
    .text-mono {
        font-family: monospace;
        font-size: 0.85rem;
        color: #94a3b8;
    }

    .loading-state {
        padding: 4rem;
        text-align: center;
        color: #94a3b8;
    }

    .spinner {
        width: 24px;
        height: 24px;
        border: 3px solid rgba(255, 255, 255, 0.1);
        border-top-color: #6366f1;
        border-radius: 50%;
        margin: 0 auto 1rem auto;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .empty-state {
        text-align: center;
        padding: 4rem;
        color: #64748b;
    }

    .empty-state h3 {
        color: #fff;
        margin: 1rem 0 0.5rem 0;
    }
</style>
