<script lang="ts">
  /*
    Instalasi (work order) v2 — gelombang 20.

    Versi lama: `(app)/admin/network/installations/+page.svelte` (2.988 baris).

    Temuan yang dikunci gelombang ini (backend sudah dipatch):
    1. Reopen tidak punya guard admin/owner padahal cancel punya — reopen
       menghidupkan kembali siklus hidup langganan. FE meng-gate tombolnya
       dengan isAdminOwner, tapi API bisa dipanggil langsung teknisi.
       Kini backend menolak 403.
    2. Complete mengubah status ke 'completed' SEBELUM bind aset; kalau
       bind gagal (race aset direbut WO lain) WO tercatat selesai tanpa
       ONT terikat ke pelanggan. Kini bind dulu, baru status — gagal bind
       = WO tetap in_progress dan bisa di-retry.

    Yang TIDAK dimigrasikan ke halaman ini (sengaja, sudah punya halaman v2
    sendiri): provisioning PPPoE/DHCP saat instalasi dan kabel designer.
    Detail WO menampilkan tautan ke halaman terkait.
  */
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { can, user as authUser, token } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { api } from '$lib/api/client';
  import type {
    AuditLog,
    InstallationWorkOrderView,
    NetworkAssetListItem,
    TeamMember,
    WorkOrderRescheduleRequestView,
  } from '$lib/api/client';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';
  import { formatDateTime } from '$lib/utils/date';
  import { toast } from '$lib/stores/toast';
  import {
    buildPersistedNotes,
    friendlyWorkOrderError,
    parseChecklistState,
    parsePhotoIds,
    stripGeneratedSections,
    woStatusLabel,
    type InstallationChecklistState,
  } from '$lib/utils/installationNotes';
  import {
    buildInstallationParentAssetOptions,
    buildInstallationTerminalAssetOptions,
    resolveInstallationAssetBinding,
    validateInstallationAssetBinding,
  } from '../../../../../(app)/admin/network/installations/installationAssetBinding';
  import {
    buildInstallationStats,
    filterAndSortInstallationRows,
    type InstallationAssignmentFilter,
    type InstallationSortKey,
  } from '../../../../../(app)/admin/network/installations/installationTableState';
  type TileTone = 'neutral' | 'positive' | 'negative' | 'warning';
  import { buildDefaultInstallationCancelReason } from '../../../../../(app)/admin/network/installations/cancelReason';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import AttentionPanel from '$lib/components/ds/AttentionPanel.svelte';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
  import type { Column } from '$lib/components/ds/table-types';
  import type { StatusTone } from '$lib/components/ds/tokens';

  const VISIBILITY_KEY = 'installation_work_order_visibility_mode';
  const CANCEL_REASON_MIN = 10;

  const canManage = $derived($can('manage', 'work_orders'));
  const canReadAudit = $derived($can('read', 'audit_logs'));
  const currentUserId = $derived(($authUser?.id || '').trim());
  const isAdminOwner = $derived.by(() => {
    const u = $authUser as { role?: string; tenant_role?: string; is_super_admin?: boolean } | null;
    if (!u) return false;
    const g = `${u.role || ''}`.trim().toLowerCase();
    const t = `${u.tenant_role || ''}`.trim().toLowerCase();
    return u.is_super_admin === true || g === 'owner' || g === 'admin' || t === 'owner' || t === 'admin';
  });

  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let rows = $state<InstallationWorkOrderView[]>([]);
  let assignees = $state<TeamMember[]>([]);
  let assets = $state<NetworkAssetListItem[]>([]);

  let statusFilter = $state<'all' | 'pending' | 'in_progress' | 'completed' | 'cancelled'>('all');
  let assignmentFilter = $state<InstallationAssignmentFilter>('all');
  let assigneeFilter = $state('');
  let search = $state('');
  let sortKey = $state<InstallationSortKey>('updated_at');
  let sortDir = $state<'asc' | 'desc'>('desc');
  let includeClosed = $state(true);

  const stats = $derived(buildInstallationStats(rows));
  const visible = $derived(
    filterAndSortInstallationRows(rows, {
      statusFilter,
      assignmentFilter,
      assigneeUserId: assigneeFilter,
      search,
      sortKey,
      sortDirection: sortDir,
    }),
  );

  const overdue = $derived(
    rows.filter(
      (r) =>
        r.status === 'pending' &&
        r.scheduled_at &&
        new Date(r.scheduled_at).getTime() < Date.now() - 24 * 3600 * 1000,
    ),
  );
  const attentionItems = $derived<AttentionItem[]>(
    overdue.length
      ? [
          {
            icon: 'clock',
            title: `${overdue.length} instalasi lewat jadwal`,
            detail: 'Jadwalnya sudah lewat lebih dari sehari dan WO masih menunggu teknisi.',
            action: 'Lihat penunggu',
            href: '/v2/admin/network/installations',
          },
        ]
      : [],
  );

  // ---- modal detail ----
  let showDetail = $state(false);
  let active = $state<InstallationWorkOrderView | null>(null);
  let busyId = $state<string | null>(null);
  let detailSeq = 0;

  let formAssignee = $state('');
  let formSchedule = $state('');
  let formNotes = $state('');
  let checklist = $state<InstallationChecklistState>({ cable: false, ont: false, pppoe: false, speed: false });
  let photoIds = $state<string[]>([]);
  let uploadingPhotos = $state(false);

  let terminalAssetId = $state('');
  let parentAssetId = $state('');
  let timeline = $state<AuditLog[]>([]);
  let timelineLoading = $state(false);
  let reschedule = $state<WorkOrderRescheduleRequestView | null>(null);
  let rescheduleNotes = $state('');
  let rescheduleOverride = $state('');
  let decisionBusy = $state(false);
  let detailError = $state<string | null>(null);

  // ---- modal cancel ----
  let showCancel = $state(false);
  let cancelTarget = $state<InstallationWorkOrderView | null>(null);
  let cancelReason = $state('');

  // ---- modal visibilitas ----
  let showVisibility = $state(false);
  let visibilityMode = $state<'admin_only' | 'all_staff'>('admin_only');
  let visibilityBusy = $state(false);

  onMount(() => {
    void (async () => {
      try {
        const [wo, assigneeList, assetList, vis] = await Promise.all([
          api.workOrders.list({ include_closed: true, limit: 500 }),
          canManage ? api.workOrders.assignees().catch(() => [] as TeamMember[]) : Promise.resolve([] as TeamMember[]),
          api.networkAssets.list({ page: 1, per_page: 500 }).catch(() => ({ data: [] as NetworkAssetListItem[] })),
          api.settings.getValue(VISIBILITY_KEY).catch(() => null),
        ]);
        rows = wo || [];
        assignees = assigneeList || [];
        assets = assetList.data || [];
        visibilityMode = vis === 'all_staff' ? 'all_staff' : 'admin_only';
        const target =
          $page.url.searchParams.get('work_order_id') || $page.url.searchParams.get('workOrderId');
        if (target) {
          const found = rows.find((r) => r.id === target);
          if (found) openDetail(found);
        }
      } catch (e) {
        loadError = friendlyWorkOrderError(extractApiErrorMessage(e));
      } finally {
        loading = false;
      }
    })();
  });

  async function reload() {
    loading = true;
    loadError = null;
    try {
      rows = await api.workOrders.list({ include_closed: includeClosed, limit: 500 });
      if (active) {
        const refreshed = rows.find((r) => r.id === active?.id);
        active = refreshed ?? null;
        if (!refreshed) closeDetail();
      }
    } catch (e) {
      loadError = friendlyWorkOrderError(extractApiErrorMessage(e));
    } finally {
      loading = false;
    }
  }

  function statusTone(st: string): StatusTone {
    if (st === 'in_progress') return 'info';
    if (st === 'completed') return 'positive';
    if (st === 'cancelled') return 'neutral';
    return 'warning';
  }

  function assigneeLabel(id: string | null): string {
    if (!id) return 'Belum ada';
    const a = assignees.find((x) => x.user_id === id);
    return a?.name || rows.find((r) => r.assigned_to === id)?.assigned_to_name || id.slice(0, 8);
  }

  function isMine(r: InstallationWorkOrderView): boolean {
    return (r.assigned_to || '').trim() === currentUserId;
  }
  function canOperate(r: InstallationWorkOrderView): boolean {
    return isAdminOwner || isMine(r);
  }
  function canTake(r: InstallationWorkOrderView): boolean {
    return canManage && r.status === 'pending' && !(r.assigned_to || '').trim();
  }
  function canRelease(r: InstallationWorkOrderView): boolean {
    return canManage && isAdminOwner && r.status === 'pending' && !!(r.assigned_to || '').trim();
  }

  // ================= detail =================

  function openDetail(row: InstallationWorkOrderView) {
    detailSeq += 1;
    active = row;
    showDetail = true;
    detailError = null;
    formAssignee = row.assigned_to || '';
    formSchedule = row.scheduled_at ? toLocalInput(row.scheduled_at) : '';
    formNotes = stripGeneratedSections(row.notes);
    checklist = parseChecklistState(row.notes);
    photoIds = parsePhotoIds(row.notes);
    const binding = resolveInstallationAssetBinding(assets, row.id);
    terminalAssetId = binding.terminal_asset_id;
    parentAssetId = binding.parent_asset_id;
    reschedule = null;
    rescheduleNotes = '';
    rescheduleOverride = '';
    void loadTimeline(row.id);
    void loadReschedule(row.id);
  }

  function closeDetail() {
    showDetail = false;
    active = null;
    timeline = [];
  }

  async function loadTimeline(id: string) {
    if (!canReadAudit) return;
    const seq = detailSeq;
    timelineLoading = true;
    try {
      const res = await api.audit.listTenant(1, 30, {
        resource: 'installation_work_orders',
        resource_id: id,
      });
      if (seq !== detailSeq) return;
      timeline = (res?.data || []).filter((l) => `${l.action || ''}`.toUpperCase().startsWith('WORK_ORDER_'));
    } catch {
      if (seq === detailSeq) timeline = [];
    } finally {
      if (seq === detailSeq) timelineLoading = false;
    }
  }

  async function loadReschedule(id: string) {
    if (!canManage) return;
    const seq = detailSeq;
    try {
      const req = await api.workOrders.getRescheduleRequest(id);
      if (seq !== detailSeq) return;
      reschedule = req;
      rescheduleOverride = req ? toLocalInput(req.requested_schedule_at) : '';
    } catch {
      if (seq === detailSeq) reschedule = null;
    }
  }

  function toLocalInput(raw: string): string {
    const d = new Date(raw);
    if (!Number.isFinite(d.getTime())) return '';
    const pad = (n: number) => `${n}`.padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function persistedNotes(): string {
    const photoLines = photoIds.map((id) => `- foto: ${getApiBaseUrl()}/storage/files/${id}/content`);
    return buildPersistedNotes(formNotes, checklist, photoLines);
  }

  async function mutate(fn: () => Promise<unknown>, okMsg: string) {
    if (!active || busyId) return false;
    busyId = active.id;
    detailError = null;
    try {
      await fn();
      toast.success(okMsg);
      await reload();
      return true;
    } catch (e) {
      const msg = friendlyWorkOrderError(extractApiErrorMessage(e));
      detailError = msg;
      toast.error(msg);
      return false;
    } finally {
      busyId = null;
    }
  }

  async function savePlan() {
    if (!active) return;
    const scheduled = formSchedule ? new Date(formSchedule).toISOString() : undefined;
    await mutate(
      () =>
        api.workOrders.assign(active!.id, {
          assigned_to: formAssignee,
          scheduled_at: scheduled,
          notes: persistedNotes(),
        }),
      'Rencana tersimpan.',
    );
  }

  async function saveNotesOnly() {
    if (!active) return;
    await mutate(
      () =>
        api.workOrders.assign(active!.id, {
          assigned_to: formAssignee || currentUserId,
          notes: persistedNotes(),
        }),
      'Catatan tersimpan.',
    );
  }

  async function startWo() {
    if (!active) return;
    if (!formAssignee || !formSchedule) {
      detailError = 'Isi teknisi dan jadwal sebelum memulai.';
      return;
    }
    await mutate(() => api.workOrders.start(active!.id, persistedNotes()), 'Pengerjaan dimulai.');
  }

  async function completeWo() {
    if (!active) return;
    if (!terminalAssetId) {
      detailError = 'Pilih aset terminal (ONT/ONU) sebelum menyelesaikan.';
      return;
    }
    await mutate(
      () =>
        api.workOrders.complete(active!.id, {
          notes: persistedNotes(),
          terminal_asset_id: terminalAssetId,
          parent_asset_id: parentAssetId || null,
        }),
      'Instalasi selesai.',
    );
  }

  async function claimWo(row: InstallationWorkOrderView) {
    busyId = row.id;
    try {
      await api.workOrders.claim(row.id);
      toast.success('WO diambil.');
      await reload();
    } catch (e) {
      toast.error(friendlyWorkOrderError(extractApiErrorMessage(e)));
    } finally {
      busyId = null;
    }
  }

  async function releaseWo(row?: InstallationWorkOrderView) {
    const target = row ?? active;
    if (!target) return;
    if (!row) return void (await mutate(() => api.workOrders.release(target.id), 'Penugasan dilepas.'));
    // dari daftar: tanpa modal, pakai busyId lokal
    busyId = target.id;
    try {
      await api.workOrders.release(target.id);
      toast.success('Penugasan dilepas.');
      await reload();
    } catch (e) {
      toast.error(friendlyWorkOrderError(extractApiErrorMessage(e)));
    } finally {
      busyId = null;
    }
  }

  function openCancel(row: InstallationWorkOrderView) {
    cancelTarget = row;
    cancelReason = buildDefaultInstallationCancelReason();
    showCancel = true;
  }

  async function confirmCancel() {
    if (!cancelTarget) return;
    if (cancelReason.trim().length < CANCEL_REASON_MIN) {
      toast.error(`Alasan pembatalan minimal ${CANCEL_REASON_MIN} karakter.`);
      return;
    }
    showCancel = false;
    await mutate(() => api.workOrders.cancel(cancelTarget!.id, cancelReason), 'WO dibatalkan.');
  }

  async function reopenWo() {
    if (!active) return;
    await mutate(() => api.workOrders.reopen(active!.id), 'WO dibuka ulang.');
  }

  async function decideReschedule(kind: 'approve' | 'reject') {
    if (!active || !reschedule) return;
    if (kind === 'reject' && !rescheduleNotes.trim()) {
      detailError = 'Alasan penolakan wajib diisi.';
      return;
    }
    decisionBusy = true;
    detailError = null;
    try {
      const payload =
        kind === 'approve'
          ? { scheduled_at: rescheduleOverride ? new Date(rescheduleOverride).toISOString() : undefined, notes: rescheduleNotes || undefined }
          : { notes: rescheduleNotes };
      if (kind === 'approve') await api.workOrders.approveReschedule(active.id, payload);
      else await api.workOrders.rejectReschedule(active.id, { notes: rescheduleNotes });
      toast.success(kind === 'approve' ? 'Jadwal ulang disetujui.' : 'Permintaan ditolak.');
      reschedule = null;
      await reload();
    } catch (e) {
      detailError = friendlyWorkOrderError(extractApiErrorMessage(e));
    } finally {
      decisionBusy = false;
    }
  }

  async function onUploadPhotos(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const files = Array.from(input.files || []).filter((f) => f.type.startsWith('image/'));
    if (!files.length) return;
    uploadingPhotos = true;
    try {
      for (const f of files) {
        const up = await api.storage.uploadFile(f);
        if (!photoIds.includes(up.id)) photoIds = [...photoIds, up.id];
      }
      toast.success('Foto terunggah.');
    } catch (e) {
      toast.error(friendlyWorkOrderError(extractApiErrorMessage(e)));
    } finally {
      uploadingPhotos = false;
      input.value = '';
    }
  }

  async function saveVisibility() {
    visibilityBusy = true;
    try {
      await api.settings.upsert(
        VISIBILITY_KEY,
        visibilityMode,
        'Kontrol visibilitas work order instalasi baru: admin saja atau semua staf instalasi.',
      );
      toast.success('Visibilitas tersimpan.');
      showVisibility = false;
      await reload();
    } catch (e) {
      toast.error(friendlyWorkOrderError(extractApiErrorMessage(e)));
    } finally {
      visibilityBusy = false;
    }
  }

  const terminalOptions = $derived(active ? buildInstallationTerminalAssetOptions(assets, active, terminalAssetId) : []);
  const parentOptions = $derived(active ? buildInstallationParentAssetOptions(assets, parentAssetId) : []);
  const bindingError = $derived(active ? validateInstallationAssetBinding(active, { terminal_asset_id: terminalAssetId, parent_asset_id: parentAssetId }) : null);

  const columns: Column[] = [
    { key: 'customer', label: 'Pelanggan' },
    { key: 'status', label: 'Status', width: '120px' },
    { key: 'assignee', label: 'Teknisi', width: '150px' },
    { key: 'schedule', label: 'Jadwal', width: '150px' },
    { key: 'package', label: 'Paket', width: '160px' },
    { key: 'actions', label: '', width: '170px', align: 'right' },
  ];

  const tiles: Array<{ st: 'pending' | 'in_progress' | 'completed' | 'cancelled'; label: string; statKey: 'pending' | 'inProgress' | 'completed' | 'cancelled'; baseTone: TileTone }> = [
    { st: 'pending', label: 'Menunggu', statKey: 'pending', baseTone: 'warning' },
    { st: 'in_progress', label: 'Dikerjakan', statKey: 'inProgress', baseTone: 'neutral' },
    { st: 'completed', label: 'Selesai', statKey: 'completed', baseTone: 'positive' },
    { st: 'cancelled', label: 'Batal', statKey: 'cancelled', baseTone: 'neutral' },
  ];
</script>

<AppShell title="Instalasi">
  <PageHeader title="Instalasi" desc="Work order pemasangan pelanggan — rencana, pengerjaan, penyelesaian.">
    {#snippet actions()}
      {#if isAdminOwner}
        <Button variant="ghost" icon="cog" onclick={() => (showVisibility = true)}>Visibilitas</Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if loadError}
    <div class="mt-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700">
      {loadError}
      <button type="button" class="ml-2 underline" onclick={() => (loadError = null)}>Tutup</button>
    </div>
  {/if}

  <div class="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
    {#each tiles as t (t.st)}
      <button
        type="button"
        class="focus-ring rounded-xl text-left {statusFilter === t.st ? 'ring-2 ring-ink-900' : ''}"
        aria-pressed={statusFilter === t.st}
        onclick={() => (statusFilter = statusFilter === t.st ? 'all' : t.st)}
      >
        <StatTile label={t.label} value={String(stats[t.statKey])} hint="klik untuk filter" tone={t.baseTone} />
      </button>
    {/each}
  </div>

  {#if attentionItems.length}
    <div class="mt-4">
      <AttentionPanel items={attentionItems} />
    </div>
  {/if}

  <div class="mt-4 flex flex-wrap items-center gap-2">
    <label class="flex items-center gap-2 text-sm text-ink-700">
      <input type="checkbox" class="h-6 w-6 accent-ink-900" bind:checked={includeClosed} onchange={() => void reload()} />
      Sertakan yang tutup
    </label>
    <select class="focus-ring h-9 rounded-lg bg-white text-sm ring-1 ring-inset ring-ink-200" bind:value={assignmentFilter} aria-label="Filter penugasan">
      <option value="all">Semua penugasan</option>
      <option value="assigned">Sudah ada teknisi</option>
      <option value="unassigned">Belum ada teknisi</option>
    </select>
    {#if assignees.length}
      <select class="focus-ring h-9 rounded-lg bg-white text-sm ring-1 ring-inset ring-ink-200" bind:value={assigneeFilter} aria-label="Filter per teknisi">
        <option value="">Semua teknisi</option>
        {#each assignees as a (a.user_id)}
          <option value={a.user_id}>{a.name}</option>
        {/each}
      </select>
    {/if}
    <div class="relative ml-auto min-w-[220px]">
      <input
        bind:value={search}
        placeholder="Cari pelanggan, lokasi, paket"
        aria-label="Cari work order"
        class="focus-ring h-9 w-full rounded-lg border-0 bg-white px-3 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
      />
    </div>
  </div>

  <div class="mt-3">
    <DataTable
      {columns}
      rows={visible}
      {loading}
      emptyTitle="Tidak ada work order"
      emptyHint="Instalasi baru dari langganan akan muncul di sini otomatis."
      footNote={`${visible.length} dari ${rows.length} work order`}
    >
      {#snippet cell(row: InstallationWorkOrderView, col: Column)}
        {#if col.key === 'customer'}
          <div class="min-w-0 max-w-[320px]">
            <div class="truncate font-medium text-ink-900">{row.customer_name || row.customer_id.slice(0, 8)}</div>
            <div class="truncate text-sm text-ink-500">{row.location_label || 'Tanpa lokasi'}{#if row.router_name} · {row.router_name}{/if}</div>
          </div>
        {:else if col.key === 'status'}
          <Badge tone={statusTone(row.status)} label={woStatusLabel(row.status)} />
        {:else if col.key === 'assignee'}
          <span class="text-sm text-ink-700">{assigneeLabel(row.assigned_to)}</span>
        {:else if col.key === 'schedule'}
          <span class="text-sm {row.scheduled_at && new Date(row.scheduled_at).getTime() < Date.now() && row.status === 'pending' ? 'text-red-600' : 'text-ink-500'}">
            {row.scheduled_at ? formatDateTime(row.scheduled_at, { timeZone: $appSettings.app_timezone }) : 'Belum dijadwal'}
          </span>
        {:else if col.key === 'package'}
          <span class="text-sm text-ink-700">{row.package_name || '—'}</span>
        {:else if col.key === 'actions'}
          <RowActions
            primary={{ label: 'Detail', icon: 'search', onclick: () => openDetail(row) }}
            rest={[
              ...(canTake(row) ? [{ label: 'Ambil', onclick: () => void claimWo(row) }] : []),
              ...(canManage && row.status === 'pending' && row.assigned_to ? [{ label: 'Lepas', onclick: () => void releaseWo(row) }] : []),
              ...(isAdminOwner && canManage && row.status !== 'completed' && row.status !== 'cancelled'
                ? [{ label: 'Batalkan', onclick: () => openCancel(row) }]
                : []),
            ]}
          />
        {/if}
      {/snippet}
    </DataTable>
  </div>

  <Modal bind:show={showDetail} title={active ? `WO — ${active.customer_name || active.id.slice(0, 8)}` : 'Detail'}>
    {#if active}
      <div class="space-y-3 text-sm">
        <div class="flex flex-wrap items-center gap-2">
          <Badge tone={statusTone(active.status)} label={woStatusLabel(active.status)} />
          {#if active.has_customer_package_invoice}
            <Badge tone="positive" label="Invoice dibuat" />
          {/if}
          {#if active.selected_zone_name}
            <Badge tone="neutral" label="Zona: {active.selected_zone_name}" />
          {/if}
        </div>
        <dl class="grid grid-cols-2 gap-x-4 gap-y-1.5 text-ink-700">
          <dt class="text-ink-500">Lokasi</dt>
          <dd>{active.location_label || '—'}</dd>
          <dt class="text-ink-500">Paket</dt>
          <dd>{active.package_name || '—'} ({active.package_provisioning_type || '—'})</dd>
          <dt class="text-ink-500">Langganan</dt>
          <dd>{woStatusLabel(active.subscription_status || '') || '—'}{#if active.subscription_grace_until} · grace s/d {formatDateTime(active.subscription_grace_until, { timeZone: $appSettings.app_timezone })}{/if}</dd>
        </dl>

        {#if reschedule}
          <div class="rounded-lg border border-amber-200 bg-amber-50 p-3">
            <div class="font-medium text-amber-900">Permintaan jadwal ulang dari pelanggan</div>
            <div class="mt-1 text-amber-800">
              Diminta: {formatDateTime(reschedule.requested_schedule_at, { timeZone: $appSettings.app_timezone })}
              {#if reschedule.reason} · Alasan: {reschedule.reason}{/if}
            </div>
            {#if canManage}
              <div class="mt-2 flex flex-wrap items-end gap-2">
                <label class="flex flex-col gap-1">
                  <span class="text-xs text-amber-900">Jadwal final</span>
                  <input type="datetime-local" class="focus-ring h-9 rounded-lg bg-white px-2 ring-1 ring-inset ring-amber-300" bind:value={rescheduleOverride} />
                </label>
                <label class="flex min-w-[160px] flex-1 flex-col gap-1">
                  <span class="text-xs text-amber-900">Catatan keputusan</span>
                  <input type="text" class="focus-ring h-9 rounded-lg bg-white px-2 ring-1 ring-inset ring-amber-300" bind:value={rescheduleNotes} placeholder="opsional untuk setuju" />
                </label>
                <Button variant="primary" size="sm" disabled={decisionBusy} onclick={() => void decideReschedule('approve')}>Setujui</Button>
                <Button variant="ghost" size="sm" disabled={decisionBusy} onclick={() => void decideReschedule('reject')}>Tolak</Button>
              </div>
            {/if}
          </div>
        {/if}

        {#if canOperate(active) && active.status !== 'completed' && active.status !== 'cancelled'}
          <div class="rounded-lg border border-ink-200 p-3">
            <div class="mb-1 font-medium text-ink-900">Rencana</div>
            <Field
              id="wo-assignee"
              label="Teknisi"
              type="select"
              stacked
              value={formAssignee}
              options={[{ value: '', label: 'Belum ada' }, ...assignees.map((a) => ({ value: a.user_id, label: a.name }))]}
              onchange={(v) => (formAssignee = String(v ?? ''))}
              disabled={!isAdminOwner}
            />
            <label class="mt-1 flex flex-col gap-1">
              <span class="text-base font-medium text-ink-800">Jadwal</span>
              <input id="wo-schedule" type="datetime-local" class="focus-ring h-9 rounded-lg bg-white px-3 text-base ring-1 ring-inset ring-ink-200" bind:value={formSchedule} />
            </label>
            <div class="mt-2 flex flex-wrap gap-2">
              {#if isAdminOwner || isMine(active)}
                <Button variant="ghost" size="sm" disabled={!!busyId} onclick={() => void savePlan()}>Simpan rencana</Button>
              {/if}
              {#if active.status === 'pending' && formAssignee && formSchedule}
                <Button variant="primary" size="sm" disabled={!!busyId} onclick={() => void startWo()}>Mulai</Button>
              {/if}
              {#if active.status === 'in_progress'}
                <Button variant="primary" size="sm" disabled={!!busyId || !terminalAssetId} onclick={() => void completeWo()}>Selesaikan</Button>
              {/if}
              {#if canRelease(active)}
                <Button variant="ghost" size="sm" disabled={!!busyId} onclick={() => void releaseWo()}>Lepas penugasan</Button>
              {/if}
            </div>
          </div>
        {:else if active.status === 'cancelled' && isAdminOwner && canManage}
          <div class="flex gap-2">
            <Button variant="ghost" size="sm" disabled={!!busyId} onclick={() => void reopenWo()}>Buka ulang</Button>
          </div>
        {/if}

        {#if active.status === 'in_progress'}
          <div class="rounded-lg border border-ink-200 p-3">
            <div class="mb-1 font-medium text-ink-900">Aset instalasi</div>
            <Field
              id="wo-terminal"
              label="Aset terminal (ONT/ONU)"
              type="select"
              stacked
              value={terminalAssetId}
              options={[{ value: '', label: 'Pilih aset…' }, ...terminalOptions]}
              onchange={(v) => (terminalAssetId = String(v ?? ''))}
              error={bindingError}
            />
            <Field
              id="wo-parent"
              label="Aset induk (opsional)"
              type="select"
              stacked
              value={parentAssetId}
              options={[{ value: '', label: 'Tidak ada' }, ...parentOptions]}
              onchange={(v) => (parentAssetId = String(v ?? ''))}
            />
          </div>
        {/if}

        {#if canOperate(active) && active.status !== 'completed' && active.status !== 'cancelled'}
          <div class="rounded-lg border border-ink-200 p-3">
            <div class="mb-1 font-medium text-ink-900">Checklist lapangan</div>
            <div class="grid grid-cols-2 gap-1">
              {#each [['cable', 'Kabel terpasang'], ['ont', 'ONT terpasang'], ['pppoe', 'PPPoE dikonfigurasi'], ['speed', 'Speedtest lolos']] as [key, label] (key)}
                <label class="flex items-center gap-2 py-1 text-ink-700">
                  <input
                    type="checkbox"
                    class="h-6 w-6 accent-ink-900"
                    checked={checklist[key as keyof InstallationChecklistState]}
                    onchange={(e) => (checklist = { ...checklist, [key]: (e.currentTarget as HTMLInputElement).checked })}
                  />
                  {label}
                </label>
              {/each}
            </div>
            <Field id="wo-notes" label="Catatan" type="textarea" stacked rows={3} value={formNotes} onchange={(v) => (formNotes = String(v ?? ''))} />
            <div class="mt-1 flex flex-wrap items-center gap-2">
              <label class="focus-ring inline-flex h-8 cursor-pointer items-center rounded-lg bg-ink-100 px-3 text-sm">
                {uploadingPhotos ? 'Mengunggah…' : 'Tambah foto'}
                <input type="file" accept="image/*" multiple class="sr-only" onchange={onUploadPhotos} disabled={uploadingPhotos} />
              </label>
              {#each photoIds as pid (pid)}
                <a class="text-sm text-brand-700 underline" href="{getApiBaseUrl()}/storage/files/{pid}/content?token={$token || ''}" target="_blank" rel="noreferrer">foto {pid.slice(0, 6)}</a>
              {/each}
              <Button variant="ghost" size="sm" class="ml-auto" disabled={!!busyId} onclick={() => void saveNotesOnly()}>Simpan catatan</Button>
            </div>
          </div>
        {/if}

        {#if detailError}
          <div class="rounded-lg border border-red-200 bg-red-50 p-2 text-red-700">{detailError}</div>
        {/if}

        {#if canReadAudit}
          <div class="rounded-lg border border-ink-200 p-3">
            <div class="mb-1 font-medium text-ink-900">Riwayat</div>
            {#if timelineLoading}
              <div class="text-ink-500">Memuat…</div>
            {:else if timeline.length === 0}
              <div class="text-ink-500">Belum ada aktivitas tercatat.</div>
            {:else}
              <ul class="max-h-48 space-y-1 overflow-auto">
                {#each timeline as log (log.id)}
                  <li class="flex items-baseline justify-between gap-2 text-ink-700">
                    <span>{log.action.replace(/WORK_ORDER_/, '').toLowerCase()} — {log.details || ''}</span>
                    <span class="shrink-0 text-xs text-ink-400">{formatDateTime(log.created_at, { timeZone: $appSettings.app_timezone })}</span>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}

        <div class="flex justify-end">
          <Button variant="ghost" onclick={closeDetail}>Tutup</Button>
        </div>
      </div>
    {/if}
  </Modal>

  <Modal bind:show={showCancel} title="Batalkan work order">
    <div class="space-y-3 text-sm">
      <p class="text-ink-500">Pembatalan juga membatalkan langganan terkait dan memberi tahu pelanggan. Alasan minimal {CANCEL_REASON_MIN} karakter.</p>
      <Field id="cancel-reason" label="Alasan" type="textarea" stacked rows={3} value={cancelReason} onchange={(v) => (cancelReason = String(v ?? ''))} />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showCancel = false)}>Kembali</Button>
        <Button variant="danger" disabled={cancelReason.trim().length < CANCEL_REASON_MIN} onclick={() => void confirmCancel()}>Batalkan WO</Button>
      </div>
    </div>
  </Modal>

  <Modal bind:show={showVisibility} title="Visibilitas work order">
    <div class="space-y-3 text-sm">
      <p class="text-ink-500">Menentukan siapa selain admin/owner yang bisa melihat WO baru di daftar mereka.</p>
      <Field
        id="vis-mode"
        label="Mode"
        type="select"
        stacked
        value={visibilityMode}
        options={[{ value: 'admin_only', label: 'Hanya admin' }, { value: 'all_staff', label: 'Semua staf instalasi' }]}
        onchange={(v) => (visibilityMode = String(v ?? 'admin_only') as 'admin_only' | 'all_staff')}
      />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (showVisibility = false)}>Batal</Button>
        <Button variant="primary" disabled={visibilityBusy} onclick={() => void saveVisibility()}>Simpan</Button>
      </div>
    </div>
  </Modal>
</AppShell>
