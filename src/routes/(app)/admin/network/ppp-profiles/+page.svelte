<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import PppProfileFormDialog, {
    type PppProfileFormModel,
  } from '$lib/components/network/PppProfileFormDialog.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import {
    getPppProfileCrudGateState,
    getPppProfileDeleteState,
    getPppProfileMutationErrorState,
    getPppProfileOnlyOneState,
    isPppProfileStaleTargetConflict,
    normalizePppProfilePayload,
  } from '$lib/utils/pppProfileCrud';
  import {
    getPppProfileRemotePoolOptions,
    getPppProfileRemotePoolValue,
  } from '$lib/utils/pppProfileRemotePool';

  type RouterRow = {
    id: string;
    name: string;
    host: string;
    port: number;
    is_online: boolean;
  };

  type PppProfileRow = {
    id: string;
    name: string;
    local_address?: string | null;
    remote_address?: string | null;
    rate_limit?: string | null;
    dns_server?: string | null;
    comment?: string | null;
    only_one?: boolean | null;
    router_present: boolean;
    last_sync_at?: string | null;
  };

  type IpPoolRow = {
    id: string;
    name: string;
  };

  let loadingRouters = $state(true);
  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');

  let loading = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let rows = $state<PppProfileRow[]>([]);
  let ipPools = $state<IpPoolRow[]>([]);
  let showForm = $state(false);
  let editing = $state<PppProfileRow | null>(null);
  let form = $state<PppProfileFormModel>({
    name: '',
    local_address: '',
    remote_address: '',
    rate_limit: '',
    dns_server: '',
    comment: '',
    only_one: false,
  });
  let showDelete = $state(false);
  let deleteTarget = $state<PppProfileRow | null>(null);
  let deleteMessage = $state('');
  let deleteKeyword = $state('');
  let deleteBlocked = $state(false);

  const columns = $derived([
    { key: 'name', label: $t('admin.network.routers.ppp_profiles.columns.name') || 'Name' },
    {
      key: 'local',
      label: $t('admin.network.routers.ppp_profiles.columns.local') || 'Local',
      class: 'mono',
      width: '170px',
    },
    {
      key: 'remote',
      label: $t('admin.network.routers.ppp_profiles.columns.remote') || 'Remote',
      class: 'mono',
      width: '170px',
    },
    {
      key: 'rate',
      label: $t('admin.network.routers.ppp_profiles.columns.rate') || 'Rate',
      class: 'mono',
      width: '160px',
    },
    {
      key: 'dns',
      label: $t('admin.network.routers.ppp_profiles.columns.dns') || 'DNS',
      class: 'mono',
      width: '170px',
    },
    {
      key: 'only_one',
      label: $t('admin.network.routers.ppp_profiles.columns.only_one') || 'Only one',
      width: '120px',
    },
    {
      key: 'state',
      label: $t('admin.network.routers.ppp_profiles.columns.state') || 'State',
      width: '120px',
    },
    {
      key: 'synced',
      label: $t('admin.network.routers.ppp_profiles.columns.synced') || 'Synced',
      class: 'mono',
      width: '130px',
    },
    { key: 'actions', label: $t('common.actions') || 'Actions', width: '120px' },
  ]);

  const tableData = $derived.by(() =>
    rows.map((r, idx) => ({
      id: r.id || `${r.name}:${idx}`,
      name: r.name,
      local: r.local_address || '—',
      remote: r.remote_address || '—',
      rate: r.rate_limit || '—',
      dns: r.dns_server || '—',
      only_one: getPppProfileOnlyOneState(r.only_one).enabled,
      state: Boolean(r.router_present),
      synced: r.last_sync_at,
      comment: r.comment,
      raw: r,
    })),
  );

  const remotePoolOptions = $derived.by(() => getPppProfileRemotePoolOptions(ipPools));

  onMount(async () => {
    if (!$can('read', 'ppp_profiles') && !$can('manage', 'ppp_profiles')) {
      goto('/unauthorized');
      return;
    }
    await loadRouters();
  });

  async function loadRouters() {
    loadingRouters = true;
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      if (routerId) await load();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loadingRouters = false;
    }
  }

  async function load() {
    if (!routerId) return;
    if (loading) return;
    loading = true;
    try {
      const [profileRows, poolRows] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId),
        api.mikrotik.routers.ipPools(routerId),
      ]);
      rows = profileRows as any;
      ipPools = poolRows as any;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  async function sync() {
    if (!routerId) return;
    if (loading) return;
    loading = true;
    try {
      rows = (await api.mikrotik.routers.syncPppProfiles(routerId)) as any;
      ipPools = (await api.mikrotik.routers.ipPools(routerId)) as any;
      toast.success($t('admin.network.routers.ppp_profiles.toasts.synced') || 'Synced PPP profiles');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  function resetForm() {
    form = {
      name: '',
      local_address: '',
      remote_address: '',
      rate_limit: '',
      dns_server: '',
      comment: '',
      only_one: false,
    };
  }

  function openCreate() {
    const gate = getPppProfileCrudGateState(routerId);
    if (gate.blocked) {
      toast.error($t('admin.network.routers.ppp_profiles.form.router_required') || 'Select a router first');
      return;
    }
    editing = null;
    resetForm();
    showForm = true;
  }

  function openEdit(row: PppProfileRow) {
    const gate = getPppProfileCrudGateState(routerId);
    if (gate.blocked) {
      toast.error($t('admin.network.routers.ppp_profiles.form.router_required') || 'Select a router first');
      return;
    }
    editing = row;
    form = {
      name: row.name || '',
      local_address: row.local_address || '',
      remote_address: getPppProfileRemotePoolValue(remotePoolOptions, row.remote_address),
      rate_limit: row.rate_limit || '',
      dns_server: row.dns_server || '',
      comment: row.comment || '',
      only_one: Boolean(row.only_one),
    };
    showForm = true;
  }

  function normalizedPayload() {
    return normalizePppProfilePayload(form);
  }

  async function save() {
    if (!routerId) {
      toast.error($t('admin.network.routers.ppp_profiles.form.router_required') || 'Select a router first');
      return;
    }
    saving = true;
    try {
      const payload = normalizedPayload();
      if (!payload.name && !editing) {
        throw new Error(
          ($t('admin.network.routers.ppp_profiles.form.name_required') as string) ||
            'Profile name is required',
        );
      }
      if (editing) {
        await api.mikrotik.routers.updatePppProfile(routerId, editing.id, payload);
        toast.success(
          ($t('admin.network.routers.ppp_profiles.toasts.updated') as string) ||
            'PPP profile updated',
        );
      } else {
        await api.mikrotik.routers.createPppProfile(routerId, payload as any);
        toast.success(
          ($t('admin.network.routers.ppp_profiles.toasts.created') as string) ||
            'PPP profile created',
        );
      }
      showForm = false;
      editing = null;
      await load();
    } catch (error: any) {
      const state = getPppProfileMutationErrorState(
        String(error?.message || '').includes('mirror')
          ? 'mirror_sync_failed'
          : 'router_write_failed',
      );
      toast.error(error?.message || state.message);
    } finally {
      saving = false;
    }
  }

  async function openDelete(row: PppProfileRow) {
    if (!routerId) {
      toast.error($t('admin.network.routers.ppp_profiles.form.router_required') || 'Select a router first');
      return;
    }
    try {
      const dependency = await api.mikrotik.routers.pppProfileDependencies(routerId, row.id);
      const counts = Object.fromEntries(
        (dependency.dependencies || []).map((item) => [item.type, item.count]),
      );
      const state = getPppProfileDeleteState(counts as any);
      deleteTarget = row;
      deleteBlocked = state.blocked;
      deleteKeyword = state.blocked ? '__blocked__' : row.name;
      deleteMessage = state.blocked
        ? $t('admin.network.routers.ppp_profiles.delete.blocked', {
            values: { name: row.name, count: state.totalDependencies },
          }) ||
          `Cannot delete ${row.name}. It is still used by ${state.totalDependencies} record(s).`
        : $t('admin.network.routers.ppp_profiles.delete.confirm', {
            values: { name: row.name },
          }) || `Delete PPP profile ${row.name} from the router?`;
      showDelete = true;
    } catch (error: any) {
      toast.error(error?.message || error);
    }
  }

  async function confirmDelete() {
    if (!routerId || !deleteTarget) return;
    if (deleteBlocked) {
      toast.error(deleteMessage);
      return;
    }
    deleting = true;
    try {
      await api.mikrotik.routers.deletePppProfile(routerId, deleteTarget.id);
      toast.success(
        ($t('admin.network.routers.ppp_profiles.toasts.deleted') as string) ||
          'PPP profile deleted',
      );
      showDelete = false;
      deleteTarget = null;
      deleteBlocked = false;
      await load();
    } catch (error: any) {
      const message = error?.message || String(error || '');
      if (isPppProfileStaleTargetConflict(message)) {
        try {
          rows = (await api.mikrotik.routers.syncPppProfiles(routerId)) as any;
          ipPools = (await api.mikrotik.routers.ipPools(routerId)) as any;
          showDelete = false;
          deleteTarget = null;
          deleteMessage = '';
          deleteKeyword = '';
          deleteBlocked = false;
          if (typeof toast.warning === 'function') {
            toast.warning(
              ($t('admin.network.routers.ppp_profiles.toasts.stale_deleted_sync') as string) ||
                'This PPP profile was already missing on the router. The list has been refreshed.',
            );
          } else {
            toast.success(
              ($t('admin.network.routers.ppp_profiles.toasts.stale_deleted_sync') as string) ||
                'This PPP profile was already missing on the router. The list has been refreshed.',
            );
          }
          return;
        } catch (syncError: any) {
          toast.error(syncError?.message || message);
          return;
        }
      }
      toast.error(message);
    } finally {
      deleting = false;
    }
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={$t('admin.network.routers.ppp_profiles.title') || 'PPP Profiles'}
    subtitle={$t('admin.network.routers.ppp_profiles.subtitle') || 'Per-router RouterOS PPP profiles (synced into database for mapping & import).'}
  >
    {#snippet actions()}
      {#if loading}
        <span class="syncing"><span class="spin"><Icon name="refresh-cw" size={14} /></span>{$t('common.loading') || 'Loading...'}</span>
      {:else}
        <span class="syncing">{$t('common.updated') || 'Updated'}</span>
      {/if}
      <button class="btn ghost" type="button" onclick={() => void load()} disabled={!routerId || loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      <button class="btn ghost" type="button" onclick={() => void sync()} disabled={!routerId || loading}>
        <Icon name="download" size={16} />
        {$t('admin.network.routers.ppp_profiles.actions.sync') || 'Sync from router'}
      </button>
      {#if $can('manage', 'ppp_profiles')}
        <button class="btn ghost" type="button" onclick={openCreate} disabled={!routerId || loading}>
          <Icon name="plus" size={16} />
          {$t('admin.network.routers.ppp_profiles.actions.add') || 'Add profile'}
        </button>
      {/if}
    {/snippet}
  </NetworkPageHeader>

  <div class="filters-wrap">
    <NetworkFilterPanel>
      <div class="control">
        <label for="ppp-profiles-router">{$t('admin.customers.pppoe.fields.router') || 'Router'}</label>
        <select
          id="ppp-profiles-router"
          class="input"
          bind:value={routerId}
          disabled={loadingRouters}
          onchange={() => void load()}
        >
          <option value="">{($t('common.select') || 'Select') + '...'}</option>
          {#each routers as r}
            <option value={r.id}>{r.name}</option>
          {/each}
        </select>
      </div>
    </NetworkFilterPanel>
  </div>

  {#if !routerId}
    <div class="empty">
      <Icon name="info" size={18} />
      <span>{$t('common.select') || 'Select'} router…</span>
    </div>
  {:else if tableData.length === 0 && !loading}
    <div class="empty">
      <Icon name="info" size={18} />
      <span>{$t('admin.network.routers.ppp_profiles.empty') || 'No profiles found.'}</span>
    </div>
  {:else}
    <div class="table-wrap">
      <Table
        columns={columns}
        data={tableData}
        keyField="id"
        pagination={true}
        pageSize={10}
        searchable={true}
        searchPlaceholder={$t('admin.network.routers.ppp_profiles.search') || 'Search profiles...'}
        mobileView="scroll"
      >
        {#snippet cell({ item, key }: any)}
          {#if key === 'state'}
            {#if item.state}
              <span class="pill ok">{$t('admin.network.routers.ppp_profiles.state.present') || 'On router'}</span>
            {:else}
              <span class="pill warn">{$t('admin.network.routers.ppp_profiles.state.missing') || 'Missing'}</span>
            {/if}
          {:else if key === 'only_one'}
            {#if item.only_one}
              <span class="pill ok">{$t('common.yes') || 'Yes'}</span>
            {:else}
              <span class="pill muted">{$t('common.no') || 'No'}</span>
            {/if}
          {:else if key === 'synced'}
            {#if item.synced}
              <span title={formatDateTime(item.synced, { timeZone: $appSettings.app_timezone })}>
                {timeAgo(item.synced)}
              </span>
            {:else}
              <span class="muted">—</span>
            {/if}
          {:else if key === 'actions'}
            <div class="actions">
              {#if $can('manage', 'ppp_profiles')}
                <button class="icon-btn" type="button" onclick={() => openEdit(item.raw)} title={$t('admin.network.routers.ppp_profiles.actions.edit') || 'Edit'}>
                  <Icon name="edit" size={16} />
                </button>
                <button class="icon-btn danger" type="button" onclick={() => void openDelete(item.raw)} title={$t('admin.network.routers.ppp_profiles.actions.delete') || 'Delete'}>
                  <Icon name="trash-2" size={16} />
                </button>
              {/if}
            </div>
          {:else}
            {item[key] ?? ''}
          {/if}
        {/snippet}
      </Table>
    </div>
  {/if}
</div>

<PppProfileFormDialog
  bind:show={showForm}
  loading={saving}
  isEditing={Boolean(editing)}
  bind:profile={form}
  remotePoolOptions={remotePoolOptions}
  onSubmit={() => void save()}
/>

<ConfirmDialog
  bind:show={showDelete}
  title={$t('admin.network.routers.ppp_profiles.delete.title') || 'Delete PPP Profile'}
  message={deleteMessage}
  confirmText={$t('common.delete') || 'Delete'}
  confirmationKeyword={deleteKeyword}
  loading={deleting}
  onconfirm={() => void confirmDelete()}
  oncancel={() => {
    deleteTarget = null;
    deleteMessage = '';
    deleteBlocked = false;
  }}
/>

<style>
  .page-content {
    padding: 28px;
    max-width: 1460px;
    margin: 0 auto;
  }

  .filters-wrap {
    margin-bottom: 12px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    font-weight: 800;
    cursor: pointer;
  }

  .btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .syncing {
    display: inline-flex;
    gap: 0.45rem;
    align-items: center;
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.82rem;
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.45rem 0.65rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 12%);
  }

  .spin {
    display: inline-flex;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .table-wrap {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.72rem;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-hover), transparent 15%);
    color: var(--text-secondary);
  }

  .pill.ok {
    border-color: rgba(34, 197, 94, 0.28);
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
  }

  .pill.warn {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.12);
    color: rgba(245, 158, 11, 0.95);
  }

  .empty {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-secondary);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--bg-hover);
  }

  .icon-btn.danger {
    color: #ef4444;
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 16px;
    }
  }
</style>
