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
    PageHeader,
    RowActions,
    StatTile,
  } from '$lib/components/ds';
  import type { Column } from '$lib/components/ds/table-types';

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
    const occupancy = getNetworkAssetPortOccupancySummary(item, rows);
    if (occupancy.length > 0) return occupancy;
    const detailSummary = getNetworkAssetDetailSummary(item);
    const coordinateSummary = formatNetworkAssetCoordinates(item.latitude, item.longitude);
    return coordinateSummary ? [...detailSummary, `Peta ${coordinateSummary}`] : detailSummary;
  }

  const columns: Column[] = [
    { key: 'name', label: 'Aset' },
    { key: 'asset_type', label: 'Tipe' },
    { key: 'status', label: 'Status' },
    { key: 'serial_number', label: 'Serial' },
    { key: 'customer_name', label: 'Relasi' },
    { key: 'location_label', label: 'Topologi' },
    { key: 'updated_at', label: 'Diperbarui' },
    { key: 'actions', label: '' },
  ];

  const typeOptions = $derived([
    { value: 'all', label: 'Semua tipe' },
    ...NETWORK_ASSET_TYPE_GROUPS.flatMap((g) => g.types.map((t) => ({ value: t, label: getNetworkAssetTypeLabel(t) }))),
  ]);
  const statusOptions = [
    { value: 'all', label: 'Semua status' },
    { value: 'available', label: 'Tersedia' },
    { value: 'reserved', label: 'Dipesan' },
    { value: 'installed', label: 'Terpasang' },
    { value: 'faulty', label: 'Rusak' },
    { value: 'retired', label: 'Pensiun' },
  ];

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
      const result = await api.networkAssets.list({ page: 1, per_page: 500 });
      rows = result.data || [];
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
      if (parsed.error === 'pair') throw new Error('Latitude dan longitude harus diisi bersamaan.');
      if (parsed.error === 'invalid') throw new Error('Latitude dan longitude harus angka yang valid.');
      if (parsed.error === 'latitude_range') throw new Error('Latitude harus di antara -90 dan 90.');
      if (parsed.error === 'longitude_range') throw new Error('Longitude harus di antara -180 dan 180.');
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
      showModal = false;
      await load();
      toast.success(editing ? 'Aset diperbarui.' : 'Aset dibuat.');
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
      toast.success('Aset dihapus.');
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    }
  }

  function openOnMap(row: NetworkAssetListItem) {
    if (row.latitude == null || row.longitude == null) {
      toast.error('Aset belum punya koordinat peta.');
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
<AppShell title="Aset FTTH">
  <PageHeader
    title="Aset FTTH"
    eyebrow="Jaringan"
    desc="Registri perangkat lapangan — ONT, OLT, ODP, kabel, dan tiang."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void load()} disabled={loading}>
        Segarkan
      </Button>
      {#if canManage}
        <Button variant="primary" onclick={openCreate}>Aset baru</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
    <StatTile label="Total aset" value={String(stats.total)} hint={`${stats.installed} terpasang`} />
    <StatTile label="Terpasang" value={String(stats.installed)} hint={`dari ${stats.total} aset`} tone="positive" />
    <StatTile label="Tersedia" value={String(stats.available)} hint="siap dipasang" />
    <StatTile label="Rusak" value={String(stats.faulty)} hint="perlu tindak lanjut" tone="negative" />
  </div>

  <Card title="Filter">
    <div class="grid gap-3 sm:grid-cols-3">
      <Field id="as-q" label="Cari" type="text" stacked value={q} onchange={(v) => (q = v)} placeholder="Cari nama/kode/serial…" />
      <Field id="as-type" label="Tipe" type="select" stacked value={assetType} options={typeOptions} onchange={(v) => (assetType = v)} />
      <Field id="as-status" label="Status" type="select" stacked value={status} options={statusOptions} onchange={(v) => (status = v)} />
    </div>
  </Card>

  <Card title={`Daftar aset (${filteredRows.length})`} padded={false}>
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
          <Badge tone={assetStatusTone(item.status)} label={getNetworkAssetStatusLabel(item.status)} />
        {:else if column.key === 'serial_number'}
          <span class="font-mono text-xs">{item.serial_number || '—'}</span>
        {:else if column.key === 'customer_name'}
          <span class="text-sm text-ink-700">{buildNetworkAssetRelationText(item)}</span>
        {:else if column.key === 'location_label'}
          <span class="text-sm text-ink-700">{buildNetworkAssetTopologyText(item, rows)}</span>
        {:else if column.key === 'updated_at'}
          <span class="font-mono text-xs text-ink-500">{item.updated_at}</span>
        {:else if column.key === 'actions'}
          {#if canManage}
            <RowActions
              primary={{ label: 'Sunting', icon: 'cog', onclick: () => void openEdit(item) }}
              rest={[
                { label: 'Lihat di peta', icon: 'pin', disabled: item.latitude == null || item.longitude == null, disabledReason: 'Aset belum punya koordinat', onclick: () => openOnMap(item) },
                { label: 'Hapus', icon: 'close', danger: true, onclick: () => remove(item) },
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
  title="Hapus aset?"
  message={`Aset ${deleteTarget?.name || ''} yang dihapus tidak bisa dikembalikan.`}
  confirmText="Hapus"
  cancelText="Batal"
  type="danger"
  onconfirm={() => void handleConfirmDelete()}
  oncancel={() => { deleteTarget = null; }}
/>
