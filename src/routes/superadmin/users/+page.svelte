<script lang="ts">
  import { isSuperAdmin } from '$lib/stores/auth';
  import { user as currentUser } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import type { User } from '$lib/api/client';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  import UserFilters from '$lib/components/superadmin/users/UserFilters.svelte';
  import UserTable from '$lib/components/superadmin/users/UserTable.svelte';
  import { loadSuperadminUsersModalModules } from './usersPageModules';

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

  let UserDetailsModalComponent = $state<any>(null);
  let UserActionModalsComponent = $state<any>(null);
  let modalModulesLoading = $state(false);

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

  async function ensureModalModulesLoaded() {
    if ((UserDetailsModalComponent && UserActionModalsComponent) || modalModulesLoading) return;

    modalModulesLoading = true;
    try {
      const { UserDetailsModalComponent: UserDetailsModal, UserActionModalsComponent: UserActionModals } =
        await loadSuperadminUsersModalModules();
      UserDetailsModalComponent = UserDetailsModal;
      UserActionModalsComponent = UserActionModals;
    } finally {
      modalModulesLoading = false;
    }
  }

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

  async function confirmReset2FA(u: User) {
    userPending2FAReset = u;
    showResetConfirm = true;
    await ensureModalModulesLoaded();
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

  // Permanent delete state. We surface a separate ConfirmDialog from
  // UserActionModals that requires typing the literal "DELETE" before
  // accepting, in addition to the in-button `disabled` guards for
  // super-admin rows and the actor's own row.
  let showDeleteConfirm = $state(false);
  let deleteConfirmLoading = $state(false);
  let userPendingDelete = $state<User | null>(null);

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

  async function confirmToggleActive(u: User) {
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
    await ensureModalModulesLoaded();
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

  // Open the typed-keyword delete confirm dialog for a target user.
  async function confirmDeleteUser(u: User) {
    if ((u as any).is_super_admin) {
      toast.error(
        get(t)('superadmin.users.toasts.superadmin_cannot_delete') ||
          'Super Admin accounts cannot be deleted here.',
      );
      return;
    }
    if (u.id === $currentUser?.id) {
      toast.error(
        get(t)('superadmin.users.toasts.cannot_delete_self') ||
          'You cannot delete your own account from this view.',
      );
      return;
    }
    userPendingDelete = u;
    showDeleteConfirm = true;
    await ensureModalModulesLoaded();
  }

  // Effective delete handler (called by UserActionModals after the user
  // has typed the literal "DELETE"). We call api.users.delete which goes
  // through the Tauri command (and on web FE it routes through the same
  // HTTP DELETE /api/users/{id} endpoint via safeInvoke).
  async function deleteUser() {
    const u = userPendingDelete;
    if (!u) return;

    deleteConfirmLoading = true;
    try {
      await api.users.delete(u.id);
      // Remove from local cache so it disappears without full reload.
      allUsers = allUsers.filter((x: any) => x.id !== u.id);
      totalUsers = Math.max(0, (totalUsers ?? 0) - 1);
      toast.success(
        get(t)('superadmin.users.toasts.deleted') ||
          `User ${u.email} deleted permanently. Their active sessions have been invalidated.`,
      );
      showDeleteConfirm = false;
    } catch (e: any) {
      toast.error(
        extractApiErrorMessage(
          e,
          get(t)('superadmin.users.toasts.delete_failed') ||
            'Failed to delete user. The user may be the last active super-admin.',
        ),
      );
    } finally {
      deleteConfirmLoading = false;
      userPendingDelete = null;
    }
  }

  let showDetailsModal = $state(false);
  let detailsUser = $state<User | null>(null);

  async function openDetails(u: User) {
    detailsUser = u;
    showDetailsModal = true;
    await ensureModalModulesLoaded();
  }

  const getInitials = (name: string) => name.substring(0, 2).toUpperCase();
</script>

<div class="sa-users fade-in">
  <!-- ── Page header ── -->
  <div class="page-head">
    <div>
      <div class="crumbs">
        {$t('superadmin.users.crumbs.root')}
        <span class="crumb-sep">›</span>
        <b>{$t('superadmin.users.crumbs.users')}</b>
      </div>
      <h1>{$t('superadmin.users.title')}</h1>
      <p class="subtitle">{$t('superadmin.users.subtitle')}</p>
    </div>
    <div class="head-actions">
      <button class="btn ghost" onclick={() => loadData()}><Icon name="refresh-cw" size={14} /></button>
    </div>
  </div>

  <!-- ── Stat chips (filter + display) ── -->
  <div class="stats-row" aria-label={$t('superadmin.users.aria.stats')}>
    <button class="stat-chip" class:on={statusFilter === 'all' && roleFilter === 'all'} onclick={() => { statusFilter = 'all'; roleFilter = 'all'; }}>
      <span class="chip-val">{stats.total}</span>
      <span class="chip-lbl">{$t('superadmin.users.stats.all_title')}</span>
    </button>
    <button class="stat-chip" class:on={statusFilter === 'active'} onclick={() => (statusFilter = 'active')}>
      <span class="chip-val">{stats.active}</span>
      <span class="chip-lbl">{$t('superadmin.users.stats.active_title')}</span>
    </button>
    <button class="stat-chip" class:on={statusFilter === 'inactive'} onclick={() => (statusFilter = 'inactive')}>
      <span class="chip-val">{stats.inactive}</span>
      <span class="chip-lbl">{$t('superadmin.users.stats.inactive_title')}</span>
    </button>
    <button class="stat-chip" class:on={roleFilter === 'superadmin'} onclick={() => { roleFilter = 'superadmin'; statusFilter = 'all'; }}>
      <span class="chip-val">{stats.superadmins}</span>
      <span class="chip-lbl">{$t('superadmin.users.stats.superadmins_title')}</span>
    </button>
  </div>

  <!-- ── Table panel ── -->
  <div class="panel">
    <div class="panel-head">
      <div class="panel-tools">
        <button class="icon-btn small" class:active={viewMode === 'table'} onclick={() => (viewMode = 'table')} title={$t('common.table') || 'Table'}>
          <Icon name="list" size={15} />
        </button>
        <button class="icon-btn small" class:active={viewMode === 'cards'} onclick={() => (viewMode = 'cards')} title={$t('common.cards') || 'Cards'}>
          <Icon name="layout-grid" size={15} />
        </button>
      </div>
    </div>
    <UserFilters bind:searchQuery bind:roleFilter bind:statusFilter bind:viewMode {isMobile} />

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
        onDelete={confirmDeleteUser}
        {getTenantName}
        {getInitials}
      />
    {/if}
  </div>
</div>

{#if UserActionModalsComponent}
  <UserActionModalsComponent
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
    bind:showDeleteConfirm
    {deleteConfirmLoading}
    deleteConfirmMessageValues={{ name: userPendingDelete?.name || 'this user', email: userPendingDelete?.email || '—' }}
    onDelete={deleteUser}
  />
{/if}

{#if UserDetailsModalComponent}
  <UserDetailsModalComponent bind:show={showDetailsModal} user={detailsUser} {getTenantName} />
{/if}

<style>
  .sa-users {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1280px;
    margin: 0 auto;
    color: var(--text-primary);
  }

  /* ── Page header ── */
  .page-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 22px;
    flex-wrap: wrap;
  }

  .crumbs {
    font-size: 0.75rem;
    color: var(--text-secondary);
    opacity: 0.75;
    margin-bottom: 6px;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .crumbs b { font-weight: 500; opacity: 1; }

  .page-head h1 {
    font-size: 1.45rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.875rem;
    margin: 2px 0 0;
  }

  .head-actions { display: flex; gap: 10px; align-items: center; }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    transition: all 0.15s;
  }

  .btn:hover { border-color: var(--color-primary); }
  .btn.ghost { background: transparent; }

  /* ── Stat chips ── */
  .stats-row {
    display: flex;
    gap: 12px;
    margin-bottom: 20px;
    flex-wrap: wrap;
  }

  .stat-chip {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 20px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    cursor: pointer;
    transition: all 0.15s;
    color: var(--text-primary);
    text-align: left;
    min-width: 100px;
  }

  .stat-chip.on {
    border-color: var(--color-primary);
    background: rgba(139, 156, 255, 0.06);
  }

  .stat-chip:hover {
    border-color: var(--color-primary);
    transform: translateY(-1px);
  }

  .chip-val {
    font-size: 1.4rem;
    font-weight: 800;
    letter-spacing: -0.02em;
  }

  .chip-lbl {
    font-size: 0.75rem;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  /* ── Panel ── */
  .panel {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
    margin-bottom: 24px;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    padding: 14px 20px;
    border-bottom: 1px solid var(--border-color);
    gap: 12px;
  }

  .panel-tools { display: flex; gap: 6px; }

  .icon-btn.small {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    background: var(--bg-raised);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }

  .icon-btn.small:hover { color: var(--color-primary); border-color: var(--color-primary); }

  .icon-btn.small.active {
    background: rgba(139, 156, 255, 0.1);
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .error-state {
    padding: 2rem 1.25rem;
    text-align: center;
    color: var(--text-secondary);
  }

  @media (max-width: 768px) {
    .page-head { align-items: flex-start; flex-direction: column; }
    .stat-chip { min-width: 80px; padding: 10px 14px; }
  }
</style>
