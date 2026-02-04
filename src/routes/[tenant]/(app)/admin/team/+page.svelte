<script lang="ts">
  import { isAdmin, user, can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { toast } from 'svelte-sonner';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { appSettings } from '$lib/stores/settings';
  import { formatDate } from '$lib/utils/date';
  import type { TeamMember, Role } from '$lib/api/client';

  const columns = $derived.by(() => [
    { key: 'member', label: $t('admin.team.columns.member') || 'Member' },
    { key: 'role', label: $t('admin.team.columns.role') || 'Role' },
    { key: 'status', label: $t('admin.team.columns.status') || 'Status' },
    {
      key: 'created_at',
      label: $t('admin.team.columns.created_at') || 'Date Added',
    },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  let teamMembers = $state<TeamMember[]>([]);
  let roles = $state<Role[]>([]);
  let loading = $state(true);
  let error = $state('');

  let searchQuery = $state('');
  let roleFilter = $state('all');

  let showInviteModal = $state(false);
  let inviteEmail = $state('');
  let inviteName = $state('');
  let inviteRoleId = $state('');
  let invitePassword = $state('');
  let inviting = $state(false);

  let showEditModal = $state(false);
  let editingMember = $state<TeamMember | null>(null);
  let editRoleId = $state('');
  let savingRole = $state(false);

  let showDeleteModal = $state(false);
  let memberToDelete = $state<TeamMember | null>(null);
  let isDeleting = $state(false);

  let statusFilter = $state<'all' | 'active' | 'inactive'>('all');

  let filteredMembers = $derived(
    teamMembers.filter((m) => {
      const matchesSearch =
        m.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        m.email.toLowerCase().includes(searchQuery.toLowerCase());
      const matchesRole = roleFilter === 'all' || m.role_id === roleFilter;
      const matchesStatus =
        statusFilter === 'all' || (statusFilter === 'active' ? m.is_active : !m.is_active);
      return matchesSearch && matchesRole && matchesStatus;
    }),
  );

  let stats = $derived({
    total: teamMembers.length,
    active: teamMembers.filter((m) => m.is_active).length,
    inactive: teamMembers.filter((m) => !m.is_active).length,
  });

  let myMember = $derived(teamMembers.find((m) => m.email === $user?.email));
  let myRoleLevel = $derived(
    myMember && roles.length > 0 ? roles.find((r) => r.id === myMember?.role_id)?.level || 0 : 0,
  );

  let roleOptions = $derived([
    {
      label: $t('admin.team.filters.all_roles') || 'All Roles',
      value: 'all',
    },
    ...roles.map((r) => ({ label: r.name, value: r.id })),
  ]);

  onMount(async () => {
    if (!$can('read', 'team')) {
      goto('/unauthorized');
      return;
    }
    await loadData();
  });

  async function loadData() {
    loading = true;
    try {
      const [membersRes, rolesRes] = await Promise.all([api.team.list(), api.roles.list()]);
      teamMembers = membersRes;
      roles = rolesRes;

      if (roles.length > 0 && !inviteRoleId) {
        const memberRole = roles.find((r) => r.name === 'Member');
        inviteRoleId = memberRole ? memberRole.id : roles[0].id;
      }
    } catch (e: any) {
      error = e.toString();
      toast.error(get(t)('admin.team.toasts.load_failed') || 'Failed to load team data');
    } finally {
      loading = false;
    }
  }

  async function inviteMember() {
    if (!inviteEmail || !inviteName || !inviteRoleId) return;
    inviting = true;
    try {
      await api.team.add(inviteEmail, inviteName, inviteRoleId, invitePassword);
      await loadData();
      showInviteModal = false;
      inviteEmail = '';
      inviteName = '';
      invitePassword = '';
      toast.success(get(t)('admin.team.toasts.added') || 'Team member added successfully');
    } catch (e: any) {
      toast.error(
        get(t)('admin.team.toasts.add_failed', {
          values: { message: e?.message || e },
        }) || 'Failed to add member: ' + e.message,
      );
    } finally {
      inviting = false;
    }
  }

  function confirmRemove(member: TeamMember) {
    memberToDelete = member;
    showDeleteModal = true;
  }

  async function handleConfirmDelete() {
    const target = memberToDelete;
    if (!target) return;
    isDeleting = true;
    try {
      await api.team.remove(target.id);
      teamMembers = teamMembers.filter((m) => m.id !== target.id);
      toast.success(get(t)('admin.team.toasts.removed') || 'Member removed successfully');
      showDeleteModal = false;
      memberToDelete = null;
    } catch (e: any) {
      toast.error(
        get(t)('admin.team.toasts.remove_failed', {
          values: { message: e?.message || e },
        }) || 'Failed to remove member: ' + e.message,
      );
    } finally {
      isDeleting = false;
    }
  }

  function openEditModal(member: TeamMember) {
    editingMember = member;
    editRoleId = member.role_id || '';
    showEditModal = true;
  }

  async function saveMemberRole() {
    const member = editingMember;
    if (!member || !editRoleId) return;
    savingRole = true;
    try {
      await api.team.updateRole(member.id, editRoleId);
      const index = teamMembers.findIndex((m) => m.id === member.id);
      if (index !== -1) {
        const role = roles.find((r) => r.id === editRoleId);
        teamMembers[index].role_id = editRoleId;
        teamMembers[index].role_name = role?.name || '';
        teamMembers = [...teamMembers];
      }
      toast.success(get(t)('admin.team.toasts.role_updated') || 'Member role updated successfully');
      showEditModal = false;
      editingMember = null;
    } catch (e: any) {
      toast.error(
        get(t)('admin.team.toasts.role_update_failed', {
          values: { message: e?.message || e },
        }) || 'Failed to update role: ' + e.message,
      );
    } finally {
      savingRole = false;
    }
  }

  const getInitials = (name: string) => name.substring(0, 2).toUpperCase();
</script>

<div class="page-content fade-in">
  <div class="stats-row" in:fly={{ y: 20, duration: 300 }}>
    <button
      class="stat-btn"
      class:active={statusFilter === 'all'}
      onclick={() => (statusFilter = 'all')}
      aria-label={$t('admin.team.stats.show_all') || 'Show all members'}
      title={$t('admin.team.stats.show_all') || 'Show all members'}
      type="button"
    >
      <StatsCard
        title={$t('admin.team.stats.total_title') || 'Total Members'}
        value={stats.total}
        icon="users"
        color="primary"
      />
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'active'}
      onclick={() => (statusFilter = 'active')}
      aria-label={$t('admin.team.stats.show_active') || 'Show active members'}
      title={$t('admin.team.stats.show_active') || 'Show active members'}
      type="button"
    >
      <StatsCard
        title={$t('admin.team.stats.active_title') || 'Active Members'}
        value={stats.active}
        icon="check-circle"
        color="success"
      />
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'inactive'}
      onclick={() => (statusFilter = 'inactive')}
      aria-label={$t('admin.team.stats.show_inactive') || 'Show inactive members'}
      title={$t('admin.team.stats.show_inactive') || 'Show inactive members'}
      type="button"
    >
      <StatsCard
        title={$t('admin.team.stats.inactive_title') || 'Inactive Members'}
        value={stats.inactive}
        icon="slash"
        color="warning"
      />
    </button>
  </div>

  <div class="glass-card" in:fly={{ y: 20, delay: 80 }}>
    <div class="card-header glass">
      <div>
        <h3>{$t('admin.team.title') || 'Team Members'}</h3>
        <span class="muted"
          >{$t('admin.team.subtitle') || 'Manage your organization members and access'}</span
        >
      </div>
      <span class="count-badge">
        {$t('admin.team.count', { values: { count: stats.total } }) || `${stats.total} members`}
      </span>
    </div>

    <div class="toolbar-wrapper">
      <TableToolbar bind:searchQuery placeholder={$t('admin.team.search') || 'Search members...'}>
        {#snippet filters()}
          <div class="filter-dropdown">
            <Select bind:value={roleFilter} options={roleOptions} width="100%" />
          </div>
          <div class="status-filter">
            <button
              type="button"
              class="filter-chip"
              class:active={statusFilter === 'all'}
              onclick={() => (statusFilter = 'all')}
            >
              {$t('admin.team.filters.all') || $t('common.all') || 'All'}
            </button>
            <button
              type="button"
              class="filter-chip"
              class:active={statusFilter === 'active'}
              onclick={() => (statusFilter = 'active')}
            >
              {$t('admin.team.filters.active') || $t('common.active') || 'Active'}
            </button>
            <button
              type="button"
              class="filter-chip"
              class:active={statusFilter === 'inactive'}
              onclick={() => (statusFilter = 'inactive')}
            >
              {$t('admin.team.filters.inactive') || $t('common.inactive') || 'Inactive'}
            </button>
          </div>
        {/snippet}
        {#snippet actions()}
          {#if $can('create', 'team')}
            <button class="btn btn-primary" onclick={() => (showInviteModal = true)}>
              <Icon name="plus" size={18} />
              {$t('admin.team.invite_button') || 'Add Member'}
            </button>
          {/if}
        {/snippet}
      </TableToolbar>
    </div>

    {#if error}
      <div class="error-state">
        <Icon name="alert-circle" size={48} color="#ef4444" />
        <p>{error}</p>
        <button class="btn btn-glass" onclick={loadData}>
          {$t('common.retry') || 'Retry'}
        </button>
      </div>
    {:else}
      <div class="table-wrapper">
        <Table
          pagination={true}
          {columns}
          data={filteredMembers}
          {loading}
          emptyText={$t('admin.team.empty.no_results') || 'No members found'}
        >
          {#snippet empty()}
            <div class="empty-state-container">
              <div class="empty-icon">
                <Icon name="users" size={64} />
              </div>
              <h3>
                {$t('admin.team.empty.no_results') || 'No members found'}
              </h3>
              <p>
                {$t('admin.team.empty.try_adjusting') || 'Try adjusting your search or filters.'}
              </p>
            </div>
          {/snippet}

          {#snippet cell({ item, key })}
            {#if key === 'member'}
              <div class="member-info">
                <div class="avatar">
                  {getInitials(item.name)}
                </div>
                <div>
                  <div class="member-name">
                    {item.name}
                    {#if item.email === $user?.email}
                      <span class="you-badge">{$t('common.you') || 'YOU'}</span>
                    {/if}
                  </div>
                  <div class="text-muted" style="font-size: 0.85rem">
                    {item.email}
                  </div>
                </div>
              </div>
            {:else if key === 'role'}
              <span class="role-pill {item.role_name?.toLowerCase() || 'member'}">
                {item.role_name || $t('admin.team.roles.member') || 'Member'}
              </span>
            {:else if key === 'status'}
              <span class="status-pill {item.is_active ? 'active' : 'inactive'}">
                <span class="dot"></span>
                {item.is_active
                  ? $t('common.active') || 'Active'
                  : $t('common.inactive') || 'Inactive'}
              </span>
            {:else if key === 'created_at'}
              {formatDate(item.created_at, { timeZone: $appSettings.app_timezone })}
            {:else if key === 'actions'}
              <div class="action-buttons-cell">
                {#if $can('update', 'team') && myRoleLevel > (roles.find((r) => r.id === item.role_id)?.level || 0)}
                  <button
                    class="btn-icon primary"
                    title={$t('admin.team.actions.edit_role') || 'Edit Role'}
                    onclick={() => openEditModal(item)}
                  >
                    <Icon name="edit" size={18} />
                  </button>
                {/if}
                {#if item.email !== $user?.email && $can('delete', 'team') && myRoleLevel > (roles.find((r) => r.id === item.role_id)?.level || 0)}
                  <button
                    class="btn-icon danger"
                    title={$t('admin.team.actions.remove_member') || 'Remove Member'}
                    onclick={() => confirmRemove(item)}
                  >
                    <Icon name="trash" size={18} />
                  </button>
                {/if}
              </div>
            {/if}
          {/snippet}
        </Table>
      </div>
    {/if}
  </div>
</div>

<ConfirmDialog
  bind:show={showDeleteModal}
  title={$t('admin.team.remove.title') || 'Remove Team Member'}
  message={$t('admin.team.remove.message', {
    values: { name: memberToDelete?.name || '' },
  }) ||
    `Are you sure you want to remove ${memberToDelete?.name} from the team? They will lose access immediately.`}
  confirmText={$t('admin.team.remove.confirm') || 'Remove Member'}
  type="danger"
  loading={isDeleting}
  onconfirm={handleConfirmDelete}
/>

<Modal
  show={showInviteModal}
  title={$t('admin.team.add_member_modal_title') || 'Add Team Member'}
  onclose={() => (showInviteModal = false)}
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      inviteMember();
    }}
  >
    <div class="form-group">
      <label>
        {$t('admin.team.name_label') || 'Name'}
        <input
          type="text"
          bind:value={inviteName}
          placeholder={$t('admin.team.placeholders.name') || 'John Doe'}
          required
        />
      </label>
    </div>
    <div class="form-group">
      <label>
        {$t('admin.team.email_label') || 'Email Address'}
        <input
          type="email"
          bind:value={inviteEmail}
          placeholder={$t('admin.team.placeholders.email') || 'colleague@company.com'}
          required
        />
      </label>
    </div>
    <div class="form-group">
      <label>
        {$t('admin.team.password_label') || 'Password (Optional)'}
        <input
          type="text"
          bind:value={invitePassword}
          placeholder={$t('admin.team.placeholders.password_auto') || 'Auto-generated if empty'}
        />
      </label>
    </div>
    <div class="form-group">
      <label>
        {$t('admin.team.role_label') || 'Role'}
        <select bind:value={inviteRoleId} required>
          {#each roles as role}
            <option value={role.id}>{role.name}</option>
          {/each}
        </select>
      </label>
    </div>
    <div class="modal-actions">
      <button type="button" class="btn btn-ghost" onclick={() => (showInviteModal = false)}>
        {$t('admin.team.cancel') || 'Cancel'}
      </button>
      <button type="submit" class="btn btn-primary" disabled={inviting}>
        {inviting ? $t('common.saving') || 'Saving...' : $t('admin.team.submit') || 'Add Member'}
      </button>
    </div>
  </form>
</Modal>

<Modal
  show={showEditModal}
  title={$t('admin.team.edit_role.title') || 'Edit Member Role'}
  onclose={() => (showEditModal = false)}
>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      saveMemberRole();
    }}
  >
    <div class="form-group">
      <label>
        {$t('admin.team.edit_role.member_name') || 'Member Name'}
        <input type="text" value={editingMember?.name} disabled class="bg-disabled" />
      </label>
    </div>
    <div class="form-group">
      <label>
        {$t('admin.team.edit_role.role_label') || 'Role'}
        <select bind:value={editRoleId} required>
          {#each roles as role}
            <option value={role.id}>{role.name}</option>
          {/each}
        </select>
      </label>
    </div>
    <div class="modal-actions">
      <button type="button" class="btn btn-ghost" onclick={() => (showEditModal = false)}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button type="submit" class="btn btn-primary" disabled={savingRole}>
        {savingRole
          ? $t('common.saving') || 'Saving...'
          : $t('common.save_changes') || 'Save Changes'}
      </button>
    </div>
  </form>
</Modal>

<style>
  .page-content {
    padding: clamp(1rem, 3vw, 1.5rem);
    max-width: 1400px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    --glass: rgba(255, 255, 255, 0.04);
    --glass-border: rgba(255, 255, 255, 0.08);
    --accent-emerald: #10b981;
    --accent-amber: #f59e0b;
    --accent-cyan: #22d3ee;
    --accent-indigo: #6366f1;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }

  .stat-btn {
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    border-radius: 18px;
    transition: transform 0.2s ease;
  }

  .stat-btn:active {
    transform: translateY(1px);
  }

  .stat-btn.active :global(.stats-card) {
    border-color: rgba(99, 102, 241, 0.45);
    box-shadow:
      0 14px 36px rgba(0, 0, 0, 0.3),
      0 0 0 1px rgba(99, 102, 241, 0.25) inset;
  }

  :global([data-theme='light']) .stat-btn.active :global(.stats-card) {
    border-color: rgba(99, 102, 241, 0.3);
    box-shadow:
      0 14px 36px rgba(0, 0, 0, 0.08),
      0 0 0 1px rgba(99, 102, 241, 0.18) inset;
  }

  .glass-card {
    background: linear-gradient(145deg, var(--bg-surface, #0f172a), #0b0c10);
    border-radius: 16px;
    border: 1px solid var(--glass-border);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
    overflow: hidden;
  }

  .card-header {
    padding: 1.25rem 1.75rem;
    border-bottom: 1px solid var(--glass-border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .card-header h3 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 800;
    color: var(--text-primary);
  }

  .card-header .muted {
    display: block;
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin-top: 0.25rem;
  }

  .count-badge {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    padding: 0.35rem 0.75rem;
    border-radius: 12px;
    font-size: 0.8rem;
    font-weight: 800;
    border: 1px solid var(--glass-border);
  }

  .toolbar-wrapper {
    padding: 1rem 1.75rem 0.5rem;
    border-bottom: 1px solid var(--glass-border);
  }

  .status-filter {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.25rem;
    border-radius: 12px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.03);
    flex-wrap: wrap;
  }

  :global([data-theme='light']) .status-filter {
    background: rgba(0, 0, 0, 0.02);
  }

  .filter-chip {
    height: 32px;
    padding: 0 0.75rem;
    border-radius: 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  @media (max-width: 768px) {
    .toolbar-wrapper {
      padding: 0.9rem 1rem 0.75rem;
    }
    .table-wrapper {
      padding: 1rem;
    }
    .filter-dropdown {
      width: 100%;
    }
  }

  .filter-chip:hover {
    background: rgba(99, 102, 241, 0.08);
    color: var(--text-primary);
  }

  .filter-chip.active {
    background: rgba(99, 102, 241, 0.14);
    border-color: rgba(99, 102, 241, 0.28);
    color: var(--text-primary);
  }

  .table-wrapper {
    padding: 1.5rem 1.75rem 1.75rem;
  }

  .member-info {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .avatar {
    width: 42px;
    height: 42px;
    background:
      radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.35), transparent 60%),
      linear-gradient(135deg, var(--color-primary, #6366f1), var(--color-primary-dark, #4f46e5));
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    color: white;
    font-size: 0.9rem;
    border: 1px solid var(--glass-border);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.25);
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
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.04);
  }

  .role-pill.admin {
    background: radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.14), transparent 60%);
    color: #818cf8;
  }

  .role-pill.user {
    background: radial-gradient(circle at 20% 20%, rgba(16, 185, 129, 0.14), transparent 60%);
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
    background: rgba(16, 185, 129, 0.14);
    color: #22c55e;
    border: 1px solid rgba(16, 185, 129, 0.25);
  }

  .status-pill.inactive {
    background: rgba(239, 68, 68, 0.12);
    color: #ef4444;
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

  /* Mobile Responsiveness */
  @media (max-width: 768px) {
    .page-content {
      padding: 1rem;
    }

    .stats-row {
      grid-template-columns: repeat(2, 1fr);
      gap: 0.75rem;
    }

    .card-header {
      padding: 1rem;
    }
  }

  .action-buttons-cell {
    display: inline-flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .btn-icon {
    width: 36px;
    height: 36px;
    padding: 0;
    border-radius: 10px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary, #cbd5e1);
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-icon:hover {
    background: rgba(99, 102, 241, 0.12);
    color: var(--text-primary, #fff);
    border-color: rgba(99, 102, 241, 0.4);
  }

  .btn-icon.danger:hover {
    background: rgba(239, 68, 68, 0.14);
    color: #ef4444;
    border-color: rgba(239, 68, 68, 0.3);
  }

  @media (max-width: 900px) {
    .btn-icon {
      width: 34px;
      height: 34px;
    }
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
    background: var(--glass);
    border: 1px solid var(--glass-border);
    border-radius: 12px;
  }

  .empty-state-container .empty-icon {
    margin-bottom: 1.5rem;
    opacity: 0.7;
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

  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid transparent;
  }
  .btn-ghost:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.06);
    border-color: var(--glass-border);
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

  .bg-disabled {
    background: rgba(255, 255, 255, 0.05) !important;
    opacity: 0.7;
    cursor: not-allowed;
  }

  /* Light theme adjustments */
  :global([data-theme='light']) .glass-card {
    background: linear-gradient(135deg, #ffffff, #f7f7fb);
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.08),
      0 0 0 1px rgba(255, 255, 255, 0.8);
  }

  :global([data-theme='light']) .btn-icon {
    background: rgba(0, 0, 0, 0.02);
    border-color: rgba(0, 0, 0, 0.08);
    color: #475569;
  }
  :global([data-theme='light']) .btn-icon:hover {
    background: rgba(99, 102, 241, 0.12);
    color: #111827;
    border-color: rgba(99, 102, 241, 0.3);
  }

  :global([data-theme='light']) .role-pill {
    border-color: rgba(0, 0, 0, 0.08);
    background: rgba(0, 0, 0, 0.03);
  }

  :global([data-theme='light']) .status-pill.active {
    background: rgba(16, 185, 129, 0.12);
    color: #15803d;
    border-color: rgba(16, 185, 129, 0.25);
  }
  :global([data-theme='light']) .status-pill.inactive {
    background: rgba(239, 68, 68, 0.12);
    color: #b91c1c;
    border-color: rgba(239, 68, 68, 0.25);
  }

  :global([data-theme='light']) .empty-state-container {
    background: #ffffff;
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.06);
  }

  :global([data-theme='light']) .form-group input,
  :global([data-theme='light']) .form-group select {
    background: #ffffff;
    border-color: rgba(0, 0, 0, 0.08);
    color: #0f172a;
  }
</style>
