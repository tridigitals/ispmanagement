<script lang="ts">
  import type { ManagedRadiusServerPayload } from '$lib/api/types';
  import Input from '$lib/components/ui/Input.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import { t } from 'svelte-i18n';

  let {
    show = $bindable(false),
    loading = false,
    isEditing = false,
    server = $bindable(),
    onSubmit,
  } = $props<{
    show: boolean;
    loading: boolean;
    isEditing: boolean;
    server: ManagedRadiusServerPayload;
    onSubmit: () => void;
  }>();
</script>

<Modal
  bind:show
  title={isEditing
    ? $t('superadmin.radius.modals.server.edit_title') || 'Edit Managed RADIUS server'
    : $t('superadmin.radius.modals.server.create_title') || 'New Managed RADIUS server'}
  width="640px"
>
  <div class="modal-form">
    <div class="grid two">
      <Input
        label={$t('superadmin.radius.form.server_name') || 'Server name'}
        bind:value={server.name}
        placeholder={$t('superadmin.radius.form.server_name_placeholder') || 'Primary RADIUS DB'}
        disabled={loading}
      />
      <Input
        label={$t('superadmin.radius.form.db_host') || 'DB host'}
        bind:value={server.db_host}
        placeholder="127.0.0.1"
        disabled={loading}
      />
    </div>

    <div class="grid three">
      <Input
        label={$t('superadmin.radius.form.db_port') || 'DB port'}
        type="number"
        bind:value={server.db_port}
        placeholder="5432"
        disabled={loading}
      />
      <Input
        label={$t('superadmin.radius.form.db_name') || 'DB name'}
        bind:value={server.db_name}
        placeholder="radius"
        disabled={loading}
      />
      <Input
        label={$t('superadmin.radius.form.db_user') || 'DB user'}
        bind:value={server.db_user}
        placeholder="radius"
        disabled={loading}
      />
    </div>

    <Input
      label={$t('superadmin.radius.form.db_password') || 'DB password'}
      type="password"
      bind:value={server.db_password}
      placeholder={isEditing
        ? $t('superadmin.radius.form.db_password_placeholder_edit') || 'Leave blank to keep current password'
        : $t('superadmin.radius.form.db_password_placeholder') || 'Encrypted at rest'}
      disabled={loading}
      showPasswordToggle
    />

    <Input
      label={$t('superadmin.radius.form.notes') || 'Notes'}
      bind:value={server.notes}
      placeholder={$t('superadmin.radius.form.notes_placeholder') || 'Optional operational note'}
      disabled={loading}
    />

    <label class="toggle-row">
      <input type="checkbox" bind:checked={server.is_active} disabled={loading} />
      <span>{$t('superadmin.radius.form.active_server_hint') || 'Allow this global server to receive tenant assignments'}</span>
    </label>

    <div class="modal-actions">
      <button class="btn btn-secondary" type="button" onclick={() => (show = false)} disabled={loading}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn btn-primary" type="button" onclick={onSubmit} disabled={loading}>
        {#if loading}<span class="spinner-sm"></span>{/if}
        {isEditing
          ? $t('superadmin.radius.actions.save_server') || 'Save server'
          : $t('superadmin.radius.actions.create_server') || 'Create server'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .modal-form {
    display: grid;
    gap: 1rem;
  }

  .grid {
    display: grid;
    gap: 1rem;
  }

  .grid.two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .grid.three {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .toggle-row {
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
    color: var(--text-secondary);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .spinner-sm {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.35);
    border-top-color: white;
    border-radius: 999px;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 720px) {
    .grid.two,
    .grid.three {
      grid-template-columns: 1fr;
    }
  }
</style>
