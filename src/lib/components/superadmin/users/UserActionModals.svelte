<script lang="ts">
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { t } from 'svelte-i18n';

  let {
    showResetConfirm = $bindable(false),
    confirmLoading = false,
    onReset2FA,
    showStatusConfirm = $bindable(false),
    statusConfirmTitle,
    statusConfirmMessage,
    statusConfirmKeyword,
    statusConfirmType,
    statusConfirmLoading,
    onToggleActive,
    pendingIsActive,
    showDeleteConfirm = $bindable(false),
    deleteConfirmTitle,
    deleteConfirmMessage,
    deleteConfirmMessageValues,
    deleteConfirmLoading = false,
    onDelete,
  } = $props<{
    showResetConfirm: boolean;
    confirmLoading: boolean;
    onReset2FA: () => void;
    showStatusConfirm: boolean;
    statusConfirmTitle: string;
    statusConfirmMessage: string;
    statusConfirmKeyword: string;
    statusConfirmType: 'danger' | 'warning' | 'info';
    statusConfirmLoading: boolean;
    onToggleActive: () => void;
    pendingIsActive: boolean;
    showDeleteConfirm: boolean;
    deleteConfirmTitle?: string;
    deleteConfirmMessage?: string;
    deleteConfirmMessageValues?: Record<string, string>;
    deleteConfirmLoading?: boolean;
    onDelete?: () => void;
  }>();
</script>

<ConfirmDialog
  bind:show={showResetConfirm}
  title={$t('superadmin.users.reset2fa.title')}
  message={$t('superadmin.users.reset2fa.message')}
  confirmText={$t('superadmin.users.reset2fa.confirm')}
  confirmationKeyword="RESET"
  type="warning"
  loading={confirmLoading}
  onconfirm={onReset2FA}
/>

<ConfirmDialog
  bind:show={showStatusConfirm}
  title={statusConfirmTitle}
  message={statusConfirmMessage}
  confirmText={pendingIsActive
    ? $t('superadmin.users.actions.activate') || 'Activate'
    : $t('superadmin.users.actions.deactivate') || 'Deactivate'}
  confirmationKeyword={statusConfirmKeyword}
  type={statusConfirmType}
  loading={statusConfirmLoading}
  onconfirm={onToggleActive}
/>

<ConfirmDialog
  bind:show={showDeleteConfirm}
  title={
    deleteConfirmTitle ||
    $t('superadmin.users.delete.title') ||
    'Delete user permanently?'
  }
  message={
    deleteConfirmMessage ||
    $t('superadmin.users.delete.message', {
      values:
        deleteConfirmMessageValues || { name: 'this user', email: '—' },
    }) ||
    'You are about to permanently delete this user. Their account, sessions, and active tabs will be removed immediately. This cannot be undone. Type DELETE to confirm.'
  }
  confirmText={$t('superadmin.users.delete.confirm') || 'Delete permanently'}
  confirmationKeyword="DELETE"
  type="danger"
  loading={deleteConfirmLoading}
  onconfirm={() => onDelete?.()}
/>
