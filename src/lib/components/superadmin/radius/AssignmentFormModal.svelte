<script lang="ts">
  import type {
    ManagedRadiusAssignmentPayload,
    SuperadminManagedRadiusServer,
    Tenant,
  } from '$lib/api/types';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import { t } from 'svelte-i18n';

  let {
    show = $bindable(false),
    loading = false,
    isEditing = false,
    assignment = $bindable(),
    tenants = [],
    servers = [],
    onSubmit,
  } = $props<{
    show: boolean;
    loading: boolean;
    isEditing: boolean;
    assignment: ManagedRadiusAssignmentPayload;
    tenants: Tenant[];
    servers: SuperadminManagedRadiusServer[];
    onSubmit: () => void;
  }>();

  let tenantOptions = $derived(
    tenants.map((tenant: Tenant) => ({
      label: tenant.name,
      value: tenant.id,
    })),
  );

  let serverOptions = $derived(
    servers.map((server: SuperadminManagedRadiusServer) => ({
      label: `${server.name} (${server.endpoint_host})`,
      value: server.id,
    })),
  );
</script>

<Modal
  bind:show
  title={isEditing
    ? $t('superadmin.radius.modals.assignment.edit_title') || 'Edit tenant assignment'
    : $t('superadmin.radius.modals.assignment.create_title') || 'New tenant assignment'}
  width="640px"
>
  <div class="modal-form">
    <div class="grid two">
      <Select
        label={$t('superadmin.radius.form.tenant')}
        options={tenantOptions}
        bind:value={assignment.tenant_id}
        placeholder={$t('superadmin.radius.form.select_tenant')}
        disabled={loading || isEditing}
      />
      <Select
        label={$t('superadmin.radius.form.server')}
        options={serverOptions}
        bind:value={assignment.radius_endpoint_id}
        placeholder={$t('superadmin.radius.form.select_server')}
        disabled={loading}
      />
    </div>

    <label class="toggle-row">
      <input type="checkbox" bind:checked={assignment.is_active} disabled={loading} />
      <span>{$t('superadmin.radius.form.active_assignment_hint')}</span>
    </label>

    <div class="modal-actions">
      <button class="btn btn-secondary" type="button" onclick={() => (show = false)} disabled={loading}>
        {$t('common.cancel')}
      </button>
      <button class="btn btn-primary" type="button" onclick={onSubmit} disabled={loading}>
        {#if loading}<span class="spinner-sm"></span>{/if}
        {isEditing
          ? $t('superadmin.radius.actions.save_assignment') || 'Save assignment'
          : $t('superadmin.radius.actions.create_assignment') || 'Create assignment'}
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
    .grid.two {
      grid-template-columns: 1fr;
    }
  }
</style>
