<script lang="ts">
  import type {
    SuperadminManagedRadiusAssignment,
    ManagedRadiusMappingPayload,
    Tenant,
  } from '$lib/api/types';
  import Input from '$lib/components/ui/Input.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import { routerToOptionLabel } from '$lib/utils/managedRadiusControlPlane';
  import { t } from 'svelte-i18n';

  type RouterOption = {
    id: string;
    tenant_id?: string | null;
    name?: string | null;
    host?: string | null;
  };

  let {
    show = $bindable(false),
    loading = false,
    isEditing = false,
    mapping = $bindable(),
    tenants = [],
    assignments = [],
    routers = [],
    onGenerateSecret,
    onSubmit,
  } = $props<{
    show: boolean;
    loading: boolean;
    isEditing: boolean;
    mapping: ManagedRadiusMappingPayload;
    tenants: Tenant[];
    assignments: SuperadminManagedRadiusAssignment[];
    routers: RouterOption[];
    onGenerateSecret: () => void;
    onSubmit: () => void;
  }>();

  let tenantOptions = $derived(
    tenants.map((tenant: Tenant) => ({
      label: tenant.name,
      value: tenant.id,
    })),
  );

  let endpointOptions = $derived(
    assignments
      .filter(
        (assignment: SuperadminManagedRadiusAssignment) =>
          !mapping.tenant_id || assignment.tenant_id === mapping.tenant_id,
      )
      .map((assignment: SuperadminManagedRadiusAssignment) => ({
        label: `${assignment.endpoint_name} (${assignment.radius_host})`,
        value: assignment.radius_endpoint_id,
      })),
  );

  let routerOptions = $derived(
    routers
      .filter(
        (router: RouterOption) => !mapping.tenant_id || router.tenant_id === mapping.tenant_id,
      )
      .map((router: RouterOption) => ({
        label: routerToOptionLabel(router),
        value: router.id,
      })),
  );

  $effect(() => {
    const activeAssignment = assignments.find(
      (assignment: SuperadminManagedRadiusAssignment) => assignment.tenant_id === mapping.tenant_id,
    );
    if (activeAssignment) {
      mapping.radius_endpoint_id = activeAssignment.radius_endpoint_id;
    }

    if (
      mapping.radius_endpoint_id &&
      !endpointOptions.some((option: { label: string; value: string }) => option.value === mapping.radius_endpoint_id)
    ) {
      mapping.radius_endpoint_id = '';
    }

    if (
      mapping.router_id &&
      !routerOptions.some((option: { label: string; value: string }) => option.value === mapping.router_id)
    ) {
      mapping.router_id = '';
    }
  });
</script>

<Modal
  bind:show
  title={isEditing
    ? $t('superadmin.radius.modals.mapping.edit_title') || 'Edit NAS mapping'
    : $t('superadmin.radius.modals.mapping.create_title') || 'New NAS mapping'}
  width="760px"
>
  <div class="modal-form">
    <div class="grid three">
      <Select
        label={$t('superadmin.radius.form.tenant') || 'Tenant'}
        options={tenantOptions}
        bind:value={mapping.tenant_id}
        placeholder={$t('superadmin.radius.form.select_tenant') || 'Select a tenant'}
        disabled={loading || isEditing}
      />
      <Select
        label={$t('superadmin.radius.form.server') || 'Server'}
        options={endpointOptions}
        bind:value={mapping.radius_endpoint_id}
        placeholder={$t('superadmin.radius.form.select_server') || 'Active tenant assignment required'}
        disabled
      />
      <Select
        label={$t('superadmin.radius.form.router') || 'Router'}
        options={routerOptions}
        bind:value={mapping.router_id}
        placeholder={$t('superadmin.radius.form.select_router') || 'Select a router'}
        disabled={loading || !mapping.tenant_id}
      />
    </div>

    <div class="grid two">
      <Input
        label={$t('superadmin.radius.form.nas_name') || 'NAS name'}
        bind:value={mapping.nas_name}
        placeholder={$t('superadmin.radius.form.nas_name_placeholder') || 'router-pop-a'}
        disabled={loading}
      />
      <Input
        label={$t('superadmin.radius.form.shortname') || 'Shortname'}
        bind:value={mapping.shortname}
        placeholder={$t('superadmin.radius.form.shortname_placeholder') || 'POP-A'}
        disabled={loading}
      />
    </div>

    <Input
      label={$t('superadmin.radius.form.nas_ip_or_cidr') || 'NAS IP / CIDR'}
      bind:value={mapping.nas_ip_or_cidr}
      placeholder="10.10.10.1/32"
      disabled={loading}
    />

    <div class="secret-row">
      <Input
        label={$t('superadmin.radius.form.shared_secret') || 'Shared secret'}
        type="password"
        bind:value={mapping.shared_secret}
        placeholder={$t('superadmin.radius.form.shared_secret_placeholder') || 'Leave blank to auto-generate'}
        disabled={loading}
        showPasswordToggle
      />
      <button class="btn btn-secondary btn-inline" type="button" onclick={onGenerateSecret} disabled={loading}>
        {$t('superadmin.radius.actions.generate_secret') || 'Generate'}
      </button>
    </div>

    <label class="toggle-row">
      <input type="checkbox" bind:checked={mapping.is_active} disabled={loading} />
      <span>{$t('superadmin.radius.form.active_mapping_hint') || 'Enable this NAS mapping immediately'}</span>
    </label>

    <div class="modal-actions">
      <button class="btn btn-secondary" type="button" onclick={() => (show = false)} disabled={loading}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn btn-primary" type="button" onclick={onSubmit} disabled={loading}>
        {#if loading}<span class="spinner-sm"></span>{/if}
        {isEditing
          ? $t('superadmin.radius.actions.save_mapping') || 'Save mapping'
          : $t('superadmin.radius.actions.create_mapping') || 'Create mapping'}
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

  .secret-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.75rem;
    align-items: end;
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

  .btn-inline {
    min-height: 44px;
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
    .grid.three,
    .secret-row {
      grid-template-columns: 1fr;
    }
  }
</style>
