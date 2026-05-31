<script lang="ts">
  import { isSuperAdmin } from '$lib/stores/auth';
  import { superadmin, type PendingUser } from '$lib/api/superadmin';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';

  let pendingUsers: PendingUser[] = $state([]);
  let loading = $state(true);
  let error = $state('');

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

  onMount(() => {
    if (!$isSuperAdmin) {
      goto('/dashboard');
      return;
    }
    loadPendingUsers();
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
      toast.success($t('superadmin.pending_approvals.approved_success'));
      closeApproveDialog();
      await loadPendingUsers();
    } catch (err: any) {
      toast.error(err?.message || 'Failed to approve user');
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
      toast.success($t('superadmin.pending_approvals.rejected_success'));
      closeRejectDialog();
      await loadPendingUsers();
    } catch (err: any) {
      toast.error(err?.message || 'Failed to reject user');
    } finally {
      rejecting = false;
    }
  }
</script>

<div class="p-6">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-2xl font-bold">{$t('superadmin.pending_approvals.title')}</h1>
    <button class="btn btn-outline btn-sm" onclick={loadPendingUsers} disabled={loading}>
      <Icon name="refresh-cw" size={16} />
    </button>
  </div>

  {#if error}
    <div class="alert error mb-4">{error}</div>
  {/if}

  {#if loading}
    <div class="text-center py-8 text-gray-500">Loading...</div>
  {:else if pendingUsers.length === 0}
    <div class="text-center py-8 text-gray-500">
      <Icon name="check-circle" size={48} strokeWidth={1.5} />
      <p class="mt-2">{$t('superadmin.pending_approvals.empty')}</p>
    </div>
  {:else}
    <div class="overflow-x-auto">
      <table class="w-full text-sm">
        <thead>
          <tr class="border-b border-gray-200 dark:border-gray-700">
            <th class="text-left py-3 px-4 font-medium">Name</th>
            <th class="text-left py-3 px-4 font-medium">Email</th>
            <th class="text-left py-3 px-4 font-medium">Registered</th>
            <th class="text-right py-3 px-4 font-medium">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each pendingUsers as user (user.id)}
            <tr class="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/50">
              <td class="py-3 px-4">{user.name}</td>
              <td class="py-3 px-4">{user.email}</td>
              <td class="py-3 px-4">{new Date(user.created_at).toLocaleDateString()}</td>
              <td class="py-3 px-4 text-right">
                <button
                  class="btn btn-primary btn-sm mr-2"
                  onclick={() => openApproveDialog(user)}
                >
                  {$t('superadmin.pending_approvals.approve')}
                </button>
                <button
                  class="btn btn-outline btn-sm text-red-600"
                  onclick={() => openRejectDialog(user)}
                >
                  {$t('superadmin.pending_approvals.reject')}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Approve Dialog -->
{#if showApproveDialog && approveTarget}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-md">
      <h2 class="text-lg font-semibold mb-4">
        {$t('superadmin.pending_approvals.approve_dialog_title')}: {approveTarget.name}
      </h2>
      <div class="mb-4">
        <label class="block text-sm font-medium mb-1" for="tenant-id">
          {$t('superadmin.pending_approvals.select_tenant')}
        </label>
        <input
          id="tenant-id"
          type="text"
          class="input w-full"
          bind:value={approveTenantId}
          placeholder="Tenant ID"
        />
      </div>
      <div class="mb-4">
        <label class="block text-sm font-medium mb-1" for="role-id">
          {$t('superadmin.pending_approvals.select_role')}
        </label>
        <input
          id="role-id"
          type="text"
          class="input w-full"
          bind:value={approveRoleId}
          placeholder="Role ID"
        />
      </div>
      <div class="flex justify-end gap-2">
        <button class="btn btn-outline" onclick={closeApproveDialog}>Cancel</button>
        <button
          class="btn btn-primary"
          onclick={confirmApprove}
          disabled={approving || !approveTenantId || !approveRoleId}
        >
          {approving ? '...' : $t('superadmin.pending_approvals.confirm_approve')}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Reject Dialog -->
{#if showRejectDialog && rejectTarget}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-md">
      <h2 class="text-lg font-semibold mb-4">
        {$t('superadmin.pending_approvals.reject_dialog_title')}: {rejectTarget.name}
      </h2>
      <div class="mb-4">
        <label class="block text-sm font-medium mb-1" for="reject-reason">
          {$t('superadmin.pending_approvals.reject_reason')}
        </label>
        <textarea
          id="reject-reason"
          class="input w-full"
          rows="3"
          bind:value={rejectReason}
          placeholder="Reason..."
        ></textarea>
      </div>
      <div class="flex justify-end gap-2">
        <button class="btn btn-outline" onclick={closeRejectDialog}>Cancel</button>
        <button
          class="btn btn-outline text-red-600"
          onclick={confirmReject}
          disabled={rejecting || !rejectReason.trim()}
        >
          {rejecting ? '...' : $t('superadmin.pending_approvals.confirm_reject')}
        </button>
      </div>
    </div>
  </div>
{/if}
