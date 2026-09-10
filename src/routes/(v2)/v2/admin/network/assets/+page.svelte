<script lang="ts">
  /*
    Aset FTTH v2 — gelombang 24b.

    Versi lama: (app)/admin/network/assets/+page.svelte (684 baris) +
    modul colocated (state, connections, coordinates, map-navigation,
    form modal lazy). Logika bisnis dipakai ulang langsung dari modul
    yang sama (bukan duplikasi); yang diganti hanya chrome:
    AppShell + PageHeader + StatTile + Card + Field + DataTable ds +
    RowActions. Form modal diimpor langsung (Vite code-split per route).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { api, type NetworkAssetListItem } from '$lib/api/client';
  import { fetchAllPages } from '$lib/utils/fetchAllPages';
  import { can, tenant, user } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import {
    NETWORK_ASSET_TYPE_GROUPS,
    getDefaultNetworkAssetStatus,
    getNetworkAssetGroupLabel,
    getNetworkAssetStatusLabel,
    getNetworkAssetTypeLabel,
  } from '$lib/utils/networkAssetTypes';
  import {
    buildNetworkAssetMetadata,
    createNetworkAssetDetailDraft,
    getNetworkAssetDetailSummary,
    type NetworkAssetDetailDraft,
    validateNetworkAssetDetailDraft,
  } from '$lib/utils/networkAssetDetails';
  import { getNetworkAssetPortOccupancySummary } from '$lib/utils/networkAssetOccupancy';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import NetworkAssetFormModal from '../../../../../(app)/admin/network/assets/NetworkAssetFormModal.svelte';
  import {
    buildNetworkAssetRelationText,
    buildNetworkAssetSavePayload,
    buildNetworkAssetStats,
    buildNetworkAssetTopologyText,
    filterNetworkAssets,
  } from '../../../../../(app)/admin/network/assets/networkAssetsPageState';
  import { buildNetworkAssetConnectionItems } from '../../../../../(app)/admin/network/assets/networkAssetConnections';
  import {
    formatNetworkAssetCoordinates,
    parseNetworkAssetCoordinates,
  } from '../../../../../(app)/admin/network/assets/networkAssetCoordinates';
  import { buildNetworkAssetMapUrl } from '../../../../../(app)/admin/network/assets/networkAssetMapNavigation';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    Icon,
    PageHeader,
    RowActions,
    StatTile,
  } from '$lib/components/ds';
  import type { Column } from '$lib/components/ds/table-types';
  import { t } from 'svelte-i18n';

  type AssetDraft = {
    asset_type: string;
    name: string;
    code: string;
    vendor: string;
    model: string;
    serial_number: string;
    status: string;
    latitude: string;
    longitude: string;
    notes: string;
  };

  let loading = $state(true);
  let saving = $state(false);
  let showModal = $state(false);
  let rows = $state<NetworkAssetListItem[]>([]);
  let assetsTruncated = $state(false);
  let detailDraft = $state<NetworkAssetDetailDraft>({});
  let q = $state('');
  let assetType = $state('all');
  let status = $state('all');
  let showDeleteConfirm = $state(false);
  let deleteTarget = $state<NetworkAssetListItem | null>(null);
  let editing = $state<NetworkAssetListItem | null>(null);
  let draft = $state<AssetDraft>(emptyDraft());

  const editingConnectionItems = $derived.by(() =>
    editing ? buildNetworkAssetConnectionItems(editing, rows) : [],
  );
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

  function assetStatusTone(s: string): 'positive' | 'warning' | 'negative' | 'neutral' | 'info' {
    if (s === 'installed') return 'positive';
    if (s === 'available' || s === 'reserved') return 'info';
    if (s === 'faulty') return 'negative';
    return 'neutral';
  }

  function getAssetSummary(item: NetworkAssetListItem): string[] {
    const occupancy = getNetworkAssetPortOccupancySummary(item, rows, undefined, (k) => $t(k));
    if (occupancy.length > 0) return occupancy;
    const detailSummary = getNetworkAssetDetailSummary(item);
    const coordinateSummary = formatNetworkAssetCoordinates(item.latitude, item.longitude);
    return coordinateSummary ? [...detailSummary, `Peta ${coordinateSummary}`] : detailSummary;
  }

  const columns = $derived<Column[]>([
    { key: 'name', label: $t('admin.network.assets.v2.col_asset') },
    { key: 'asset_type', label: $t('admin.network.assets.v2.col_type') },
    { key: 'status', label: $t('admin.network.assets.v2.col_status') },
    { key: 'serial_number', label: $t('admin.network.assets.v2.col_serial') },
    { key: 'customer_name', label: $t('admin.network.assets.v2.col_relation') },
    { key: 'location_label', label: $t('admin.network.assets.v2.col_topology') },
    { key: 'updated_at', label: $t('admin.network.assets.v2.col_updated') },
    { key: 'actions', label: '' },
  ]);

  const typeOptions = $derived([
    { value: 'all', label: $t('admin.network.assets.v2.all_types') },
    ...NETWORK_ASSET_TYPE_GROUPS.flatMap((g) => g.types.map((t) => ({ value: t, label: getNetworkAssetTypeLabel(t) }))),
  ]);
  const statusOptions = $derived([
    { value: 'all', label: $t('admin.network.assets.v2.all_status') },
    { value: 'available', label: $t('admin.network.assets.v2.st_available') },
    { value: 'reserved', label: $t('admin.network.assets.v2.st_reserved') },
    { value: 'installed', label: $t('admin.network.assets.v2.st_installed') },
    { value: 'faulty', label: $t('admin.network.assets.v2.st_faulty') },
    { value: 'retired', label: $t('admin.network.assets.v2.st_retired') },
  ]);

  const canRead = $derived($can('read', 'ftth_assets') || $can('manage', 'ftth_assets'));
  const canManage = $derived($can('manage', 'ftth_assets'));

  onMount(async () => {
    if (!canRead) {
      goto('/unauthorized');
      return;
    }
    await load();
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
      latitude: '',
      longitude: '',
      notes: '',
    };
  }

  async function load() {
    loading = true;
    try {
      // A-03: sebelumnya sekali tarik per_page:500 — aset ke-501 dan
      // seterusnya hilang tanpa sinyal. Sekarang loop halaman (cap 10 x 500)
      // dan `assetsTruncated` menandai jujur kalau cap-nya tersentuh.
      const all = await fetchAllPages((page, per_page) =>
        api.networkAssets.list({ page, per_page }),
      );
      rows = all.rows;
      assetsTruncated = !all.complete;
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
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

  async function save() {
    saving = true;
    try {
      const detailErrors = validateNetworkAssetDetailDraft(draft.asset_type, detailDraft);
      if (detailErrors.length > 0) throw new Error(detailErrors[0]);
      const parsed = parseNetworkAssetCoordinates(draft.latitude, draft.longitude);
      if (parsed.error === 'pair') throw new Error($t('admin.network.assets.v2.e_pair'));
      if (parsed.error === 'invalid') throw new Error($t('admin.network.assets.v2.e_invalid'));
      if (parsed.error === 'latitude_range') throw new Error($t('admin.network.assets.v2.e_lat'));
      if (parsed.error === 'longitude_range') throw new Error($t('admin.network.assets.v2.e_lng'));
      const payload = buildNetworkAssetSavePayload({
        draft: {
          ...draft,
          latitude: parsed.latitude != null ? String(parsed.latitude) : '',
          longitude: parsed.longitude != null ? String(parsed.longitude) : '',
        },
        metadata: buildNetworkAssetMetadata(draft.asset_type, detailDraft, editing?.metadata || {}),
        existingRelations: editing
          ? {
              customer_id: editing.customer_id,
              location_id: editing.location_id,
              work_order_id: editing.work_order_id,
              parent_asset_id: editing.parent_asset_id,
            }
          : undefined,
      });
      if (editing) await api.networkAssets.update(editing.id, payload);
      else await api.networkAssets.create(payload);
      const wasEdit = Boolean(editing);
      showModal = false;
      editing = null;
      draft = emptyDraft();
      detailDraft = createNetworkAssetDetailDraft(draft.asset_type, {});
      await load();
      toast.success(wasEdit ? $t('admin.network.assets.v2.t_updated') : $t('admin.network.assets.v2.t_created'));
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      saving = false;
    }
  }

  function remove(row: NetworkAssetListItem) {
    deleteTarget = row;
    showDeleteConfirm = true;
  }

  async function handleConfirmDelete() {
    if (!deleteTarget) return;
    const row = deleteTarget;
    deleteTarget = null;
    try {
      await api.networkAssets.delete(row.id);
      rows = rows.filter((item) => item.id !== row.id);
      toast.success($t('admin.network.assets.v2.t_deleted'));
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    }
  }

  function openOnMap(row: NetworkAssetListItem) {
    if (row.latitude == null || row.longitude == null) {
      toast.error($t('admin.network.assets.v2.t_nocoord'));
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
<AppShell title={ $t('admin.network.assets.v2.title') }>
  <PageHeader
    title={ $t('admin.network.assets.v2.title') }
    eyebrow={ $t('admin.eyebrows.network') }
    desc={ $t('admin.network.assets.v2.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void load()} disabled={loading}>
        { $t('common.refresh') }
      </Button>
      {#if canManage}
        <Button variant="primary" onclick={openCreate}>{ $t('admin.network.assets.v2.new_btn') }</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
    <StatTile label={ $t('admin.network.assets.v2.st_total') } value={String(stats.total)} hint={ $t('admin.network.assets.v2.h_installed', { values: { n: stats.installed } }) } />
    <StatTile label={ $t('admin.network.assets.v2.st_installed') } value={String(stats.installed)} hint={ $t('admin.network.assets.v2.h_from_total', { values: { n: stats.total } }) } tone="positive" />
    <StatTile label={ $t('admin.network.assets.v2.st_available') } value={String(stats.available)} hint={ $t('admin.network.assets.v2.h_ready') } />
    <StatTile label={ $t('admin.network.assets.v2.st_faulty') } value={String(stats.faulty)} hint={ $t('admin.network.assets.v2.h_followup') } tone="negative" />
  </div>

  <Card title={ $t('admin.network.assets.v2.f_filter') }>
    <div class="grid gap-3 sm:grid-cols-3">
      <Field id="as-q" label={ $t('common.search') } type="text" stacked value={q} onchange={(v) => (q = v)} placeholder={ $t('admin.network.assets.v2.search_ph') } />
      <Field id="as-type" label={ $t('admin.network.assets.v2.col_type') } type="select" stacked value={assetType} options={typeOptions} onchange={(v) => (assetType = v)} />
      <Field id="as-status" label={ $t('admin.network.assets.v2.col_status') } type="select" stacked value={status} options={statusOptions} onchange={(v) => (status = v)} />
    </div>
  </Card>

  {#if assetsTruncated}
    <div
      role="alert"
      class="flex items-start gap-2 rounded-xl border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-900"
    >
      <Icon name="alert" size={16} class="mt-0.5 shrink-0 text-amber-700" />
      <span>
        Menampilkan 5.000 aset teratas — masih ada aset di luar batas ini. Sempitkan
        filter tipe/status atau buka lewat halaman pelanggan untuk data lengkap.
      </span>
    </div>
  {/if}

  <Card title={`Daftar aset (${filteredRows.length}${assetsTruncated ? '+' : ''})`} padded={false}>
    <DataTable
      {columns}
      rows={filteredRows}
      {loading}
      emptyTitle="Belum ada aset"
      emptyHint="Tambahkan aset pertama lewat tombol Aset baru."
    >
      {#snippet cell(item, column)}
        {#if column.key === 'name'}
          <div>
            <div class="text-sm font-semibold text-ink-900">{item.name}</div>
            {#if item.code}<div class="font-mono text-xs text-ink-400">{item.code}</div>{/if}
            {#if getAssetSummary(item).length > 0}
              <div class="mt-0.5 text-xs text-ink-500">{getAssetSummary(item).join(' • ')}</div>
            {/if}
          </div>
        {:else if column.key === 'asset_type'}
          <div>
            <div class="text-sm text-ink-800">{getNetworkAssetTypeLabel(item.asset_type)}</div>
            <div class="text-xs text-ink-400">{getNetworkAssetGroupLabel(item.asset_group)}</div>
          </div>
        {:else if column.key === 'status'}
          <Badge tone={assetStatusTone(item.status)} label={getNetworkAssetStatusLabel(item.status, (k) => $t(k))} />
        {:else if column.key === 'serial_number'}
          <span class="font-mono text-xs">{item.serial_number || '—'}</span>
        {:else if column.key === 'customer_name'}
          <span class="text-sm text-ink-700">{buildNetworkAssetRelationText(item, (k) => $t(k))}</span>
        {:else if column.key === 'location_label'}
          <span class="text-sm text-ink-700">{buildNetworkAssetTopologyText(item, rows)}</span>
        {:else if column.key === 'updated_at'}
          <span class="font-mono text-xs text-ink-500">{item.updated_at}</span>
        {:else if column.key === 'actions'}
          {#if canManage}
            <RowActions
              primary={{ label: $t('admin.network.assets.v2.a_edit'), icon: 'cog', onclick: () => void openEdit(item) }}
              rest={[
                { label: $t('admin.network.assets.v2.a_map'), icon: 'pin', disabled: item.latitude == null || item.longitude == null, disabledReason: 'Aset belum punya koordinat', onclick: () => openOnMap(item) },
                { label: $t('common.delete'), icon: 'close', danger: true, onclick: () => remove(item) },
              ]}
            />
          {/if}
        {/if}
      {/snippet}
    </DataTable>
  </Card>
</AppShell>

{#if showModal}
  <NetworkAssetFormModal
    bind:show={showModal}
    {saving}
    {editing}
    connectedItems={editingConnectionItems}
    {draft}
    {detailDraft}
    onassettypechange={handleAssetTypeChange}
    onclose={() => (showModal = false)}
    onsave={() => void save()}
  />
{/if}

<ConfirmDialog
  bind:show={showDeleteConfirm}
  title={ $t('admin.network.assets.v2.del_q') }
  message={ $t('admin.network.assets.v2.del_msg', { values: { n: deleteTarget?.name || '' } }) }
  confirmText="Hapus"
  cancelText="Batal"
  type="danger"
  onconfirm={() => void handleConfirmDelete()}
  oncancel={() => { deleteTarget = null; }}
/>
