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
  <section class="hero">
    <div class="hero-bg"></div>
    <div class="hero-inner">
      <div class="hgroup">
        <div class="kicker">
          <span class="dot"></span>
          {$t('sidebar.sections.network') || 'Network'}
        </div>
        <h1 class="h1">{$t('admin.network.routers.ppp_profiles.title') || 'PPP Profiles'}</h1>
        <div class="sub">
          {$t('admin.network.routers.ppp_profiles.subtitle') ||
            'Per-router RouterOS PPP profiles (synced into database for mapping & import).'}
        </div>
      </div>

      <div class="hero-actions">
        <label class="router-filter">
          <span class="label">{$t('admin.customers.pppoe.fields.router') || 'Router'}</span>
          <select
            class="select"
            bind:value={routerId}
            disabled={loadingRouters}
            onchange={() => void load()}
          >
            <option value="">{($t('common.select') || 'Select') + '...'}</option>
            {#each routers as r}
              <option value={r.id}>
                {r.name}
              </option>
            {/each}
          </select>
        </label>

        <div class="hero-buttons">
          {#if loading}
            <span class="syncing"><span class="spin"><Icon name="refresh-cw" size={14} /></span>{$t('common.loading') || 'Loading...'}</span>
          {:else}
            <span class="syncing">{$t('common.updated') || 'Updated'}</span>
          {/if}
          <button class="btn btn-secondary" type="button" onclick={load} disabled={!routerId || loading}>
            <Icon name="refresh-cw" size={16} />
            {$t('common.refresh') || 'Refresh'}
          </button>
          <button class="btn btn-secondary" type="button" onclick={sync} disabled={!routerId || loading}>
            <Icon name="download" size={16} />
            {$t('admin.network.routers.ppp_profiles.actions.sync') || 'Sync from router'}
          </button>
        </div>
      </div>
    </div>
  </section>

  <div class="card table-card">

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
</div>

<style>
  .hero {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 22px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.02);
    box-shadow: var(--shadow-md);
    margin-bottom: 1rem;
  }

  .hero-bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(900px 220px at 10% 0%, rgba(99, 102, 241, 0.28), transparent 60%),
      radial-gradient(900px 220px at 85% 30%, rgba(16, 185, 129, 0.12), transparent 60%),
      radial-gradient(900px 220px at 50% 110%, rgba(245, 158, 11, 0.14), transparent 60%),
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0.01));
  }

  :global([data-theme='light']) .hero-bg {
    background:
      radial-gradient(900px 220px at 10% 0%, rgba(99, 102, 241, 0.18), transparent 60%),
      radial-gradient(900px 220px at 85% 30%, rgba(16, 185, 129, 0.08), transparent 60%),
      radial-gradient(900px 220px at 50% 110%, rgba(245, 158, 11, 0.1), transparent 60%),
      linear-gradient(180deg, rgba(0, 0, 0, 0.02), rgba(0, 0, 0, 0.01));
  }

  .hero-inner {
    position: relative;
    padding: 1.15rem 1.2rem 1.2rem;
    display: grid;
    gap: 1rem;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 850;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 0.78rem;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: rgba(99, 102, 241, 0.9);
    box-shadow: 0 0 0 6px rgba(99, 102, 241, 0.12);
  }

  .h1 {
    margin-top: 0.25rem;
    color: var(--text-primary);
    font-size: clamp(1.55rem, 2.2vw, 2rem);
    font-weight: 1000;
    letter-spacing: 0.01em;
    line-height: 1.12;
  }

  .sub {
    margin-top: 0.25rem;
    color: var(--text-secondary);
    font-weight: 650;
    max-width: 70ch;
  }

  .hero-actions {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .hero-buttons {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .router-filter {
    display: flex;
    gap: 0.45rem;
    align-items: center;
    flex-wrap: wrap;
    min-width: min(460px, 100%);
  }

  .label {
    font-size: 12px;
    opacity: 0.75;
    font-weight: 800;
  }

  .select {
    padding: 0.55rem 0.7rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    outline: none;
    min-width: min(260px, 100%);
  }

  .select option {
    background: var(--bg-surface);
    color: var(--text-primary);
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

  .table-card {
    padding: 0.85rem;
  }

  .table-wrap {
    overflow: auto;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card), transparent 8%);
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
    .hero-buttons {
      width: 100%;
    }

    .hero-buttons :global(.btn) {
      width: 100%;
      justify-content: center;
    }

    .router-filter {
      width: 100%;
    }

    .select {
      min-width: 100%;
    }
  }
</style>
