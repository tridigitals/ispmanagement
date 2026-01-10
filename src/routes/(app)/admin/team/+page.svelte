<script lang="ts">
    import { isAdmin, user, can } from "$lib/stores/auth";
    import { api } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import StatsCard from "$lib/components/StatsCard.svelte";
    import Modal from "$lib/components/Modal.svelte";
    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
    import { toast } from "$lib/stores/toast";
    import { t } from "svelte-i18n";
    import type { TeamMember, Role } from "$lib/api/client";

    let teamMembers: TeamMember[] = [];
    let roles: Role[] = [];
    let loading = true;
    let error = "";

    // Filters & Search
    let searchQuery = "";
    let roleFilter = "all";

    // Invite modal
    let showInviteModal = false;
    let inviteEmail = "";
    let inviteName = "";
    let inviteRoleId = "";
    let invitePassword = ""; // Optional password for new user
    let inviting = false;

    // Remove Confirmation
    let showRemoveConfirm = false;
    let memberToRemove: string | null = null;
    let removing = false;

    $: filteredMembers = teamMembers.filter((m) => {
        const matchesSearch =
            m.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            m.email.toLowerCase().includes(searchQuery.toLowerCase());
        const matchesRole = roleFilter === "all" || m.role_id === roleFilter;
        return matchesSearch && matchesRole;
    });

    $: stats = {
        total: teamMembers.length,
        active: teamMembers.filter((m) => m.is_active).length,
        inactive: teamMembers.filter((m) => !m.is_active).length,
    };

    onMount(async () => {
        if (!$can("read", "team")) {
            goto("/dashboard");
            return;
        }
        await loadData();
    });

    async function loadData() {
        loading = true;
        try {
            const [membersRes, rolesRes] = await Promise.all([
                api.team.list(),
                api.roles.list(),
            ]);
            teamMembers = membersRes;
            roles = rolesRes;

            // Set default role for invite
            if (roles.length > 0 && !inviteRoleId) {
                // Try to find "Member" role, otherwise first one
                const memberRole = roles.find((r) => r.name === "Member");
                inviteRoleId = memberRole ? memberRole.id : roles[0].id;
            }
        } catch (e: any) {
            error = e.toString();
            console.error("Failed to load team data:", e);
            toast.error("Failed to load team data");
        } finally {
            loading = false;
        }
    }

    async function inviteMember() {
        if (!inviteEmail || !inviteName || !inviteRoleId) return;
        inviting = true;
        try {
            await api.team.add(
                inviteEmail,
                inviteName,
                inviteRoleId,
                invitePassword,
            );
            await loadData(); // Reload list
            showInviteModal = false;
            inviteEmail = "";
            inviteName = "";
            invitePassword = "";
            // Keep role selection
            toast.success("Team member added successfully");
        } catch (e: any) {
            toast.error("Failed to add member: " + e.message);
        } finally {
            inviting = false;
        }
    }

    function confirmRemove(memberId: string) {
        memberToRemove = memberId;
        showRemoveConfirm = true;
    }

    async function removeMember() {
        if (!memberToRemove) return;
        removing = true;
        try {
            await api.team.remove(memberToRemove);
            teamMembers = teamMembers.filter((m) => m.id !== memberToRemove);
            toast.success("Member removed successfully");
            showRemoveConfirm = false;
            memberToRemove = null;
        } catch (e: any) {
            toast.error("Failed to remove member: " + e.message);
        } finally {
            removing = false;
        }
    }

    // Role updates are disabled in table view for now
    // async function updateRole(memberId: string, roleId: string) {
    //     try {
    //         await api.team.updateRole(memberId, roleId);
    //         // Update local state
    //         const member = teamMembers.find((m) => m.id === memberId);
    //         if (member) {
    //             const role = roles.find((r) => r.id === roleId);
    //             if (role) {
    //                 member.role_id = role.id;
    //                 member.role_name = role.name;
    //                 teamMembers = [...teamMembers]; // Trigger reactivity
    //                 toast.success(`Role updated to ${role.name}`);
    //             }
    //         }
    //     } catch (e: any) {
    //         toast.error("Failed to update role: " + e.message);
    //         await loadData(); // Reload to revert logic
    //     }
    // }

    const getInitials = (name: string) => name.substring(0, 2).toUpperCase();
</script>

<div class="page-content fade-in">
    <!-- Stats Row -->
    <div class="stats-row" in:fly={{ y: 20, duration: 400 }}>
        <StatsCard
            title="Total Members"
            value={stats.total}
            icon="users"
            color="primary"
        />
        <StatsCard
            title="Active Members"
            value={stats.active}
            icon="check-circle"
            color="success"
        />
        <StatsCard
            title="Inactive Members"
            value={stats.inactive}
            icon="slash"
            color="warning"
        />
    </div>

    <div class="content-card" in:fly={{ y: 20, delay: 100 }}>
        <div class="card-header">
            <div class="header-left">
                <h2>{$t("admin.team.title") || "Team Members"}</h2>
                <div class="search-bar">
                    <Icon name="search" size={16} class="icon" />
                    <input
                        type="text"
                        bind:value={searchQuery}
                        placeholder="Search members..."
                    />
                </div>
                <div class="filter-dropdown">
                    <select bind:value={roleFilter}>
                        <option value="all">All Roles</option>
                        {#each roles as role}
                            <option value={role.id}>{role.name}</option>
                        {/each}
                    </select>
                </div>
            </div>
            {#if $can("create", "team")}
                <button
                    class="btn btn-primary"
                    on:click={() => (showInviteModal = true)}
                >
                    <Icon name="plus" size={18} />
                    {$t("admin.team.invite_button") || "Add Member"}
                </button>
            {/if}
        </div>

        {#if loading}
            <div class="loading-state">
                <div class="spinner"></div>
                <p>Loading members...</p>
            </div>
        {:else if error}
            <div class="error-state">
                <Icon name="alert-circle" size={48} color="#ef4444" />
                <p>{error}</p>
                <button class="btn btn-glass" on:click={loadData}>Retry</button>
            </div>
        {:else if filteredMembers.length === 0}
            <div class="empty-state-container">
                <div class="empty-icon">
                    <Icon name="users" size={64} />
                </div>
                <h3>No members found</h3>
                <p>Try adjusting your search or filters.</p>
            </div>
        {:else}
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>Member</th>
                            <th>Role</th>
                            <th>Status</th>
                            <th>Date Added</th>
                            <th class="text-right">Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each filteredMembers as member (member.id)}
                            <tr transition:fade>
                                <td>
                                    <div class="member-info">
                                        <div class="avatar">
                                            {getInitials(member.name)}
                                        </div>
                                        <div>
                                            <div class="member-name">
                                                {member.name}
                                                {#if member.email === $user?.email}
                                                    <span class="you-badge"
                                                        >YOU</span
                                                    >
                                                {/if}
                                            </div>
                                            <div
                                                class="text-muted"
                                                style="font-size: 0.85rem"
                                            >
                                                {member.email}
                                            </div>
                                        </div>
                                    </div>
                                </td>
                                <td>
                                    <span
                                        class="role-pill {member.role_name?.toLowerCase() ||
                                            'member'}"
                                    >
                                        {member.role_name || "Member"}
                                    </span>
                                </td>
                                <td>
                                    <span
                                        class="status-pill {member.is_active
                                            ? 'active'
                                            : 'inactive'}"
                                    >
                                        <span class="dot"></span>
                                        {member.is_active
                                            ? "Active"
                                            : "Inactive"}
                                    </span>
                                </td>
                                <td>
                                    {new Date(
                                        member.created_at,
                                    ).toLocaleDateString()}
                                </td>
                                <td class="text-right">
                                    {#if member.email !== $user?.email && $can("delete", "team")}
                                        <button
                                            class="action-btn danger"
                                            title="Remove Member"
                                            on:click={() =>
                                                confirmRemove(member.id)}
                                        >
                                            <Icon name="trash" size={18} />
                                        </button>
                                    {/if}
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </div>
</div>

<!-- Add Member Modal -->
<Modal
    show={showInviteModal}
    title={$t("admin.team.add_member_modal_title") || "Add Team Member"}
    on:close={() => (showInviteModal = false)}
>
    <form on:submit|preventDefault={inviteMember}>
        <div class="form-group">
            <label>
                {$t("admin.team.name_label") || "Name"}
                <input
                    type="text"
                    bind:value={inviteName}
                    placeholder="John Doe"
                    required
                />
            </label>
        </div>
        <div class="form-group">
            <label>
                {$t("admin.team.email_label") || "Email Address"}
                <input
                    type="email"
                    bind:value={inviteEmail}
                    placeholder="colleague@company.com"
                    required
                />
            </label>
        </div>
        <div class="form-group">
            <label>
                {$t("admin.team.password_label") || "Password (Optional)"}
                <input
                    type="text"
                    bind:value={invitePassword}
                    placeholder="Auto-generated if empty"
                />
            </label>
        </div>
        <div class="form-group">
            <label>
                {$t("admin.team.role_label") || "Role"}
                <select bind:value={inviteRoleId} required>
                    {#each roles as role}
                        <option value={role.id}>{role.name}</option>
                    {/each}
                </select>
            </label>
        </div>
        <div class="modal-actions">
            <button
                type="button"
                class="btn btn-ghost"
                on:click={() => (showInviteModal = false)}
            >
                {$t("admin.team.cancel") || "Cancel"}
            </button>
            <button type="submit" class="btn btn-primary" disabled={inviting}>
                {inviting
                    ? "Saving..."
                    : $t("admin.team.submit") || "Add Member"}
            </button>
        </div>
    </form>
</Modal>

<!-- Remove Confirmation Dialog -->
<ConfirmDialog
    show={showRemoveConfirm}
    title="Remove Member"
    message="Are you sure you want to remove this member? They will lose access to the workspace immediately."
    confirmText="Remove Member"
    type="danger"
    loading={removing}
    on:confirm={removeMember}
    on:cancel={() => (showRemoveConfirm = false)}
/>

<style>
    .page-content {
        padding: 1.5rem;
        max-width: 1400px;
        margin: 0 auto;
    }

    .stats-row {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2rem;
    }

    .content-card {
        background: var(--bg-surface, #1e293b);
        border-radius: 16px;
        border: 1px solid var(--border-color, rgba(255, 255, 255, 0.05));
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
    }

    .card-header {
        padding: 1.5rem 2rem;
        border-bottom: 1px solid var(--border-color, rgba(255, 255, 255, 0.05));
        display: flex;
        justify-content: space-between;
        align-items: center;
        flex-wrap: wrap;
        gap: 1rem;
    }

    .header-left {
        display: flex;
        align-items: center;
        gap: 2rem;
        flex: 1;
    }

    .card-header h2 {
        font-size: 1.25rem;
        font-weight: 700;
        margin: 0;
        color: var(--text-primary, #fff);
        white-space: nowrap;
    }

    .search-bar {
        position: relative;
        width: 100%;
        max-width: 320px;
    }

    .search-bar :global(.icon) {
        position: absolute;
        left: 0.75rem;
        top: 50%;
        transform: translateY(-50%);
        color: var(--text-secondary, #94a3b8);
        pointer-events: none;
    }

    .search-bar input {
        width: 100%;
        background: var(--bg-primary, #0f172a);
        border: 1px solid rgba(255, 255, 255, 0.1);
        padding: 0.5rem 1rem 0.5rem 2.25rem;
        border-radius: 8px;
        color: var(--text-primary, white);
        font-size: 0.9rem;
    }

    .search-bar input:focus {
        outline: none;
        border-color: var(--color-primary, #6366f1);
    }

    .filter-dropdown select {
        background: var(--bg-primary, #0f172a);
        border: 1px solid rgba(255, 255, 255, 0.1);
        padding: 0.5rem 2rem 0.5rem 1rem;
        border-radius: 8px;
        color: var(--text-primary, white);
        font-size: 0.9rem;
        cursor: pointer;
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
        color: var(--text-secondary, #64748b);
        font-weight: 600;
        border-bottom: 1px solid var(--border-color, rgba(255, 255, 255, 0.05));
        background: rgba(0, 0, 0, 0.1);
    }

    .data-table td {
        padding: 1.25rem 2rem;
        border-bottom: 1px solid var(--border-color, rgba(255, 255, 255, 0.03));
        vertical-align: middle;
        color: var(--text-primary, #e2e8f0);
        font-size: 0.95rem;
    }

    .data-table tr:hover {
        background: rgba(255, 255, 255, 0.02);
    }

    .member-info {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .avatar {
        width: 40px;
        height: 40px;
        background: linear-gradient(
            135deg,
            var(--color-primary, #6366f1),
            var(--color-primary-dark, #4f46e5)
        );
        border-radius: 10px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 700;
        color: white;
        font-size: 0.9rem;
    }

    .member-name {
        font-weight: 600;
        color: var(--text-primary, #fff);
    }

    .you-badge {
        background: rgba(99, 102, 241, 0.2);
        color: var(--color-primary, #818cf8);
        padding: 0.1rem 0.4rem;
        border-radius: 4px;
        font-size: 0.7rem;
        font-weight: 600;
        margin-left: 0.5rem;
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
        color: var(--text-secondary, #64748b);
    }
    .text-right {
        text-align: right;
    }

    .action-btn {
        width: 36px;
        height: 36px;
        border-radius: 8px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border: none;
        background: transparent;
        color: var(--text-secondary, #64748b);
        cursor: pointer;
        transition: all 0.2s;
    }

    .action-btn:hover {
        background: rgba(255, 255, 255, 0.1);
        color: var(--text-primary, #fff);
    }

    .action-btn.danger:hover {
        background: rgba(239, 68, 68, 0.2);
        color: #ef4444;
    }

    .loading-state,
    .error-state {
        padding: 4rem;
        text-align: center;
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

    .empty-state-container {
        text-align: center;
        padding: 4rem 2rem;
        color: var(--text-secondary, #94a3b8);
    }

    .empty-state-container .empty-icon {
        margin-bottom: 1.5rem;
        opacity: 0.5;
    }

    .empty-state-container h3 {
        color: var(--text-primary, #fff);
        margin: 0 0 0.5rem 0;
        font-size: 1.25rem;
    }

    .empty-state-container p {
        margin: 0.5rem 0;
    }

    /* Buttons */
    .btn {
        display: flex;
        align-items: center;
        gap: 0.6rem;
        padding: 0.75rem 1.25rem;
        border-radius: 10px;
        font-weight: 600;
        cursor: pointer;
        border: none;
        font-size: 0.95rem;
        transition: all 0.2s;
    }

    .btn-primary {
        background: var(--color-primary, #6366f1);
        color: white;
        box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
    }

    .btn-primary:hover {
        opacity: 0.9;
        transform: translateY(-1px);
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

    .btn-ghost {
        background: transparent;
        color: var(--text-secondary, #cbd5e1);
    }
    .btn-ghost:hover {
        color: var(--text-primary, #fff);
        background: rgba(255, 255, 255, 0.05);
    }

    /* Forms */
    .form-group {
        margin-bottom: 1.25rem;
    }
    .form-group label {
        display: block;
        margin-bottom: 0.5rem;
        font-size: 0.9rem;
        color: var(--text-secondary, #94a3b8);
    }
    .form-group input,
    .form-group select {
        width: 100%;
        background: var(--bg-primary, #0f172a);
        border: 1px solid rgba(255, 255, 255, 0.1);
        padding: 0.75rem 1rem;
        border-radius: 8px;
        color: var(--text-primary, white);
        font-size: 1rem;
    }
    .form-group input:focus,
    .form-group select:focus {
        outline: none;
        border-color: var(--color-primary, #6366f1);
        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
        margin-top: 2rem;
    }

    .role-select {
        background: transparent;
        border: 1px solid rgba(255, 255, 255, 0.1);
        color: var(--text-primary, #fff);
        padding: 0.3rem 0.5rem;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.85rem;
    }
    .role-select:focus {
        outline: none;
        border-color: var(--color-primary, #6366f1);
    }
</style>
