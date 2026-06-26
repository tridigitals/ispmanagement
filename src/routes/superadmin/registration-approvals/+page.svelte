<script lang="ts">
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { isSuperAdmin } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { superadmin, type PendingUser } from '$lib/api/superadmin';
  import { roles as rolesApi } from '$lib/api/roles';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select from '$lib/components/ui/Select.svelte';

  let pendingUsers: PendingUser[] = $state([]);
  let loading = $state(true);
  let error = $state('');

  let tenantOptions = $state<Array<{ label: string; value: string }>>([]);
  let roleOptions = $state<Array<{ label: string; value: string }>>([]);

  let isMobile = $state(false);

  // Approve dialog state
  let showApproveDialog = $state(false);
  let approveTarget: PendingUser | null = $state(null);
  let approveTenantId = $state('');
  let approveRoleId = $state('');
  let approving = $state(false);

  // Reject dialog state
  let showRejectDialog = $state(false);
  let rejectTarget: PendingUser | null = $state(null);
  let rejectReason = $state('');
  let rejecting = $state(false);

  async function loadPendingUsers() {
    loading = true;
    error = '';
    try {
      const res = await superadmin.listPendingApprovals();
      pendingUsers = res.users || [];
    } catch (err: any) {
      console.error('Failed to load pending users:', err);
      error = err?.message || String(err);
    } finally {
      loading = false;
    }
  }

  async function loadTenantOptions() {
    try {
      const res = await superadmin.listTenants();
      const tenants = (res?.data || []) as any[];
      tenantOptions = tenants
        .filter((tenant) => tenant?.id)
        .map((tenant) => ({
          label: tenant.name || tenant.slug || String(tenant.id),
          value: String(tenant.id),
        }));
    } catch (err) {
      console.error('Failed to load tenants:', err);
      tenantOptions = [];
    }
  }

  async function loadRoleOptions() {
    try {
      const list = (await rolesApi.list()) as any[];
      roleOptions = (list || [])
        .filter((role) => role?.id)
        .map((role) => ({
          label: role.name || String(role.id),
          value: String(role.id),
        }));
    } catch (err) {
      console.error('Failed to load roles:', err);
      roleOptions = [];
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
        // @ts-ignore Safari fallback
        mq.addListener?.(sync);
        // @ts-ignore
        cleanup = () => mq.removeListener?.(sync);
      }
    }

    void loadPendingUsers();
    void loadTenantOptions();
    void loadRoleOptions();

    return cleanup;
  });

  function openApproveDialog(user: PendingUser) {
    approveTarget = user;
    approveTenantId = '';
    approveRoleId = '';
    showApproveDialog = true;
  }

  function closeApproveDialog() {
    showApproveDialog = false;
    approveTarget = null;
  }

  async function confirmApprove() {
    if (!approveTarget || !approveTenantId || !approveRoleId) return;
    approving = true;
    try {
      await superadmin.approvePendingUser(approveTarget.id, approveTenantId, approveRoleId);
      toast.success(
        $t('superadmin.pending_approvals.approved_success') || 'User approved successfully',
      );
      closeApproveDialog();
      await loadPendingUsers();
    } catch (err: any) {
      toast.error(
        err?.message ||
          $t('superadmin.pending_approvals.approve_failed') ||
          'Failed to approve user',
      );
    } finally {
      approving = false;
    }
  }

  function openRejectDialog(user: PendingUser) {
    rejectTarget = user;
    rejectReason = '';
    showRejectDialog = true;
  }

  function closeRejectDialog() {
    showRejectDialog = false;
    rejectTarget = null;
  }

  async function confirmReject() {
    if (!rejectTarget || !rejectReason.trim()) return;
    rejecting = true;
    try {
      await superadmin.rejectPendingUser(rejectTarget.id, rejectReason);
      toast.success(
        $t('superadmin.pending_approvals.rejected_success') || 'User rejected successfully',
      );
      closeRejectDialog();
      await loadPendingUsers();
    } catch (err: any) {
      toast.error(
        err?.message ||
          $t('superadmin.pending_approvals.reject_failed') ||
          'Failed to reject user',
      );
    } finally {
      rejecting = false;
    }
  }

  function formatRegistered(value: string) {
    try {
      return formatDateTime(value, { timeZone: $appSettings.app_timezone });
    } catch {
      return new Date(value).toLocaleString();
    }
  }

  function initials(name: string) {
    const parts = (name || '').trim().split(/\s+/).filter(Boolean);
    if (parts.length === 0) return '?';
    const first = parts[0]!.charAt(0);
    const last = parts.length > 1 ? parts[parts.length - 1]!.charAt(0) : '';
    return (first + last).toUpperCase();
  }
</script>

<div class="superadmin-content fade-in">
  <div class="page-header">
    <div class="header-content">
      <h1>{$t('superadmin.pending_approvals.title')}</h1>
      <p class="subtitle">
        {$t('superadmin.pending_approvals.subtitle')}
      </p>
    </div>
    <button
      class="btn-refresh"
      onclick={loadPendingUsers}
      disabled={loading}
      title={$t('common.refresh')}
      aria-label={$t('common.refresh')}
    >
      <Icon name="refresh-cw" size={18} />
    </button>
  </div>

  <div class="glass-card" in:fly={{ y: 20, delay: 60 }}>
    <div class="card-header">
      <div>
        <h3>{$t('superadmin.pending_approvals.queue_title')}</h3>
        <span class="muted">
          {$t('superadmin.pending_approvals.queue_subtitle')}
        </span>
      </div>
      {#if !loading && pendingUsers.length > 0}
        <span class="count-badge">
          {pendingUsers.length}
          {$t('superadmin.pending_approvals.count_badge')}
        </span>
      {/if}
    </div>

    {#if error}
      <div class="state error-state">
        <Icon name="alert-circle" size={28} />
        <p>{error}</p>
        <button class="btn btn-primary" onclick={loadPendingUsers}>
          {$t('common.retry')}
        </button>
      </div>
    {:else if loading}
      <div class="state loading-state">
        <div class="spinner"></div>
        <p>{$t('superadmin.pending_approvals.loading')}</p>
      </div>
    {:else if pendingUsers.length === 0}
      <div class="state empty-state">
        <Icon name="check-circle" size={42} strokeWidth={1.5} />
        <p class="empty-title">
          {$t('superadmin.pending_approvals.empty')}
        </p>
        <p class="empty-hint">
          {$t('superadmin.pending_approvals.empty_hint')}
        </p>
      </div>
    {:else if isMobile}
      <ul class="card-list">
        {#each pendingUsers as user (user.id)}
          <li class="user-card">
            <div class="user-card-head">
              <div class="avatar" aria-hidden="true">{initials(user.name)}</div>
              <div class="user-card-info">
                <span class="user-name">{user.name}</span>
                <span class="user-email">{user.email}</span>
              </div>
            </div>
            <dl class="user-card-meta">
              <div>
                <dt>{$t('superadmin.pending_approvals.col_registered')}</dt>
                <dd>{formatRegistered(user.created_at)}</dd>
              </div>
            </dl>
            <div class="user-card-actions">
              <button class="btn btn-primary" onclick={() => openApproveDialog(user)}>
                <Icon name="check-circle" size={16} />
                {$t('superadmin.pending_approvals.approve')}
              </button>
              <button class="btn btn-danger-outline" onclick={() => openRejectDialog(user)}>
                <Icon name="x" size={16} />
                {$t('superadmin.pending_approvals.reject')}
              </button>
            </div>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="table-wrapper">
        <table class="data-table">
          <thead>
            <tr>
              <th>{$t('superadmin.pending_approvals.col_name')}</th>
              <th>{$t('superadmin.pending_approvals.col_email')}</th>
              <th>{$t('superadmin.pending_approvals.col_registered')}</th>
              <th class="col-actions">{$t('common.actions')}</th>
            </tr>
          </thead>
          <tbody>
            {#each pendingUsers as user (user.id)}
              <tr>
                <td>
                  <div class="user-cell">
                    <div class="avatar" aria-hidden="true">{initials(user.name)}</div>
                    <span>{user.name}</span>
                  </div>
                </td>
                <td class="muted-cell">{user.email}</td>
                <td class="muted-cell">{formatRegistered(user.created_at)}</td>
                <td class="col-actions">
                  <div class="row-actions">
                    <button class="btn btn-primary btn-sm" onclick={() => openApproveDialog(user)}>
                      <Icon name="check-circle" size={14} />
                      {$t('superadmin.pending_approvals.approve')}
                    </button>
                    <button
                      class="btn btn-danger-outline btn-sm"
                      onclick={() => openRejectDialog(user)}
                    >
                      <Icon name="x" size={14} />
                      {$t('superadmin.pending_approvals.reject')}
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>

<!-- Approve Dialog -->
<Modal
  bind:show={showApproveDialog}
  title={`${$t('superadmin.pending_approvals.approve_dialog_title')}${approveTarget ? `: ${approveTarget.name}` : ''}`}
  width="480px"
  bodyOverflow="visible"
  onclose={closeApproveDialog}
>
  {#snippet children()}
    {#if approveTarget}
      <div class="dialog-summary">
        <div class="avatar avatar-lg" aria-hidden="true">{initials(approveTarget.name)}</div>
        <div>
          <div class="dialog-summary-name">{approveTarget.name}</div>
          <div class="dialog-summary-email">{approveTarget.email}</div>
        </div>
      </div>
    {/if}

    <div class="form-field">
      <span class="field-label">
        {$t('superadmin.pending_approvals.select_tenant')}
      </span>
      {#if tenantOptions.length === 0}
        <p class="field-empty">
          {$t('superadmin.pending_approvals.no_tenants')}
        </p>
      {:else}
        <Select
          bind:value={approveTenantId}
          options={tenantOptions}
          placeholder={$t('superadmin.pending_approvals.select_tenant_placeholder')}
          width="100%"
        />
      {/if}
    </div>

    <div class="form-field">
      <span class="field-label">
        {$t('superadmin.pending_approvals.select_role')}
      </span>
      {#if roleOptions.length === 0}
        <p class="field-empty">
          {$t('superadmin.pending_approvals.no_roles')}
        </p>
      {:else}
        <Select
          bind:value={approveRoleId}
          options={roleOptions}
          placeholder={$t('superadmin.pending_approvals.select_role_placeholder')}
          width="100%"
        />
      {/if}
    </div>
  {/snippet}

  {#snippet footer()}
    <button class="btn btn-secondary" onclick={closeApproveDialog} disabled={approving}>
      {$t('common.cancel')}
    </button>
    <button
      class="btn btn-primary"
      onclick={confirmApprove}
      disabled={approving || !approveTenantId || !approveRoleId}
    >
      {#if approving}
        <span class="spinner spinner-sm"></span>
        {$t('common.processing')}
      {:else}
        {$t('superadmin.pending_approvals.confirm_approve')}
      {/if}
    </button>
  {/snippet}
</Modal>

<!-- Reject Dialog -->
<Modal
  bind:show={showRejectDialog}
  title={`${$t('superadmin.pending_approvals.reject_dialog_title')}${rejectTarget ? `: ${rejectTarget.name}` : ''}`}
  width="480px"
  onclose={closeRejectDialog}
>
  {#snippet children()}
    {#if rejectTarget}
      <div class="dialog-summary">
        <div class="avatar avatar-lg avatar-danger" aria-hidden="true">
          {initials(rejectTarget.name)}
        </div>
        <div>
          <div class="dialog-summary-name">{rejectTarget.name}</div>
          <div class="dialog-summary-email">{rejectTarget.email}</div>
        </div>
      </div>
    {/if}

    <div class="form-field">
      <label class="field-label" for="reject-reason">
        {$t('superadmin.pending_approvals.reject_reason')}
      </label>
      <textarea
        id="reject-reason"
        class="textarea"
        rows="4"
        bind:value={rejectReason}
        placeholder={$t('superadmin.pending_approvals.reject_reason_placeholder')}
      ></textarea>
    </div>
  {/snippet}

  {#snippet footer()}
    <button class="btn btn-secondary" onclick={closeRejectDialog} disabled={rejecting}>
      {$t('common.cancel')}
    </button>
    <button
      class="btn btn-danger"
      onclick={confirmReject}
      disabled={rejecting || !rejectReason.trim()}
    >
      {#if rejecting}
        <span class="spinner spinner-sm"></span>
        {$t('common.processing')}
      {:else}
        {$t('superadmin.pending_approvals.confirm_reject')}
      {/if}
    </button>
  {/snippet}
</Modal>

<style>
  .superadmin-content {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1400px;
    margin: 0 auto;
    color: var(--text-primary);
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    min-width: 0;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .header-content h1 {
    font-size: 1.5rem;
    font-weight: 800;
    margin: 0 0 0.35rem 0;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.92rem;
    margin: 0;
  }

  .btn-refresh {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    width: 40px;
    height: 40px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
    flex-shrink: 0;
  }

  .btn-refresh:hover:not(:disabled) {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .btn-refresh:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .glass-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .glass-card {
    background: var(--bg-surface);
    border-color: var(--border-color);
    box-shadow: var(--shadow-sm);
  }

  .card-header {
    padding: 1.25rem 1.25rem 1rem 1.25rem;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .card-header h3 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -0.01em;
  }

  .muted {
    display: block;
    margin-top: 0.25rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .count-badge {
    background: color-mix(in srgb, var(--color-warning) 18%, var(--bg-tertiary));
    border: 1px solid color-mix(in srgb, var(--color-warning) 35%, var(--border-color));
    color: var(--text-primary);
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    font-size: 0.82rem;
    font-weight: 700;
    white-space: nowrap;
    align-self: flex-start;
  }

  /* States: loading / empty / error */
  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1.5rem;
    gap: 0.6rem;
    color: var(--text-secondary);
    text-align: center;
  }

  .empty-state .empty-title {
    color: var(--text-primary);
    font-weight: 700;
    margin: 0.25rem 0 0 0;
  }

  .empty-state .empty-hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .empty-state :global(svg) {
    color: var(--color-success, #10b981);
  }

  .error-state {
    background: color-mix(in srgb, var(--color-danger) 6%, transparent);
    color: var(--color-danger);
  }

  .error-state p {
    margin: 0;
    color: var(--color-danger);
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .spinner-sm {
    width: 14px;
    height: 14px;
    border-width: 2px;
    margin-right: 0.4rem;
    vertical-align: middle;
    display: inline-block;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Table */
  .table-wrapper {
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.92rem;
  }

  .data-table thead th {
    text-align: left;
    padding: 0.85rem 1.25rem;
    font-weight: 700;
    color: var(--text-secondary);
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    white-space: nowrap;
  }

  .data-table tbody td {
    padding: 0.85rem 1.25rem;
    border-bottom: 1px solid var(--border-color);
    color: var(--text-primary);
    vertical-align: middle;
  }

  .data-table tbody tr:last-child td {
    border-bottom: none;
  }

  .data-table tbody tr:hover {
    background: color-mix(in srgb, var(--color-primary) 4%, transparent);
  }

  .col-actions {
    text-align: right;
    white-space: nowrap;
  }

  .muted-cell {
    color: var(--text-secondary);
  }

  .user-cell {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
  }

  .row-actions {
    display: inline-flex;
    gap: 0.4rem;
    justify-content: flex-end;
  }

  /* Card list (mobile) */
  .card-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .user-card {
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    border-bottom: 1px solid var(--border-color);
  }

  .user-card:last-child {
    border-bottom: none;
  }

  .user-card-head {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    min-width: 0;
  }

  .user-card-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .user-name {
    font-weight: 700;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .user-email {
    color: var(--text-secondary);
    font-size: 0.88rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .user-card-meta {
    margin: 0;
    display: grid;
    grid-template-columns: 1fr;
    gap: 0.4rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
  }

  .user-card-meta dt {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    margin: 0;
  }

  .user-card-meta dd {
    margin: 0;
    color: var(--text-primary);
  }

  .user-card-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  /* Avatar */
  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--color-primary) 18%, var(--bg-tertiary));
    color: var(--color-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 0.78rem;
    flex-shrink: 0;
  }

  .avatar-lg {
    width: 44px;
    height: 44px;
    font-size: 0.95rem;
  }

  .avatar-danger {
    background: color-mix(in srgb, var(--color-danger) 18%, var(--bg-tertiary));
    color: var(--color-danger);
  }

  /* Buttons */
  .btn {
    appearance: none;
    border: 1px solid transparent;
    padding: 0.55rem 0.95rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 700;
    font-size: 0.9rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .btn-sm {
    padding: 0.4rem 0.7rem;
    font-size: 0.82rem;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  .btn-primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-primary) 88%, black);
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-color: var(--border-color);
  }

  .btn-secondary:hover:not(:disabled) {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .btn-danger {
    background: var(--color-danger);
    color: white;
    border-color: var(--color-danger);
  }

  .btn-danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-danger) 88%, black);
  }

  .btn-danger-outline {
    background: transparent;
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 35%, var(--border-color));
  }

  .btn-danger-outline:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    border-color: var(--color-danger);
  }

  /* Dialog form */
  .dialog-summary {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 0.85rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md, 10px);
    margin-bottom: 1rem;
    min-width: 0;
  }

  .dialog-summary-name {
    font-weight: 700;
    color: var(--text-primary);
  }

  .dialog-summary-email {
    color: var(--text-secondary);
    font-size: 0.88rem;
    overflow-wrap: anywhere;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-bottom: 1rem;
  }

  .form-field:last-child {
    margin-bottom: 0;
  }

  .field-label {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .field-empty {
    margin: 0.25rem 0 0;
    font-size: 0.85rem;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    border: 1px dashed var(--border-color);
    padding: 0.55rem 0.75rem;
    border-radius: 8px;
  }

  .textarea {
    width: 100%;
    padding: 0.6rem 0.75rem;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.92rem;
    resize: vertical;
    min-height: 80px;
    transition:
      border-color 0.15s,
      box-shadow 0.15s;
  }

  .textarea:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 18%, transparent);
  }

  /* Mobile tweaks */
  @media (max-width: 720px) {
    .page-header {
      flex-wrap: wrap;
    }

    .header-content h1 {
      font-size: 1.25rem;
    }

    .card-header {
      padding: 1rem 1rem 0.85rem;
    }
  }

  @media (max-width: 420px) {
    .user-card-actions {
      grid-template-columns: 1fr;
    }
  }
</style>
