<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { can } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';

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
    router_present: boolean;
    last_sync_at?: string | null;
  };

  let loadingRouters = $state(true);
  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');

  let loading = $state(false);
  let rows = $state<PppProfileRow[]>([]);

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
  ]);

  const tableData = $derived.by(() =>
    rows.map((r, idx) => ({
      id: r.id || `${r.name}:${idx}`,
      name: r.name,
      local: r.local_address || '—',
      remote: r.remote_address || '—',
      rate: r.rate_limit || '—',
      dns: r.dns_server || '—',
      state: Boolean(r.router_present),
      synced: r.last_sync_at,
      comment: r.comment,
    })),
  );

  onMount(async () => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }
    await loadRouters();
  });

  async function loadRouters() {
    loadingRouters = true;
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      if (!routerId && routers.length) routerId = routers[0].id;
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
      rows = (await api.mikrotik.routers.pppProfiles(routerId)) as any;
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
      toast.success($t('admin.network.routers.ppp_profiles.toasts.synced') || 'Synced PPP profiles');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
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
          {:else if key === 'synced'}
            {#if item.synced}
              <span title={formatDateTime(item.synced, { timeZone: $appSettings.app_timezone })}>
                {timeAgo(item.synced)}
              </span>
            {:else}
              <span class="muted">—</span>
            {/if}
          {:else}
            {item[key] ?? ''}
          {/if}
        {/snippet}
      </Table>
    </div>
  {/if}
</div>

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
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.2);
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
    border-radius: 16px;
    border: 1px solid var(--border-color);
    background: var(--bg-card);
    color: var(--text-secondary);
  }

  @media (max-width: 900px) {
    .page-content {
      padding: 16px;
    }
  }
</style>
