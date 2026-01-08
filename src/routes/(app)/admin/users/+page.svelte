<script lang="ts">
    import { isAuthenticated, isAdmin } from "$lib/stores/auth";
    import { users } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import type { User } from "$lib/api/client";

    let allUsers: User[] = [];
    let loading = true;

    onMount(async () => {
        if (!$isAuthenticated) {
            goto("/login");
            return;
        }
        if (!$isAdmin) {
            goto("/dashboard");
            return;
        }

        try {
            const usersRes = await users.list(1, 100); // Fetch more for the full list page
            allUsers = usersRes.data;
        } catch (err) {
            console.error("Failed to load users:", err);
        } finally {
            loading = false;
        }
    });
</script>

<div class="page-content fade-in">
    <div class="actions-header">
        <button class="btn btn-primary">Add New User</button>
    </div>

    {#if loading}
        <div class="loading">Loading users...</div>
    {:else}
        <div class="card">
            <table class="table">
                <thead>
                    <tr>
                        <th>Name</th>
                        <th>Email</th>
                        <th>Role</th>
                        <th>Status</th>
                        <th>Joined</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {#each allUsers as u}
                        <tr>
                            <td>
                                <div class="user-cell">
                                    <div class="avatar-sm">{u.name.charAt(0).toUpperCase()}</div>
                                    <span>{u.name}</span>
                                </div>
                            </td>
                            <td>{u.email}</td>
                            <td>
                                <span class="badge badge-{u.role}">{u.role}</span>
                            </td>
                            <td>
                                <span class="badge badge-{u.is_active ? 'active' : 'inactive'}">
                                    {u.is_active ? "Active" : "Inactive"}
                                </span>
                            </td>
                            <td class="text-muted">{new Date(u.created_at).toLocaleDateString()}</td>
                            <td>
                                <button class="btn-icon">✏️</button>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</div>

<style>
    .page-content {
        padding: 1.5rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .actions-header {
        margin-bottom: 1.5rem;
        display: flex;
        justify-content: flex-end;
    }

    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        overflow-x: auto;
    }

    .table {
        width: 100%;
        border-collapse: collapse;
        min-width: 800px;
    }

    .table th,
    .table td {
        padding: 1rem;
        text-align: left;
        border-bottom: 1px solid var(--border-color);
    }

    .table th {
        font-weight: 500;
        color: var(--text-secondary);
        font-size: 0.875rem;
        background: rgba(255,255,255,0.02);
    }

    .table tr:last-child td {
        border-bottom: none;
    }

    .user-cell {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        font-weight: 500;
    }

    .avatar-sm {
        width: 32px;
        height: 32px;
        background: var(--bg-hover);
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.8rem;
        color: var(--text-secondary);
    }

    .badge {
        display: inline-block;
        padding: 0.25rem 0.75rem;
        border-radius: 999px;
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: capitalize;
    }

    .badge-admin { background: rgba(99, 102, 241, 0.2); color: var(--color-primary); }
    .badge-user { background: rgba(16, 185, 129, 0.2); color: var(--color-success); }
    .badge-active { background: rgba(34, 197, 94, 0.2); color: var(--color-success); }
    .badge-inactive { background: rgba(239, 68, 68, 0.2); color: var(--color-danger); }

    .text-muted { color: var(--text-secondary); font-size: 0.9rem; }

    .btn-icon {
        background: transparent;
        border: none;
        cursor: pointer;
        font-size: 1rem;
        opacity: 0.7;
        transition: opacity 0.2s;
    }

    .btn-icon:hover { opacity: 1; }

    .btn-primary {
        background: var(--color-primary);
        color: white;
        border: none;
        padding: 0.6rem 1.2rem;
        border-radius: var(--radius-sm);
        font-weight: 600;
        cursor: pointer;
        transition: opacity 0.2s;
    }
    
    .btn-primary:hover { opacity: 0.9; }

    .loading {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 50vh;
        color: var(--text-secondary);
    }

    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: translateY(0); }
    }
</style>