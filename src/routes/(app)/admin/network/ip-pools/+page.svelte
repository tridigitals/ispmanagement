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
  import IpPoolFormDialog, { type IpPoolFormModel } from '$lib/components/network/IpPoolFormDialog.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import {
    getIpPoolCrudGateState,
    getIpPoolDeleteState,
    getIpPoolMutationErrorState,
    isIpPoolStaleTargetConflict,
  } from '$lib/utils/ipPoolCrud';

  type RouterRow = {
    id: string;
    name: string;
    host: string;
    port: number;
    is_online: boolean;
  };

  type IpPoolRow = {
    id: string;
    name: string;
    ranges?: string | null;
    next_pool?: string | null;
    comment?: string | null;
    router_present: boolean;
    last_sync_at?: string | null;
  };

  let loadingRouters = $state(true);
  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');

  let loading = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let rows = $state<IpPoolRow[]>([]);
  let showForm = $state(false);
  let editing = $state<IpPoolRow | null>(null);
  let form = $state<IpPoolFormModel>({
    name: '',
    ranges: '',
    next_pool: '',
    comment: '',
  });
  let showDelete = $state(false);
  let deleteTarget = $state<IpPoolRow | null>(null);
  let deleteMessage = $state('');
  let deleteKeyword = $state('');
  let deleteDialogType = $state<'danger' | 'warning'>('danger');
  let deleteWarningCount = $state(0);

  const columns = $derived([
    { key: 'name', label: $t('admin.network.routers.ip_pools.columns.name') || 'Name' },
    { key: 'ranges', label: $t('admin.network.routers.ip_pools.columns.ranges') || 'Ranges', class: 'mono' },
    { key: 'next', label: $t('admin.network.routers.ip_pools.columns.next') || 'Next pool', class: 'mono', width: '170px' },
    { key: 'state', label: $t('admin.network.routers.ip_pools.columns.state') || 'State', width: '120px' },
    { key: 'synced', label: $t('admin.network.routers.ip_pools.columns.synced') || 'Synced', class: 'mono', width: '130px' },
    { key: 'actions', label: $t('common.actions') || 'Actions', width: '120px' },
  ]);

  const tableData = $derived.by(() =>
    rows.map((r, idx) => ({
      id: r.id || `${r.name}:${idx}`,
      name: r.name,
      ranges: r.ranges || '—',
      next: r.next_pool || '—',
      state: Boolean(r.router_present),
      synced: r.last_sync_at,
      comment: r.comment,
      raw: r,
    })),
  );

  const nextPoolOptions = $derived.by(() => {
    const currentName = editing?.name || form.name;
    return rows
      .map((row) => row.name?.trim())
      .filter((name): name is string => Boolean(name))
      .filter((name, index, names) => names.indexOf(name) === index)
      .filter((name) => name !== currentName)
      .sort((left, right) => left.localeCompare(right));
  });

  onMount(async () => {
    if (!$can('read', 'ip_pools') && !$can('manage', 'ip_pools')) {
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
      rows = (await api.mikrotik.routers.ipPools(routerId)) as any;
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
      rows = (await api.mikrotik.routers.syncIpPools(routerId)) as any;
      toast.success($t('admin.network.routers.ip_pools.toasts.synced') || 'Synced IP pools');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  function resetForm() {
    form = {
      name: '',
      ranges: '',
      next_pool: '',
      comment: '',
    };
  }

  function openCreate() {
    const gate = getIpPoolCrudGateState(routerId);
    if (gate.blocked) {
      toast.error($t('admin.network.routers.ip_pools.form.router_required') || 'Select a router first');
      return;
    }
    editing = null;
    resetForm();
    showForm = true;
  }

  function openEdit(row: IpPoolRow) {
    const gate = getIpPoolCrudGateState(routerId);
    if (gate.blocked) {
      toast.error($t('admin.network.routers.ip_pools.form.router_required') || 'Select a router first');
      return;
    }
    editing = row;
    form = {
      name: row.name || '',
      ranges: row.ranges || '',
      next_pool: row.next_pool || '',
      comment: row.comment || '',
    };
    showForm = true;
  }

  function normalizedPayload() {
    const normalize = (value: string) => {
      const trimmed = value.trim();
      return trimmed ? trimmed : null;
    };
    return {
      name: form.name.trim(),
      ranges: normalize(form.ranges),
      next_pool: normalize(form.next_pool),
      comment: normalize(form.comment),
    };
  }

  async function save() {
    if (!routerId) {
      toast.error($t('admin.network.routers.ip_pools.form.router_required') || 'Select a router first');
      return;
    }
    saving = true;
    try {
      const payload = normalizedPayload();
      if (!payload.name && !editing) {
        throw new Error(
          ($t('admin.network.routers.ip_pools.form.name_required') as string) || 'Pool name is required',
        );
      }
      if (editing) {
        await api.mikrotik.routers.updateIpPool(routerId, editing.id, payload);
        toast.success(($t('admin.network.routers.ip_pools.toasts.updated') as string) || 'IP pool updated');
      } else {
        await api.mikrotik.routers.createIpPool(routerId, payload as any);
        toast.success(($t('admin.network.routers.ip_pools.toasts.created') as string) || 'IP pool created');
      }
      showForm = false;
      editing = null;
      await load();
    } catch (error: any) {
      const state = getIpPoolMutationErrorState(
        String(error?.message || '').includes('mirror refresh failed') ? 'mirror_sync_failed' : 'router_write_failed',
      );
      if (state.tone === 'warning' && typeof toast.warning === 'function') {
        toast.warning(error?.message || state.message);
      } else {
        toast.error(error?.message || state.message);
      }
    } finally {
      saving = false;
    }
  }

  async function openDelete(row: IpPoolRow) {
    if (!routerId) {
      toast.error($t('admin.network.routers.ip_pools.form.router_required') || 'Select a router first');
      return;
    }
    try {
      const dependency = await api.mikrotik.routers.ipPoolDependencies(routerId, row.id);
      const counts = Object.fromEntries((dependency.dependencies || []).map((item) => [item.type, item.count]));
      const state = getIpPoolDeleteState(counts as any);
      deleteTarget = row;
      deleteDialogType = state.warning ? 'warning' : 'danger';
      deleteWarningCount = state.totalDependencies;
      deleteKeyword = row.name;
      deleteMessage = state.warning
        ? $t('admin.network.routers.ip_pools.delete.warning', {
            values: { name: row.name, count: state.totalDependencies },
          }) || `Delete ${row.name} from the router? ${state.totalDependencies} internal record(s) still reference this pool.`
        : $t('admin.network.routers.ip_pools.delete.confirm', {
            values: { name: row.name },
          }) || `Delete IP pool ${row.name} from the router?`;
      showDelete = true;
    } catch (error: any) {
      toast.error(error?.message || error);
    }
  }

  async function confirmDelete() {
    if (!routerId || !deleteTarget) return;
    deleting = true;
    try {
      const result = await api.mikrotik.routers.deleteIpPool(routerId, deleteTarget.id);
      showDelete = false;
      if ((result.warnings || []).some((item) => Number(item.count || 0) > 0) && typeof toast.warning === 'function') {
        toast.warning(
          ($t('admin.network.routers.ip_pools.toasts.deleted_with_warning', {
            values: { name: deleteTarget.name, count: deleteWarningCount },
          }) as string) || `IP pool ${deleteTarget.name} was deleted with ${deleteWarningCount} warning reference(s).`,
        );
      } else {
        toast.success(($t('admin.network.routers.ip_pools.toasts.deleted') as string) || 'IP pool deleted');
      }
      deleteTarget = null;
      deleteMessage = '';
      deleteKeyword = '';
      deleteWarningCount = 0;
      await load();
    } catch (error: any) {
      const message = error?.message || String(error || '');
      if (isIpPoolStaleTargetConflict(message)) {
        try {
          rows = (await api.mikrotik.routers.syncIpPools(routerId)) as any;
          showDelete = false;
          deleteTarget = null;
          deleteMessage = '';
          deleteKeyword = '';
          deleteWarningCount = 0;
          if (typeof toast.warning === 'function') {
            toast.warning(
              ($t('admin.network.routers.ip_pools.toasts.stale_deleted_sync') as string) ||
                'This IP pool was already missing on the router. The list has been refreshed.',
            );
          } else {
            toast.success(
              ($t('admin.network.routers.ip_pools.toasts.stale_deleted_sync') as string) ||
                'This IP pool was already missing on the router. The list has been refreshed.',
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
    title={$t('admin.network.routers.ip_pools.title')}
    subtitle={$t('network.ip_pool.title')}
  >
    {#snippet actions()}
      {#if loading}
        <span class="syncing"><span class="spin"><Icon name="refresh-cw" size={14} /></span>{$t('common.loading')}</span>
      {:else}
        <span class="syncing">{$t('common.updated')}</span>
      {/if}
      <button class="btn ghost" type="button" onclick={() => void load()} disabled={!routerId || loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh')}
      </button>
      <button class="btn ghost" type="button" onclick={() => void sync()} disabled={!routerId || loading}>
        <Icon name="download" size={16} />
        {$t('admin.network.routers.ip_pools.actions.sync')}
      </button>
      {#if $can('manage', 'ip_pools')}
        <button class="btn ghost" type="button" onclick={openCreate} disabled={!routerId || loading}>
          <Icon name="plus" size={16} />
          {$t('admin.network.routers.ip_pools.actions.add')}
        </button>
      {/if}
    {/snippet}
  </NetworkPageHeader>

  <div class="filters-wrap">
    <NetworkFilterPanel>
      <div class="control">
        <label for="ip-pools-router">{$t('admin.customers.pppoe.fields.router')}</label>
        <select
          id="ip-pools-router"
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
      <span>{$t('common.select')} router…</span>
    </div>
  {:else if tableData.length === 0 && !loading}
    <div class="empty">
      <Icon name="info" size={18} />
      <span>{$t('admin.network.routers.ip_pools.empty')}</span>
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
        searchPlaceholder={$t('admin.network.routers.ip_pools.search')}
        mobileView="card"
      >
        {#snippet cell({ item, key }: any)}
          {#if key === 'state'}
            {#if item.state}
              <span class="pill ok">{$t('admin.network.routers.ip_pools.state.present')}</span>
            {:else}
              <span class="pill warn">{$t('admin.network.routers.ip_pools.state.missing')}</span>
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
              {#if $can('manage', 'ip_pools')}
                <button class="icon-btn" type="button" onclick={() => openEdit(item.raw)} title={$t('admin.network.routers.ip_pools.actions.edit')}>
                  <Icon name="edit" size={16} />
                </button>
                <button class="icon-btn danger" type="button" onclick={() => void openDelete(item.raw)} title={$t('admin.network.routers.ip_pools.actions.delete')}>
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

<IpPoolFormDialog
  bind:show={showForm}
  loading={saving}
  isEditing={Boolean(editing)}
  bind:pool={form}
  nextPoolOptions={nextPoolOptions}
  onSubmit={() => void save()}
/>

<ConfirmDialog
  bind:show={showDelete}
  title={$t('admin.network.routers.ip_pools.delete.title')}
  message={deleteMessage}
  confirmText={deleteDialogType === 'warning'
    ? $t('admin.network.routers.ip_pools.delete.confirm_warning') || 'Delete with warning'
    : $t('common.delete') || 'Delete'}
  confirmationKeyword={deleteKeyword}
  type={deleteDialogType}
  loading={deleting}
  onconfirm={() => void confirmDelete()}
  oncancel={() => {
    deleteTarget = null;
    deleteMessage = '';
    deleteKeyword = '';
    deleteDialogType = 'danger';
    deleteWarningCount = 0;
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
