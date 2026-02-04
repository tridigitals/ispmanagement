<script lang="ts">
  import { isSuperAdmin } from '$lib/stores/auth';
  import { user as currentUser } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import { toast } from '$lib/stores/toast';
  import type { User } from '$lib/api/client';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  import UserFilters from '$lib/components/superadmin/users/UserFilters.svelte';
  import UserTable from '$lib/components/superadmin/users/UserTable.svelte';
  import UserDetailsModal from '$lib/components/superadmin/users/UserDetailsModal.svelte';
  import UserActionModals from '$lib/components/superadmin/users/UserActionModals.svelte';

  let allUsers = $state<User[]>([]);
  let totalUsers = $state(0);
  let loading = $state(true);
  let error = $state('');

  let tenantNameById = $state<Record<string, string>>({});
  let tenantNameBySlug = $state<Record<string, string>>({});

  let searchQuery = $state('');
  let statusFilter = $state<'all' | 'active' | 'inactive'>('all');
  let roleFilter = $state<'all' | 'superadmin' | 'admin' | 'user'>('all');

  let isMobile = $state(false);
  let viewMode = $state<'table' | 'cards'>('table');

  async function loadData() {
    loading = true;
    error = '';

    try {
      const [usersRes, tenantsRes] = await Promise.all([
        api.users.list(1, 200),
        api.superadmin.listTenants().catch(() => null),
      ]);

      allUsers = usersRes.data || [];
      totalUsers = usersRes.total ?? allUsers.length;

      const tenants: any[] = (tenantsRes as any)?.data || [];
      const byId: Record<string, string> = {};
      const bySlug: Record<string, string> = {};
      for (const t of tenants) {
        if (t?.id && t?.name) byId[String(t.id)] = String(t.name);
        if (t?.slug && t?.name) bySlug[String(t.slug)] = String(t.name);
      }
      tenantNameById = byId;
      tenantNameBySlug = bySlug;
    } catch (err: any) {
      console.error('Failed to load users:', err);
      error = err?.message || String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    let cleanup: (() => void) | undefined;

    if (!$isSuperAdmin) {
      goto('/dashboard');
      return cleanup;
    }

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 720px)');
      const sync = () => (isMobile = mq.matches);
      sync();

      try {
        mq.addEventListener('change', sync);
        cleanup = () => mq.removeEventListener('change', sync);
      } catch {
        // Safari/older WebView fallback
        // @ts-ignore
        mq.addListener?.(sync);
        // @ts-ignore
        cleanup = () => mq.removeListener?.(sync);
      }
    }

    void loadData();
    return cleanup;
  });

  function getRoleKey(u: User) {
    if ((u as any).is_super_admin) return 'superadmin';
    const tenantRole = (u as any).tenant_role;
    if (tenantRole) return String(tenantRole).toLowerCase();
    return String((u as any).role || 'user').toLowerCase();
  }

  function getTenantName(u: any) {
    const id = u?.tenant_id ? String(u.tenant_id) : '';
    const slug = u?.tenant_slug ? String(u.tenant_slug) : '';
    return (id && tenantNameById[id]) || (slug && tenantNameBySlug[slug]) || '';
  }

  let stats = $derived({
    total: allUsers.length,
    active: allUsers.filter((u: any) => u.is_active).length,
    inactive: allUsers.filter((u: any) => !u.is_active).length,
    superadmins: allUsers.filter((u: any) => u.is_super_admin).length,
  });

  let filteredUsers = $derived(
    allUsers.filter((u: any) => {
      const q = searchQuery.trim().toLowerCase();
      const matchesSearch =
        !q ||
        String(u.name || '')
          .toLowerCase()
          .includes(q) ||
        String(u.email || '')
          .toLowerCase()
          .includes(q) ||
        String(getTenantName(u) || u.tenant_slug || u.tenant_id || '')
          .toLowerCase()
          .includes(q);

      const matchesStatus =
        statusFilter === 'all' || (statusFilter === 'active' ? u.is_active : !u.is_active);

      const roleKey = getRoleKey(u);
      const matchesRole = roleFilter === 'all' || roleKey === roleFilter;

      return matchesSearch && matchesStatus && matchesRole;
    }),
  );

  $effect(() => {
    if (isMobile) viewMode = 'cards';
  });

  // --- Action Logic ---

  let showResetConfirm = $state(false);
  let confirmLoading = $state(false);
  let userPending2FAReset = $state<User | null>(null);

  function confirmReset2FA(u: User) {
    userPending2FAReset = u;
    showResetConfirm = true;
  }

  async function reset2FA() {
    const u = userPending2FAReset;
    if (!u) return;

    confirmLoading = true;
    try {
      await api.auth.resetUser2FA(u.id);
      // Update local state
      allUsers = allUsers.map((user) =>
        user.id === u.id ? ({ ...user, two_factor_enabled: false } as any) : user,
      );
      toast.success(
        get(t)('superadmin.users.toasts.reset_2fa_success') ||
          'Two-factor authentication has been reset',
      );
      showResetConfirm = false;
    } catch (err: any) {
      toast.error(
        get(t)('superadmin.users.toasts.reset_2fa_failed', {
          values: { message: err?.message || err },
        }) || 'Failed to reset 2FA: ' + (err?.message || err),
      );
    } finally {
      confirmLoading = false;
      userPending2FAReset = null;
    }
  }

  let showStatusConfirm = $state(false);
  let statusConfirmLoading = $state(false);
  let userPendingStatus = $state<User | null>(null);
  let pendingIsActive = $state<boolean>(false);

  let statusConfirmTitle = $derived.by(() =>
    pendingIsActive
      ? $t('superadmin.users.status.activate_title') || 'Activate User'
      : $t('superadmin.users.status.deactivate_title') || 'Deactivate User',
  );

  let statusConfirmMessage = $derived.by(() => {
    const u = userPendingStatus;
    const name = u?.name || $t('superadmin.users.status.this_user') || 'this user';
    if (pendingIsActive) {
      return (
        $t('superadmin.users.status.activate_message', {
          values: { name },
        }) || `Activate ${name}? They will be able to login again.`
      );
    }
    return (
      $t('superadmin.users.status.deactivate_message', {
        values: { name },
      }) || `Deactivate ${name}? They will not be able to login.`
    );
  });

  let statusConfirmKeyword = $derived(pendingIsActive ? 'ACTIVATE' : 'DEACTIVATE');

  let statusConfirmType = $derived<'danger' | 'warning' | 'info'>(
    pendingIsActive ? 'info' : 'danger',
  );

  function confirmToggleActive(u: User) {
    if ((u as any).is_super_admin) {
      toast.error(
        get(t)('superadmin.users.toasts.superadmin_cannot_deactivate') ||
          'Super Admin accounts cannot be deactivated here',
      );
      return;
    }
    if (u.id === $currentUser?.id) {
      toast.error(
        get(t)('superadmin.users.toasts.cannot_deactivate_self') ||
          'You cannot deactivate your own account',
      );
      return;
    }
    userPendingStatus = u;
    pendingIsActive = !Boolean((u as any).is_active);
    showStatusConfirm = true;
  }

  async function toggleActive() {
    const u = userPendingStatus;
    if (!u) return;

    statusConfirmLoading = true;
    try {
      await api.users.update(u.id, { isActive: pendingIsActive });
      allUsers = allUsers.map((x: any) =>
        x.id === u.id ? { ...x, is_active: pendingIsActive } : x,
      );
      toast.success(
        pendingIsActive
          ? get(t)('superadmin.users.toasts.activated') || 'User activated'
          : get(t)('superadmin.users.toasts.deactivated') || 'User deactivated',
      );
      showStatusConfirm = false;
    } catch (e: any) {
      toast.error(
        get(t)('superadmin.users.toasts.update_status_failed', {
          values: { message: e?.message || e },
        }) || 'Failed to update user status: ' + (e?.message || e),
      );
    } finally {
      statusConfirmLoading = false;
      userPendingStatus = null;
    }
  }

  let showDetailsModal = $state(false);
  let detailsUser = $state<User | null>(null);

  function openDetails(u: User) {
    detailsUser = u;
    showDetailsModal = true;
  }

  const getInitials = (name: string) => name.substring(0, 2).toUpperCase();
</script>

<div class="superadmin-content fade-in">
  <div class="stats-row" aria-label={$t('superadmin.users.aria.stats') || 'User stats'}>
    <button
      class="stat-btn"
      class:active={statusFilter === 'all'}
      onclick={() => {
        statusFilter = 'all';
        roleFilter = 'all';
      }}
      aria-label={$t('superadmin.users.stats.show_all') || 'Show all users'}
      title={$t('superadmin.users.stats.show_all') || 'Show all users'}
      type="button"
    >
      <StatsCard
        title={$t('superadmin.users.stats.all_title') || 'All Users'}
        value={stats.total}
        icon="users"
        color="primary"
      />
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'active'}
      onclick={() => (statusFilter = 'active')}
      aria-label={$t('superadmin.users.stats.show_active') || 'Show active users'}
      title={$t('superadmin.users.stats.show_active') || 'Show active users'}
      type="button"
    >
      <StatsCard
        title={$t('superadmin.users.stats.active_title') || 'Active Users'}
        value={stats.active}
        icon="check-circle"
        color="success"
      />
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'inactive'}
      onclick={() => (statusFilter = 'inactive')}
      aria-label={$t('superadmin.users.stats.show_inactive') || 'Show inactive users'}
      title={$t('superadmin.users.stats.show_inactive') || 'Show inactive users'}
      type="button"
    >
      <StatsCard
        title={$t('superadmin.users.stats.inactive_title') || 'Inactive Users'}
        value={stats.inactive}
        icon="slash"
        color="warning"
      />
    </button>
    <button
      class="stat-btn"
      class:active={roleFilter === 'superadmin'}
      onclick={() => {
        roleFilter = 'superadmin';
        statusFilter = 'all';
      }}
      aria-label={$t('superadmin.users.stats.show_superadmins') || 'Show super admins'}
      title={$t('superadmin.users.stats.show_superadmins') || 'Show super admins'}
      type="button"
    >
      <StatsCard
        title={$t('superadmin.users.stats.superadmins_title') || 'Super Admins'}
        value={stats.superadmins}
        icon="server"
        color="danger"
      />
    </button>
  </div>

  <div class="glass-card" in:fly={{ y: 20, delay: 80 }}>
    <div class="card-header glass">
      <div>
        <h3>{$t('superadmin.users.title') || 'Users'}</h3>
        <span class="muted">
          {$t('superadmin.users.subtitle') || 'Manage global users and access'}
        </span>
      </div>
      <span class="count-badge">
        {totalUsers || stats.total}
        {$t('superadmin.users.count') || 'users'}
      </span>
    </div>

    <div class="toolbar-wrapper">
      <UserFilters bind:searchQuery bind:roleFilter bind:statusFilter bind:viewMode {isMobile} />
    </div>

    {#if error}
      <div class="error-state">
        <p>{error}</p>
      </div>
    {:else}
      <UserTable
        users={filteredUsers}
        {loading}
        {isMobile}
        {viewMode}
        currentUser={$currentUser}
        onOpenDetails={openDetails}
        onReset2FA={confirmReset2FA}
        onToggleActive={confirmToggleActive}
        {getTenantName}
        {getInitials}
      />
    {/if}
  </div>
</div>

<UserActionModals
  bind:showResetConfirm
  {confirmLoading}
  onReset2FA={reset2FA}
  bind:showStatusConfirm
  {statusConfirmTitle}
  {statusConfirmMessage}
  {statusConfirmKeyword}
  {statusConfirmType}
  {statusConfirmLoading}
  onToggleActive={toggleActive}
  {pendingIsActive}
/>

<UserDetailsModal bind:show={showDetailsModal} user={detailsUser} {getTenantName} />

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
    grid-template-columns: repeat(4, minmax(0, 1fr));
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

  :global([data-theme='light']) .glass-card {
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

  :global([data-theme='light']) .card-header {
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

  :global([data-theme='light']) .count-badge {
    background: rgba(0, 0, 0, 0.03);
    border-color: rgba(0, 0, 0, 0.06);
  }

  .toolbar-wrapper {
    padding: 1rem 1.25rem 0.25rem 1.25rem;
  }

  .error-state {
    padding: 2rem 1.25rem;
    text-align: center;
    color: var(--text-secondary);
  }

  .error-state p {
    margin: 0.75rem 0 0 0;
    color: var(--text-secondary);
  }
</style>
