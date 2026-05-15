<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { t } from 'svelte-i18n';
  import {
    api,
    type NetworkAssetListItem,
  } from '$lib/api/client';
  import { can, tenant, user } from '$lib/stores/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import {
    buildNetworkAssetStats,
    buildNetworkAssetSavePayload,
    filterNetworkAssets,
  } from './networkAssetsPageState';
  import {
    formatNetworkAssetCoordinates,
    parseNetworkAssetCoordinates,
  } from './networkAssetCoordinates';
  import {
    NETWORK_ASSET_TYPES,
    getDefaultNetworkAssetStatus,
    getNetworkAssetGroupLabel,
    getNetworkAssetStatusLabel,
    getNetworkAssetTypeLabel,
  } from '$lib/utils/networkAssetTypes';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import {
    buildNetworkAssetMetadata,
    createNetworkAssetDetailDraft,
    getNetworkAssetDetailSummary,
    type NetworkAssetDetailDraft,
    validateNetworkAssetDetailDraft,
  } from '$lib/utils/networkAssetDetails';
  import { getNetworkAssetPortOccupancySummary } from '$lib/utils/networkAssetOccupancy';
  import { loadNetworkAssetFormModal } from './networkAssetsPageModules';
  import { buildNetworkAssetMapUrl } from './networkAssetMapNavigation';

  type DeferredComponent = any;
  type AssetDraft = {
    asset_type: string;
    name: string;
    code: string;
    vendor: string;
    model: string;
    serial_number: string;
    status: string;
    customer_id: string;
    location_id: string;
    work_order_id: string;
    parent_asset_id: string;
    latitude: string;
    longitude: string;
    notes: string;
  };

  let loading = $state(true);
  let saving = $state(false);
  let showModal = $state(false);
  let FormModalComponent = $state<DeferredComponent | null>(null);
  let rows = $state<NetworkAssetListItem[]>([]);
  let detailDraft = $state<NetworkAssetDetailDraft>({});
  let q = $state('');
  let assetType = $state('all');
  let status = $state('all');
  let editing = $state<NetworkAssetListItem | null>(null);
  let draft = $state<AssetDraft>(emptyDraft());

  const stats = $derived.by(() => buildNetworkAssetStats(rows));
  const filteredRows = $derived.by(() => filterNetworkAssets(rows, { q, assetType, status }));
  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  function getAssetSummary(item: NetworkAssetListItem): string[] {
    const occupancy = getNetworkAssetPortOccupancySummary(item, rows);
    if (occupancy.length > 0) return occupancy;
    const detailSummary = getNetworkAssetDetailSummary(item);
    const coordinateSummary = formatNetworkAssetCoordinates(item.latitude, item.longitude);
    return coordinateSummary ? [...detailSummary, `Map: ${coordinateSummary}`] : detailSummary;
  }
  const columns = $derived.by(() => [
    { key: 'name', label: $t('admin.ftth_assets.table.asset') || 'Asset' },
    { key: 'asset_type', label: $t('admin.ftth_assets.table.type') || 'Type' },
    { key: 'status', label: $t('admin.ftth_assets.table.status') || 'Status' },
    { key: 'serial_number', label: $t('admin.ftth_assets.table.serial') || 'Serial' },
    { key: 'customer_name', label: $t('admin.ftth_assets.table.customer') || 'Customer' },
    { key: 'location_label', label: $t('admin.ftth_assets.table.location') || 'Location' },
    { key: 'updated_at', label: $t('admin.ftth_assets.table.updated') || 'Updated' },
    { key: 'actions', label: '', align: 'right' as const },
  ]);

  onMount(async () => {
    if (!$can('read', 'ftth_assets') && !$can('manage', 'ftth_assets')) {
      goto('/unauthorized');
      return;
    }
    await Promise.all([load(), ensureModalLoaded()]);
  });

  function emptyDraft(): AssetDraft {
    return {
      asset_type: 'ont',
      name: '',
      code: '',
      vendor: '',
      model: '',
      serial_number: '',
      status: getDefaultNetworkAssetStatus(),
      customer_id: '',
      location_id: '',
      work_order_id: '',
      parent_asset_id: '',
      latitude: '',
      longitude: '',
      notes: '',
    };
  }

  async function ensureModalLoaded() {
    if (FormModalComponent) return;
    const module = await loadNetworkAssetFormModal();
    FormModalComponent = module.NetworkAssetFormModalComponent;
  }

  async function load() {
    loading = true;
    try {
      const result = await api.networkAssets.list({ page: 1, per_page: 500 });
      rows = result.data || [];
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load FTTH assets');
    } finally {
      loading = false;
    }
  }

  function openCreate() {
    draft = emptyDraft();
    detailDraft = createNetworkAssetDetailDraft(draft.asset_type, {});
    editing = null;
    showModal = true;
  }

  async function openEdit(row: NetworkAssetListItem) {
    editing = row;
    draft = {
      asset_type: row.asset_type,
      name: row.name,
      code: row.code || '',
      vendor: row.vendor || '',
      model: row.model || '',
      serial_number: row.serial_number || '',
      status: row.status,
      customer_id: row.customer_id || '',
      location_id: row.location_id || '',
      work_order_id: row.work_order_id || '',
      parent_asset_id: row.parent_asset_id || '',
      latitude: row.latitude != null ? String(row.latitude) : '',
      longitude: row.longitude != null ? String(row.longitude) : '',
      notes: row.notes || '',
    };
    detailDraft = createNetworkAssetDetailDraft(row.asset_type, row.metadata || {});
    showModal = true;
  }

  function handleAssetTypeChange(value: string) {
    draft.asset_type = value;
    detailDraft = createNetworkAssetDetailDraft(value, editing?.metadata || {});
  }

  function clearAssetCoordinates() {
    draft.latitude = '';
    draft.longitude = '';
  }

  async function save() {
    saving = true;
    try {
      const detailErrors = validateNetworkAssetDetailDraft(draft.asset_type, detailDraft);
      if (detailErrors.length > 0) {
        throw new Error(detailErrors[0]);
      }
      const parsedCoordinates = parseNetworkAssetCoordinates(draft.latitude, draft.longitude);
      if (parsedCoordinates.error === 'pair') {
        throw new Error('Latitude and longitude must be filled together.');
      }
      if (parsedCoordinates.error === 'invalid') {
        throw new Error('Latitude and longitude must be valid numbers.');
      }
      if (parsedCoordinates.error === 'latitude_range') {
        throw new Error('Latitude must be between -90 and 90.');
      }
      if (parsedCoordinates.error === 'longitude_range') {
        throw new Error('Longitude must be between -180 and 180.');
      }

      const payload = buildNetworkAssetSavePayload({
        draft: {
          ...draft,
          latitude: parsedCoordinates.latitude != null ? String(parsedCoordinates.latitude) : '',
          longitude: parsedCoordinates.longitude != null ? String(parsedCoordinates.longitude) : '',
        },
        metadata: buildNetworkAssetMetadata(
          draft.asset_type,
          detailDraft,
          editing?.metadata || {},
        ),
        existingRelations: editing
          ? {
              customer_id: editing.customer_id,
              location_id: editing.location_id,
              work_order_id: editing.work_order_id,
              parent_asset_id: editing.parent_asset_id,
            }
          : undefined,
      });

      if (editing) {
        await api.networkAssets.update(editing.id, payload);
      } else {
        await api.networkAssets.create(payload);
      }

      showModal = false;
      await load();
      toast.success(editing ? 'FTTH asset updated' : 'FTTH asset created');
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save FTTH asset');
    } finally {
      saving = false;
    }
  }

  async function remove(row: NetworkAssetListItem) {
    if (!confirm(`Delete asset "${row.name}"?`)) return;
    try {
      await api.networkAssets.delete(row.id);
      rows = rows.filter((item) => item.id !== row.id);
      toast.success('FTTH asset deleted');
    } catch (e: any) {
      toast.error(e?.message || 'Failed to delete FTTH asset');
    }
  }

  function openOnMap(row: NetworkAssetListItem) {
    if (row.latitude == null || row.longitude == null) {
      toast.error('Asset does not have map coordinates yet');
      return;
    }

    void goto(
      buildNetworkAssetMapUrl({
        tenantPrefix,
        assetId: row.id,
        latitude: Number(row.latitude),
        longitude: Number(row.longitude),
      }),
    );
  }
</script>

<div class="page-content fade-in">
  <div class="head">
    <div>
      <h1>{$t('sidebar.ftth_assets') || 'FTTH Assets'}</h1>
      <p class="sub">
        {$t('admin.ftth_assets.page.subtitle') ||
          'Manage tenant FTTH assets such as OLT, ODP, splitter, ONT, and related devices.'}
      </p>
    </div>
    <div class="head-actions">
      <button class="btn ghost" type="button" onclick={load}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      {#if $can('manage', 'ftth_assets')}
        <button class="btn" type="button" onclick={openCreate}>
          <Icon name="plus" size={16} />
          {$t('admin.ftth_assets.actions.new_asset') || 'New asset'}
        </button>
      {/if}
    </div>
  </div>

  <div class="stats">
    <StatsCard title={$t('admin.ftth_assets.stats.total') || 'Total'} value={stats.total} icon="box" color="primary" />
    <StatsCard
      title={$t('admin.ftth_assets.stats.installed') || 'Installed'}
      value={stats.installed}
      icon="plug"
      color="success"
    />
    <StatsCard
      title={$t('admin.ftth_assets.stats.available') || 'Available'}
      value={stats.available}
      icon="check-circle"
      color="primary"
    />
    <StatsCard
      title={$t('admin.ftth_assets.stats.faulty') || 'Faulty'}
      value={stats.faulty}
      icon="alert-triangle"
      color="danger"
    />
  </div>

  <section class="registry-shell">
    <div class="filter-shell">
      <div class="filter-shell__head">
        <div>
          <span class="filter-kicker">Filter Registry</span>
          <strong>Find and narrow FTTH assets quickly</strong>
        </div>
        <span class="filter-count">{filteredRows.length} asset{filteredRows.length === 1 ? '' : 's'}</span>
      </div>

      <div class="toolbar">
        <input
          class="input"
          bind:value={q}
          placeholder={$t('admin.ftth_assets.filters.search_placeholder') ||
            'Search asset, code, or serial...'}
        />
        <select class="input" bind:value={assetType}>
          <option value="all">{$t('admin.ftth_assets.filters.all_types') || 'All types'}</option>
          {#each NETWORK_ASSET_TYPES as type}
            <option value={type}>{getNetworkAssetTypeLabel(type)}</option>
          {/each}
        </select>
        <select class="input" bind:value={status}>
          <option value="all">{$t('admin.ftth_assets.filters.all_statuses') || 'All statuses'}</option>
          <option value="available">{$t('admin.ftth_assets.status.available') || 'Available'}</option>
          <option value="reserved">{$t('admin.ftth_assets.status.reserved') || 'Reserved'}</option>
          <option value="installed">{$t('admin.ftth_assets.status.installed') || 'Installed'}</option>
          <option value="faulty">{$t('admin.ftth_assets.status.faulty') || 'Faulty'}</option>
          <option value="retired">{$t('admin.ftth_assets.status.retired') || 'Retired'}</option>
        </select>
      </div>
    </div>

    <div class="table-shell">
      <Table
        {columns}
        data={filteredRows}
        {loading}
        emptyText={$t('admin.ftth_assets.table.empty') || 'No FTTH assets yet.'}
        pagination={false}
      >
        {#snippet cell({ item, key }: any)}
          {#if key === 'name'}
            <div class="asset-cell">
              <strong>{item.name}</strong>
              {#if item.code}
                <span class="muted mono">{item.code}</span>
              {/if}
              {#if getAssetSummary(item).length > 0}
                <span class="asset-detail-summary">
                  {getAssetSummary(item).join(' • ')}
                </span>
              {/if}
            </div>
          {:else if key === 'asset_type'}
            <div class="asset-type-cell">
              <span class="asset-type-chip">{getNetworkAssetTypeLabel(item.asset_type)}</span>
              <span class="asset-group-label">{getNetworkAssetGroupLabel(item.asset_group)}</span>
            </div>
          {:else if key === 'status'}
            <span class={`asset-status-chip status-${item.status}`}>{getNetworkAssetStatusLabel(item.status)}</span>
          {:else if key === 'serial_number'}
            {item.serial_number || '—'}
          {:else if key === 'customer_name'}
            {item.customer_name || '—'}
          {:else if key === 'location_label'}
            {item.location_label || '—'}
          {:else if key === 'updated_at'}
            <span class="mono">{item.updated_at}</span>
          {:else if key === 'actions'}
            <div class="row-actions">
              {#if $can('manage', 'ftth_assets')}
                <button
                  class="btn-icon"
                  type="button"
                  title="Open on map"
                  onclick={() => openOnMap(item)}
                  disabled={item.latitude == null || item.longitude == null}
                >
                  <Icon name="map-pin" size={15} />
                </button>
                <button class="btn-icon" type="button" title="Edit" onclick={() => openEdit(item)}>
                  <Icon name="pencil" size={15} />
                </button>
                <button class="btn-icon danger" type="button" title="Delete" onclick={() => remove(item)}>
                  <Icon name="trash-2" size={15} />
                </button>
              {/if}
            </div>
          {/if}
        {/snippet}
      </Table>
    </div>
  </section>
</div>

{#if FormModalComponent}
  <FormModalComponent
    bind:show={showModal}
    {saving}
    {editing}
    {draft}
    {detailDraft}
    onclearcoordinates={clearAssetCoordinates}
    onassettypechange={handleAssetTypeChange}
    onclose={() => (showModal = false)}
    onsave={save}
  />
{/if}

<style>
  .page-content {
    display: grid;
    gap: 1rem;
    padding: clamp(12px, 2vw, 24px);
  }
  .head,
  .head-actions,
  .toolbar,
  .row-actions {
    display: flex;
    gap: 0.75rem;
  }
  .head {
    justify-content: space-between;
    align-items: flex-start;
  }
  .head-actions,
  .row-actions {
    align-items: center;
  }
  .sub,
  .muted {
    color: var(--text-secondary);
  }
  .asset-detail-summary {
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.35;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.9rem;
  }
  .stats :global(.stats-card) {
    border-radius: 18px;
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--bg-surface) 91%, #f3ead8 9%) 0%,
        color-mix(in srgb, var(--bg-surface) 97%, transparent) 100%
      );
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.08);
  }
  .registry-shell {
    display: grid;
    gap: 0;
    border: 1px solid var(--border-subtle, var(--border-color));
    border-radius: 20px;
    background: color-mix(in srgb, var(--bg-surface) 94%, transparent);
    overflow: hidden;
    box-shadow: 0 14px 44px rgba(0, 0, 0, 0.12);
  }
  .filter-shell {
    display: grid;
    gap: 0.9rem;
    padding: 1rem 1rem 0.95rem;
    border-bottom: 1px solid var(--border-subtle, var(--border-color));
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 92%, #f1e7d7 8%) 0%, var(--bg-surface) 100%);
  }
  .filter-shell__head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.8rem;
  }
  .filter-kicker {
    display: inline-block;
    margin-bottom: 0.25rem;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .filter-count {
    display: inline-flex;
    align-items: center;
    min-height: 2rem;
    padding: 0.18rem 0.7rem;
    border-radius: 999px;
    border: 1px solid var(--border-subtle, var(--border-color));
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
    color: var(--text-secondary);
    font-size: 0.82rem;
    white-space: nowrap;
  }
  .toolbar {
    display: grid;
    grid-template-columns: minmax(320px, 1.7fr) minmax(180px, 0.7fr) minmax(180px, 0.7fr);
    gap: 0.75rem;
    align-items: stretch;
  }
  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.68rem 0.8rem;
    outline: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease, background 0.15s ease;
  }
  .input:focus {
    border-color: color-mix(in srgb, var(--color-primary, #6366f1) 55%, var(--border-color));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary, #6366f1) 18%, transparent);
  }
  select.input {
    appearance: auto;
  }
  .input::placeholder {
    color: var(--text-secondary);
  }
  .toolbar .input {
    min-height: 44px;
    background: color-mix(in srgb, var(--bg-surface) 88%, #ffffff 12%);
  }
  .table-shell {
    padding: 0.28rem 0.28rem 0.4rem;
  }
  .table-shell :global(.table-container) {
    border-radius: 16px;
    overflow: hidden;
  }
  .table-shell :global(.responsive-table thead th) {
    padding: 0.9rem 1rem;
    font-size: 0.76rem;
    letter-spacing: 0.06em;
    background: color-mix(in srgb, var(--bg-surface) 96%, #f5ecdb 4%);
  }
  .table-shell :global(.responsive-table tbody td) {
    padding: 0.92rem 1rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border-subtle, var(--border-color)) 84%, transparent);
  }
  .table-shell :global(.responsive-table tbody tr) {
    transition: background 0.14s ease;
  }
  .table-shell :global(.responsive-table tbody tr:hover) {
    background: color-mix(in srgb, var(--bg-surface) 84%, #fbf4e6 16%);
  }
  .table-shell :global(.empty-state) {
    min-height: 170px;
  }
  .asset-cell {
    display: grid;
    gap: 0.2rem;
  }
  .asset-type-chip,
  .asset-status-chip {
    display: inline-flex;
    align-items: center;
    min-height: 1.8rem;
    padding: 0.18rem 0.62rem;
    border-radius: 999px;
    border: 1px solid var(--border-color, rgba(255, 255, 255, 0.08));
    font-size: 0.78rem;
    line-height: 1;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }
  .asset-type-chip {
    background: color-mix(in srgb, var(--bg-surface, #111827) 88%, var(--text-primary, #fff) 12%);
    color: var(--text-secondary);
  }
  .asset-type-cell {
    display: grid;
    gap: 0.28rem;
  }
  .asset-group-label {
    font-size: 0.72rem;
    color: var(--text-secondary);
    line-height: 1.1;
  }
  .asset-status-chip {
    background: color-mix(in srgb, var(--bg-surface, #111827) 90%, var(--text-primary, #fff) 10%);
    color: var(--text-primary);
  }
  .asset-status-chip.status-available {
    border-color: color-mix(in srgb, #2f855a 35%, var(--border-color, rgba(255, 255, 255, 0.08)));
    color: color-mix(in srgb, #2f855a 82%, var(--text-primary, #fff));
  }
  .asset-status-chip.status-installed {
    border-color: color-mix(in srgb, #2563eb 35%, var(--border-color, rgba(255, 255, 255, 0.08)));
    color: color-mix(in srgb, #2563eb 78%, var(--text-primary, #fff));
  }
  .asset-status-chip.status-reserved {
    border-color: color-mix(in srgb, #b7791f 35%, var(--border-color, rgba(255, 255, 255, 0.08)));
    color: color-mix(in srgb, #b7791f 78%, var(--text-primary, #fff));
  }
  .asset-status-chip.status-faulty,
  .asset-status-chip.status-retired {
    border-color: color-mix(in srgb, #c53030 30%, var(--border-color, rgba(255, 255, 255, 0.08)));
    color: color-mix(in srgb, #c53030 78%, var(--text-primary, #fff));
  }
  .mono {
    font-family: var(--font-mono, monospace);
  }
  .pill {
    display: inline-flex;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-soft);
  }
  .btn-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: transparent;
  }
  .btn-icon.danger {
    color: var(--color-danger, #ef4444);
  }
  @media (max-width: 860px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .head {
      flex-direction: column;
    }
    .filter-shell__head {
      flex-direction: column;
    }
    .toolbar {
      grid-template-columns: 1fr;
    }
  }
</style>
