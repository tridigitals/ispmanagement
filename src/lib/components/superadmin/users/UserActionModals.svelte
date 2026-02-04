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
  }>();
</script>

<ConfirmDialog
  bind:show={showResetConfirm}
  title={$t('superadmin.users.reset2fa.title') || 'Reset Two-Factor Authentication'}
  message={$t('superadmin.users.reset2fa.message') ||
    'Reset 2FA for this user? They will be able to login without a secondary code.'}
  confirmText={$t('superadmin.users.reset2fa.confirm') || 'Reset 2FA'}
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
