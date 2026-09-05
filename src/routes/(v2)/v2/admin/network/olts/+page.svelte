<script lang="ts">
  /*
    OLT v2 — daftar perangkat.

    Versi lama: (app)/admin/network/olts/+page.svelte (1.111 baris).
    Temuan gelombang 15:
    1. Dropdown menawarkan `vsol_epon`, padahal create_driver() hanya kenal
       hioso/mikrotik/mock -> OLT VSOL tersimpan lalu SEMUA operasi monitoring
       (poll, test, details) gagal "Unsupported OLT type" selamanya.
    2. Pilih "— none —" pada uplink mengirim router_id='' -> FK
       fk_olts_uplink_router menolak -> 500 mentah (dibuktikan via psql).
    3. Polling UI tiap 10 detik padahal poller backend jalan tiap 30 detik.
    Perbaikan server ada di olt_service (validasi + normalisasi) dengan tes.
  */
  import { onDestroy, onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { api } from '$lib/api/client';
  import type { Olt } from '$lib/api/olt';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { appSettings } from '$lib/stores/settings';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { appendBackParam } from '$lib/utils/backNavigation';
  import Modal from '$lib/components/ui/Modal.svelte';
  import MapCanvasShell from '$lib/components/network/MapCanvasShell.svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import {
    AppShell,
    AttentionPanel,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    Icon,
    PageHeader,
    RowActions,
    StatTile,
    type Column,
    type FieldOption,
  } from '$lib/components/ds';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
  import type { RowAction } from '$lib/components/ds/RowActions.svelte';
  import {
    OLT_DRIVER_TYPES,
    friendlyOltError,
    hasOltDriver,
    oltTypeLabel,
    validateOltDraft,
  } from '$lib/utils/oltInsights';

  type RouterRow = { id: string; name: string; host: string; enabled: boolean };

  let loading = $state(true);
  let olts = $state<Olt[]>([]);
  let search = $state('');
  let now = $state(Date.now());
  let testingId = $state<string | null>(null);
  let tickHandle: ReturnType<typeof setInterval> | null = null;

  const canManage = $derived($can('manage', 'router_inventory'));
  const tenantPrefix = $derived($page.url.pathname.replace(/\/admin\/network\/olts.*$/, '') || '');

  /* ── modal form ─────────────────────────────────────────── */
  let formOpen = $state(false);
  let saving = $state(false);
  let editTarget = $state<Olt | null>(null);
  let fName = $state('');
  let fDesc = $state('');
  let fType = $state<string>(OLT_DRIVER_TYPES[0]);
  let fHost = $state('');
  let fPort = $state<string>('80');
  let fUser = $state('');
  let fPass = $state('');
  let fLat = $state<number | null>(null);
  let fLng = $state<number | null>(null);
  let fAddr = $state('');
  let fUplinkRouter = $state('');
  let fUplinkPort = $state('');
  let formErrs = $state<Record<string, string>>({});

  const typeOptions: FieldOption[] = OLT_DRIVER_TYPES.map((t) => ({
    value: t,
    label: oltTypeLabel(t),
  }));

  /* ── modal hapus ────────────────────────────────────────── */
  let deleteOpen = $state(false);
  let deleteTarget = $state<Olt | null>(null);

  /* ── map picker ─────────────────────────────────────────── */
  let showMapPicker = $state(false);
  let pickerMapHost = $state<HTMLDivElement | null>(null);
  let pickerMap: any = null;
  let pickerMarker: any = null;
  let pickerLat = $state<number | null>(null);
  let pickerLng = $state<number | null>(null);
  let pickerViewMode = $state<'standard' | 'satellite'>('standard');
  let pickerMapLoading = $state(false);
  let pickerMapUnavailable = $state(false);
  let pickerMapErrorMessage = $state('');
  let maplibrePromise: Promise<any> | null = null;
  const pickerMapTilerKey = (import.meta.env.VITE_MAPTILER_KEY as string | undefined)?.trim();
  const pickerStandardMaxZoom = 19;
  const pickerSatelliteMaxZoom = pickerMapTilerKey ? 21 : 18;

  /* ── routers untuk dropdown uplink ──────────────────────── */
  let routers = $state<RouterRow[]>([]);
  const routerOptions = $derived<FieldOption[]>([
    { value: '', label: '— Tidak ada —' },
    ...routers.map((r) => ({ value: r.id, label: `${r.name} (${r.host})` })),
  ]);

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return olts;
    return olts.filter((o) =>
      `${o.name} ${o.host} ${o.description ?? ''} ${oltTypeLabel(o.olt_type)}`
        .toLowerCase()
        .includes(q),
    );
  });

  const stats = $derived.by(() => {
    const total = olts.length;
    const online = olts.filter((o) => o.is_online).length;
    const noUplink = olts.filter((o) => !o.uplink_router_id).length;
    return { total, online, offline: total - online, noUplink };
  });

  /* OLT yang datanya tidak bisa dipercaya: error poll terakhir, atau tipe
     tanpa driver (warisan dropdown lama). Versi lama mengubur ini di sel. */
  const attention = $derived<AttentionItem[]>(
    olts
      .map((o): AttentionItem | null => {
        if (!hasOltDriver(o.olt_type)) {
          return {
            icon: 'alert' as const,
            title: `${o.name} — tipe ${oltTypeLabel(o.olt_type)} tidak punya driver`,
            detail: 'Monitoring, uji koneksi, dan detail akan selalu gagal untuk tipe ini.',
            action: 'Buka',
            href: `${tenantPrefix}/admin/network/olts/${o.id}`,
            severity: 'high' as const,
          };
        }
        if (o.last_error) {
          return {
            icon: 'wifi' as const,
            title: `${o.name} — error koneksi terakhir`,
            detail: o.last_error,
            action: 'Buka',
            href: `${tenantPrefix}/admin/network/olts/${o.id}`,
            severity: 'medium' as const,
          };
        }
        return null;
      })
      .filter((x): x is AttentionItem => x !== null),
  );

  const columns: Column[] = [
    { key: 'name', label: 'OLT' },
    { key: 'status', label: 'Status' },
    { key: 'uplink', label: 'Uplink', hideSm: true },
    { key: 'seen', label: 'Data terakhir', hideSm: true },
    { key: 'actions', label: '', align: 'right', width: '170px' },
  ];

  async function load() {
    loading = true;
    try {
      olts = (await api.olt.list()) as Olt[];
      now = Date.now();
    } catch (e: unknown) {
      toast.error(friendlyOltError(extractApiErrorMessage(e)));
    } finally {
      loading = false;
    }
  }

  async function loadSilent() {
    if (formOpen || deleteOpen || showMapPicker) return;
    try {
      olts = (await api.olt.list()) as Olt[];
    } catch {
      /* diam: penyegaran latar */
    }
    now = Date.now();
  }

  function openCreate() {
    editTarget = null;
    fName = '';
    fDesc = '';
    fType = OLT_DRIVER_TYPES[0];
    fHost = '';
    fPort = '80';
    fUser = '';
    fPass = '';
    fLat = null;
    fLng = null;
    fAddr = '';
    fUplinkRouter = '';
    fUplinkPort = '';
    formErrs = {};
    formOpen = true;
  }

  function openEdit(o: Olt) {
    editTarget = o;
    fName = o.name ?? '';
    fDesc = o.description ?? '';
    fType = o.olt_type;
    fHost = o.host ?? '';
    fPort = String(o.port ?? 80);
    fUser = o.username ?? '';
    fPass = '';
    fLat = o.latitude ?? null;
    fLng = o.longitude ?? null;
    fAddr = o.address_line ?? '';
    fUplinkRouter = o.uplink_router_id ?? '';
    fUplinkPort = o.uplink_port ?? '';
    formErrs = {};
    formOpen = true;
  }

  function draft() {
    return {
      name: fName,
      host: fHost,
      port: Number(fPort),
      username: fUser,
      password: fPass,
      oltType: fType,
      latitude: fLat,
      longitude: fLng,
      isNew: !editTarget,
    };
  }

  async function save() {
    const errs = validateOltDraft(draft());
    formErrs = errs;
    if (Object.keys(errs).length) return;
    saving = true;
    try {
      const name = fName.trim();
      const host = fHost.trim();
      const username = fUser.trim();
      const port = Number(fPort);
      const desc = fDesc.trim() || null;
      const addr = fAddr.trim() || null;
      if (editTarget) {
        // Uplink lewat endpoint khusus supaya NetworkLink topologi ikut dibuat.
        if (fUplinkRouter !== (editTarget.uplink_router_id ?? '')) {
          await api.olt.setUplink(editTarget.id, {
            router_id: fUplinkRouter,
            port: fUplinkPort.trim() || null,
          });
        }
        await api.olt.update(editTarget.id, {
          name,
          description: desc,
          host,
          port,
          username,
          password: fPass.trim() ? fPass : undefined,
          latitude: fLat,
          longitude: fLng,
          address_line: addr,
        });
        toast.success('OLT diperbarui.');
      } else {
        await api.olt.create({
          name,
          description: desc,
          olt_type: fType,
          host,
          port,
          username,
          password: fPass,
          latitude: fLat,
          longitude: fLng,
          address_line: addr,
        } as any);
        if (fUplinkRouter) {
          const created = ((await api.olt.list()) as Olt[]).find(
            (x) => x.name === name && x.host === host,
          );
          if (created) {
            await api.olt.setUplink(created.id, {
              router_id: fUplinkRouter,
              port: fUplinkPort.trim() || null,
            });
          }
        }
        toast.success('OLT ditambahkan.');
      }
      formOpen = false;
      await load();
    } catch (e: unknown) {
      const msg = friendlyOltError(extractApiErrorMessage(e));
      formErrs = { ...formErrs, _: msg };
      toast.error(msg);
    } finally {
      saving = false;
    }
  }

  async function uji(o: Olt) {
    testingId = o.id;
    try {
      const res = (await api.olt.test({
        id: o.id,
        host: o.host,
        port: o.port,
        username: o.username,
        password: '',
        olt_type: o.olt_type,
      })) as { success?: boolean; info?: { model?: string; version?: string }; error?: string };
      if (res?.success) {
        toast.success(`Terhubung · ${res.info?.model ?? ''} ${res.info?.version ? `v${res.info.version}` : ''}`.trim());
      } else {
        toast.error(friendlyOltError(res?.error || 'Koneksi gagal.'));
      }
      await loadSilent();
    } catch (e: unknown) {
      toast.error(friendlyOltError(extractApiErrorMessage(e)));
    } finally {
      testingId = null;
    }
  }

  function askDelete(o: Olt) {
    deleteTarget = o;
    deleteOpen = true;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    try {
      await api.olt.delete(deleteTarget.id);
      toast.success('OLT dihapus.');
      deleteOpen = false;
      deleteTarget = null;
      await load();
    } catch (e: unknown) {
      toast.error(friendlyOltError(extractApiErrorMessage(e)));
    }
  }

  function openOnMap(o: Olt) {
    if (o.latitude == null || o.longitude == null) {
      toast.error('OLT belum punya koordinat lokasi.');
      return;
    }
    const params = new URLSearchParams({
      asset_id: o.id,
      asset_lat: String(o.latitude),
      asset_lng: String(o.longitude),
    });
    void goto(`${tenantPrefix}/admin/network/map?${params.toString()}`);
  }

  function rowActions(o: Olt): RowAction[] {
    const acts: RowAction[] = [
      { label: testingId === o.id ? 'Menguji…' : 'Uji koneksi', icon: 'zap', onclick: () => void uji(o) },
      { label: 'Lihat di peta', icon: 'pin', onclick: () => openOnMap(o) },
    ];
    if (canManage) {
      acts.push({ label: 'Sunting', icon: 'cog', onclick: () => openEdit(o) });
      acts.push({ label: 'Hapus', icon: 'close', danger: true, onclick: () => askDelete(o) });
    }
    return acts;
  }

  /* ── map picker (dipertahankan dari legacy, sama persis perilakunya) ── */
  async function getMaplibre() {
    if (!maplibrePromise) maplibrePromise = import('maplibre-gl');
    return maplibrePromise;
  }

  function setPickerPoint(lat: number, lng: number) {
    pickerLat = lat;
    pickerLng = lng;
    if (pickerMarker) {
      pickerMarker.setLngLat([lng, lat]);
      return;
    }
    if (!pickerMap) return;
    pickerMarker = new (pickerMap as any).libregl.Marker({ draggable: true })
      .setLngLat([lng, lat])
      .addTo(pickerMap);
    pickerMarker.on('dragend', () => {
      const pos = pickerMarker.getLngLat();
      pickerLat = Number(pos.lat.toFixed(7));
      pickerLng = Number(pos.lng.toFixed(7));
    });
  }

  function syncPickerViewMode() {
    if (!pickerMap) return;
    const sat = pickerViewMode === 'satellite';
    const vis = (layerId: string, on: boolean) => {
      if (!pickerMap.getLayer(layerId)) return;
      pickerMap.setLayoutProperty(layerId, 'visibility', on ? 'visible' : 'none');
    };
    vis('olt-picker-standard', !sat);
    vis('olt-picker-satellite', sat);
    const maxZ = sat ? pickerSatelliteMaxZoom : pickerStandardMaxZoom;
    pickerMap.setMaxZoom(maxZ);
    if (pickerMap.getZoom() > maxZ) pickerMap.setZoom(maxZ);
  }

  async function openMapPicker() {
    const initialLat = fLat ?? -6.2;
    const initialLng = fLng ?? 106.816666;
    pickerLat = initialLat;
    pickerLng = initialLng;
    pickerMapUnavailable = false;
    pickerMapErrorMessage = '';
    showMapPicker = true;
    await tick();
    if (!pickerMapHost) return;
    pickerMapLoading = true;
    try {
      const libregl = await getMaplibre();
      if (!pickerMap) {
        pickerMap = new libregl.Map({
          container: pickerMapHost,
          style: {
            version: 8,
            sources: {
              standard: {
                type: 'raster',
                tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
                tileSize: 256,
                attribution: 'OpenStreetMap contributors',
                maxzoom: pickerStandardMaxZoom,
              },
              satellite: {
                type: 'raster',
                tiles: pickerMapTilerKey
                  ? [`https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=${pickerMapTilerKey}`]
                  : ['https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'],
                tileSize: 256,
                attribution: pickerMapTilerKey ? 'MapTiler' : 'Esri',
                maxzoom: pickerSatelliteMaxZoom,
              },
            },
            layers: [
              { id: 'olt-picker-standard', type: 'raster', source: 'standard' },
              { id: 'olt-picker-satellite', type: 'raster', source: 'satellite', layout: { visibility: 'none' } },
            ],
          },
          center: [initialLng, initialLat],
          zoom: 13,
          maxZoom: pickerStandardMaxZoom,
        });
        (pickerMap as any).libregl = libregl;
        pickerMap.addControl(new libregl.NavigationControl({ showCompass: true, showZoom: true }), 'top-right');
        pickerMap.addControl(
          new libregl.GeolocateControl({ trackUserLocation: false, showAccuracyCircle: true }),
          'top-right',
        );
        pickerMap.on('click', (event: any) => {
          const { lat, lng } = event.lngLat;
          setPickerPoint(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
        });
      } else {
        pickerMap.resize();
        pickerMap.setCenter([initialLng, initialLat]);
        pickerMap.setZoom(Math.min(13, pickerViewMode === 'satellite' ? pickerSatelliteMaxZoom : pickerStandardMaxZoom));
      }
      syncPickerViewMode();
      setPickerPoint(initialLat, initialLng);
    } catch (e: any) {
      pickerMapUnavailable = true;
      pickerMapErrorMessage = e?.message || 'Peta gagal dimuat';
    } finally {
      pickerMapLoading = false;
      pickerMap?.resize();
    }
  }

  function onPickerSearchSelect(event: CustomEvent<{ lat: number; lng: number }>) {
    const { lat, lng } = event.detail;
    setPickerPoint(Number(lat.toFixed(7)), Number(lng.toFixed(7)));
    pickerMap?.flyTo({ center: [lng, lat], zoom: Math.max(pickerMap.getZoom(), 15), duration: 480 });
  }

  function applyPickedCoordinates() {
    if (pickerLat == null || pickerLng == null) return;
    fLat = pickerLat;
    fLng = pickerLng;
    showMapPicker = false;
  }

  onMount(() => {
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) {
      goto('/unauthorized');
      return;
    }
    void load();
    api.mikrotik
      .routers.list()
      .then((r: any) => {
        if (Array.isArray(r)) routers = r.filter((rt: RouterRow) => rt.enabled);
      })
      .catch(() => {});
    /* Poller backend jalan tiap 30 detik; UI ikut ritme itu, bukan 10 detik. */
    tickHandle = setInterval(() => {
      now = Date.now();
      void loadSilent();
    }, 30_000);
  });

  onDestroy(() => {
    if (tickHandle) clearInterval(tickHandle);
    pickerMarker?.remove();
    pickerMap?.remove();
  });
</script>

<AppShell title="OLT">
  <PageHeader
    title="OLT"
    eyebrow="Jaringan"
    desc="Optical Line Terminal yang dipantau. Status, ONU, dan riwayat diambil oleh poller backend tiap 30 detik."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={load}>Muat ulang</Button>
      {#if canManage}
        <Button icon="plus" onclick={openCreate}>Tambah OLT</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-4">
      <StatTile label="Total OLT" value={String(stats.total)} hint="perangkat terdaftar" />
      <StatTile
        label="Online"
        value={String(stats.online)}
        hint={stats.total ? `${stats.online} dari ${stats.total} menjawab poll terakhir` : 'belum ada perangkat'}
        tone={stats.total > 0 && stats.online === stats.total ? 'positive' : 'neutral'}
      />
      <StatTile
        label="Offline"
        value={String(stats.offline)}
        hint={stats.offline ? 'tidak menjawab poll terakhir' : 'semua menjawab'}
        tone={stats.offline ? 'negative' : 'neutral'}
      />
      <StatTile
        label="Tanpa uplink"
        value={String(stats.noUplink)}
        hint="belum dipetakan ke router"
        tone={stats.noUplink ? 'warning' : 'neutral'}
      />
    </div>
  </Card>

  {#if attention.length}
    <div class="mt-4">
      <AttentionPanel items={attention} title="Perlu diperiksa" />
    </div>
  {/if}

  <div class="mt-4">
    <Card>
      <div class="mb-3 flex items-center gap-2">
        <div class="relative max-w-sm flex-1">
          <Icon
            name="search"
            size={15}
            class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400"
          />
          <input
            bind:value={search}
            placeholder="Cari nama, host, atau tipe"
            aria-label="Cari OLT"
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          />
        </div>
      </div>

      <DataTable
        {columns}
        rows={filtered}
        {loading}
        emptyTitle="Belum ada OLT"
        emptyHint="Tambahkan perangkat OLT untuk mulai memantau PON."
        footNote={`${filtered.length} dari ${olts.length} OLT`}
      >
        {#snippet cell(o, c)}
          {#if c.key === 'name'}
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <a
                  href={`${tenantPrefix}/admin/network/olts/${o.id}`}
                  class="font-medium text-ink-900 hover:underline"
                >{o.name}</a>
                <span class="rounded-md bg-ink-100 px-1.5 py-0.5 text-sm text-ink-600">{oltTypeLabel(o.olt_type)}</span>
                {#if !hasOltDriver(o.olt_type)}
                  <span class="rounded-md bg-red-50 px-1.5 py-0.5 text-sm text-red-700 ring-1 ring-inset ring-red-200"
                    >tanpa driver</span
                  >
                {/if}
              </div>
              <div class="num text-sm text-ink-500">{o.username}@{o.host}:{o.port}</div>
              {#if o.description}
                <div class="truncate text-sm text-ink-400">{o.description}</div>
              {/if}
            </div>
          {:else if c.key === 'status'}
            <div class="min-w-0">
              <Badge tone={o.is_online ? 'positive' : 'negative'} label={o.is_online ? 'Online' : 'Offline'} />
              {#if o.last_error}
                <div class="mt-1 max-w-[22rem] truncate text-sm text-red-700" title={o.last_error}>
                  {o.last_error}
                </div>
              {/if}
            </div>
          {:else if c.key === 'uplink'}
            {#if o.uplink_router_name}
              <div>{o.uplink_router_name}</div>
              {#if o.uplink_port}
                <div class="num text-sm text-ink-500">{o.uplink_port}</div>
              {/if}
            {:else if o.uplink_router_id}
              <span class="num text-sm text-ink-400">{o.uplink_router_id}</span>
            {:else}
              <span class="text-ink-400">—</span>
            {/if}
          {:else if c.key === 'seen'}
            {#if o.last_polled_at}
              <span class="text-sm text-ink-500" title={formatDateTime(o.last_polled_at, { timeZone: $appSettings.app_timezone })}>
                {timeAgo(o.last_polled_at)}
              </span>
            {:else}
              <span class="text-ink-400">belum pernah</span>
            {/if}
          {:else if c.key === 'actions'}
            <RowActions
              primary={{ label: 'Buka', icon: 'chevronRight', href: `${tenantPrefix}/admin/network/olts/${o.id}` }}
              rest={rowActions(o)}
            />
          {/if}
        {/snippet}
      </DataTable>
    </Card>
  </div>
</AppShell>

<!-- Modal form OLT -->
<Modal
  bind:show={formOpen}
  title={editTarget ? `Sunting OLT — ${editTarget.name}` : 'OLT baru'}
  width="720px"
>
  <div class="space-y-1 py-1">
    {#if formErrs._}
      <div class="mb-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 ring-1 ring-inset ring-red-200">
        {formErrs._}
      </div>
    {/if}
    <div class="grid gap-x-6 sm:grid-cols-2">
      <Field stacked id="o-name" label="Nama" value={fName} placeholder="OLT Jambu"
        error={formErrs.name} onchange={(v) => (fName = v)} />
      <Field stacked id="o-desc" label="Deskripsi" value={fDesc} placeholder="Catatan lokasi / seri"
        onchange={(v) => (fDesc = v)} />
    </div>
    <div class="grid gap-x-6 sm:grid-cols-2">
      <Field stacked id="o-type" label="Tipe" value={fType} type="select" options={typeOptions}
        disabled={!!editTarget} error={formErrs.oltType}
        help={editTarget ? 'Tipe tidak bisa diubah setelah dibuat.' : 'Hanya tipe dengan driver di server.'}
        onchange={(v) => (fType = v)} />
      <div></div>
    </div>
    <div class="grid gap-x-6 sm:grid-cols-[1fr_160px]">
      <Field stacked id="o-host" label="Host" value={fHost} placeholder="192.168.1.1"
        error={formErrs.host} onchange={(v) => (fHost = v)} />
      <Field stacked id="o-port" label="Port" value={fPort} type="number" min={1} max={65535}
        error={formErrs.port} onchange={(v) => (fPort = v)} />
    </div>
    <div class="grid gap-x-6 sm:grid-cols-2">
      <Field stacked id="o-user" label="Username" value={fUser} placeholder="admin"
        error={formErrs.username} onchange={(v) => (fUser = v)} />
      <Field stacked id="o-pass" label="Password" value={fPass} type="password"
        placeholder={editTarget ? 'Kosongkan untuk mempertahankan' : '••••••'}
        error={formErrs.password} onchange={(v) => (fPass = v)} />
    </div>

    <div class="mt-5 mb-2 border-t border-ink-200 pt-4 text-sm font-semibold uppercase tracking-wide text-ink-500">
      Lokasi
    </div>
    <div class="mb-3">
      <button
        type="button"
        class="focus-ring flex w-full items-center justify-center gap-2 rounded-lg border border-dashed border-ink-300 px-3 py-2.5 text-base text-ink-600 transition hover:border-ink-400 disabled:opacity-50"
        onclick={openMapPicker}
        disabled={saving}
      >
        <Icon name="pin" size={15} />
        {fLat != null && fLng != null
          ? `Ubah titik (${fLat.toFixed(6)}, ${fLng.toFixed(6)})`
          : 'Pilih lokasi di peta'}
      </button>
      {#if formErrs.location}
        <div class="mt-1 text-sm text-red-700">{formErrs.location}</div>
      {/if}
    </div>
    <Field stacked id="o-addr" label="Alamat" value={fAddr} placeholder="Jalan, desa, kecamatan"
      onchange={(v) => (fAddr = v)} />

    <div class="mt-5 mb-2 border-t border-ink-200 pt-4 text-sm font-semibold uppercase tracking-wide text-ink-500">
      Uplink router
    </div>
    <div class="grid gap-x-6 sm:grid-cols-[1fr_160px]">
      <Field stacked id="o-up-router" label="Router" value={fUplinkRouter} type="select"
        options={routerOptions} help="Membuat koneksi otomatis di peta topologi."
        onchange={(v) => (fUplinkRouter = v)} />
      <Field stacked id="o-up-port" label="Port" value={fUplinkPort} placeholder="ether1"
        onchange={(v) => (fUplinkPort = v)} />
    </div>
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (formOpen = false)}>Batal</Button>
    <Button onclick={() => void save()} disabled={saving}>
      {saving ? 'Menyimpan…' : editTarget ? 'Simpan perubahan' : 'Buat OLT'}
    </Button>
  {/snippet}
</Modal>

<!-- Modal hapus -->
<Modal bind:show={deleteOpen} title="Hapus OLT" width="480px">
  <p class="py-2 text-ink-700">
    Hapus <strong>{deleteTarget?.name}</strong> ({deleteTarget?.host}:{deleteTarget?.port})?
    Riwayat ONU perangkat ini ikut terhapus dan peta topologi akan diperbarui.
  </p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (deleteOpen = false)}>Batal</Button>
    <Button variant="danger" onclick={() => void confirmDelete()}>Hapus</Button>
  {/snippet}
</Modal>

<!-- Modal pemilih lokasi peta -->
{#if showMapPicker}
  <!-- svelte-ignore a11y_interactive_supports_focus, a11y_click_events_have_key_events -->
  <div class="fixed inset-0 z-[70] grid place-items-center bg-black/40 p-4" onclick={() => (showMapPicker = false)} role="presentation">
    <div class="w-full max-w-3xl overflow-hidden rounded-2xl bg-white shadow-2xl" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="flex items-center justify-between border-b border-ink-200 px-5 py-3">
        <h3 class="font-semibold text-ink-900">Pilih lokasi di peta</h3>
        <button type="button" class="rounded-lg p-1.5 text-ink-500 hover:bg-ink-100 focus-ring" onclick={() => (showMapPicker = false)}>
          <Icon name="close" size={18} />
        </button>
      </div>
      <div class="p-5">
        <div class="mb-2 flex items-center justify-between">
          <span class="text-sm text-ink-500">Klik peta untuk memindahkan titik.</span>
          {#if pickerLat != null && pickerLng != null}
            <span class="num text-sm text-ink-700">{pickerLat.toFixed(7)}, {pickerLng.toFixed(7)}</span>
          {/if}
        </div>
        <MapCanvasShell
          bind:mapEl={pickerMapHost}
          bind:viewMode={pickerViewMode}
          on:searchselect={onPickerSearchSelect}
          loading={pickerMapLoading}
          mapUnavailable={pickerMapUnavailable}
          mapErrorMessage={pickerMapErrorMessage}
          mapUnavailableTitle="Peta tidak tersedia di perangkat ini"
          mapUnavailableSubtitle="Koordinat tetap bisa diisi lewat tombol di bawah."
          height="min(55vh, 480px)"
        />
        <div class="mt-4 flex justify-end gap-2">
          <Button variant="ghost" onclick={() => (showMapPicker = false)}>Batal</Button>
          <Button onclick={applyPickedCoordinates}>Gunakan titik ini</Button>
        </div>
      </div>
    </div>
  </div>
{/if}
