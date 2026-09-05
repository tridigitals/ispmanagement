<script lang="ts">
  /*
    OLT v2 — detail perangkat.

    Versi lama: (app)/admin/network/olts/[id]/+page.svelte (1.055 baris).
    Temuan gelombang 15:
    1. Tile "Sinyal lemah" menghitung `o.rx_power < -27`, padahal server hanya
       mengirim `rx` bertipe string ("21.4 dBm") -> rx_power selalu undefined
       -> tile itu SELALU 0 di layar lama.
    2. Sortir kolom 'signal' memakai parseFloat(a.rx) tanpa membuang satuan ->
       parseFloat("21.4 dBm") OK, tapi nilai minus unicode tidak; kini lewat
       parseDbm() yang teruji.
    3. Polling UI 10 detik vs poller backend 30 detik (sama seperti list).
  */
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { Olt, OltDetails, OltOnuHistoryEntry, OltStats, OnuDetail } from '$lib/api/olt';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { resolveBackTarget } from '$lib/utils/backNavigation';
  import { appSettings } from '$lib/stores/settings';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import Modal from '$lib/components/ui/Modal.svelte';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    DetailHeader,
    FieldRow,
    Icon,
    StatTile,
    Tabs,
    type Column,
  } from '$lib/components/ds';
  import {
    friendlyOltError,
    hasOltDriver,
    onuStatusTone,
    oltTypeLabel,
    parseDbm,
    signalColor,
    signalLabel,
  } from '$lib/utils/oltInsights';

  let initialLoading = $state(true);
  let refreshing = $state(false);
  let olt = $state<Olt | null>(null);
  let stats = $state<OltStats | null>(null);
  let onus = $state<OnuDetail[]>([]);
  let details = $state<OltDetails | null>(null);
  let history = $state<OltOnuHistoryEntry[]>([]);
  let historyLoading = $state(false);
  let refreshInFlight = $state(false);
  let activeTab = $state<'overview' | 'onus' | 'history'>('overview');
  let tickHandle: ReturnType<typeof setInterval> | null = null;

  let rebootOpen = $state(false);
  let rebootTarget = $state<OnuDetail | null>(null);
  let rebooting = $state(false);

  const id = $derived($page.params.id || '');
  const listPath = $derived($page.url.pathname.replace(/\/[^/]+\/?$/, ''));
  const backTarget = $derived(resolveBackTarget($page.url, listPath));
  const tenantPrefix = $derived($page.url.pathname.replace(/\/admin\/network\/olts\/.*$/, '') || '');

  const onuStats = $derived.by(() => {
    const total = onus.length;
    const online = onus.filter((o) => (o.status ?? '').toLowerCase() === 'online').length;
    // Perbaikan bug lama: rx adalah string ("21.4 dBm"), bukan rx_power.
    const low = onus.filter((o) => {
      const rx = parseDbm(o.rx);
      return rx != null && rx < -27;
    }).length;
    return { total, online, offline: total - online, low };
  });

  const tabItems = $derived([
    { id: 'overview', label: 'Ringkasan' },
    { id: 'onus', label: 'ONU', count: onus.length },
    { id: 'history', label: 'Riwayat ONU' },
  ]);

  const onuColumns: Column[] = [
    { key: 'onu', label: 'ONU' },
    { key: 'port', label: 'PON' },
    { key: 'status', label: 'Status' },
    { key: 'signal', label: 'Sinyal (dBm)' },
    { key: 'distance', label: 'Jarak', hideSm: true },
    { key: 'mac', label: 'MAC', hideSm: true },
    { key: 'actions', label: '', align: 'right', width: '110px' },
  ];

  const historyColumns: Column[] = [
    { key: 'time', label: 'Waktu' },
    { key: 'onu', label: 'ONU' },
    { key: 'pon', label: 'PON', hideSm: true },
    { key: 'status', label: 'Status' },
    { key: 'rx', label: 'RX (dBm)', align: 'right', num: true },
    { key: 'tx', label: 'TX (dBm)', align: 'right', num: true, hideSm: true },
  ];

  const sortedOnus = $derived([...onus].sort((a, b) => (a.pon + a.onu_id).localeCompare(b.pon + b.onu_id)));

  async function refresh(silent = false) {
    if (refreshInFlight || !id) return;
    refreshInFlight = true;
    if (!silent) {
      if (!olt) initialLoading = true;
      else refreshing = true;
    }
    try {
      const d = (await api.olt.details(id)) as OltDetails;
      details = d;
      stats = d?.stats || null;
      onus = d?.onus || [];
      try {
        const info = (await api.olt.get(id)) as Olt;
        if (info) olt = info;
      } catch {
        /* pakai data details saja */
      }
    } catch (e: unknown) {
      if (!silent) toast.error(friendlyOltError(extractApiErrorMessage(e)));
    } finally {
      initialLoading = false;
      refreshing = false;
      refreshInFlight = false;
    }
  }

  async function loadHistory() {
    if (!id) return;
    historyLoading = true;
    try {
      history = (await api.olt.onuHistory(id)) as OltOnuHistoryEntry[];
    } catch (e: unknown) {
      toast.error(friendlyOltError(extractApiErrorMessage(e)));
    } finally {
      historyLoading = false;
    }
  }

  function switchTab(tabId: string) {
    activeTab = tabId as typeof activeTab;
    if (tabId === 'history' && history.length === 0) void loadHistory();
  }

  async function forceRefreshStats() {
    if (!id) return;
    refreshing = true;
    try {
      await refresh(false);
      toast.success('Data disegarkan langsung dari perangkat.');
    } finally {
      refreshing = false;
    }
  }

  function promptReboot(onu: OnuDetail) {
    rebootTarget = onu;
    rebootOpen = true;
  }

  async function confirmReboot() {
    if (!rebootTarget) return;
    rebooting = true;
    try {
      await api.olt.rebootOnu(id, rebootTarget.onu_id, rebootTarget.name || rebootTarget.onu_id);
      toast.success(`ONU ${rebootTarget.name || rebootTarget.onu_id} sedang reboot.`);
      rebootOpen = false;
      rebootTarget = null;
    } catch (e: unknown) {
      toast.error(friendlyOltError(extractApiErrorMessage(e)));
    } finally {
      rebooting = false;
    }
  }

  function openOnMap() {
    if (!olt || olt.latitude == null || olt.longitude == null) {
      toast.error('OLT belum punya koordinat lokasi.');
      return;
    }
    const params = new URLSearchParams({
      asset_id: olt.id,
      asset_lat: String(olt.latitude),
      asset_lng: String(olt.longitude),
    });
    void goto(`${tenantPrefix}/admin/network/map?${params.toString()}`);
  }

  onMount(() => {
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) {
      goto('/unauthorized');
      return;
    }
    void refresh(false);
    tickHandle = setInterval(() => void refresh(true), 30_000);
  });

  onDestroy(() => {
    if (tickHandle) clearInterval(tickHandle);
  });
</script>

<AppShell title={olt?.name ?? 'Detail OLT'}>
  <DetailHeader
    title={olt?.name ?? 'Detail OLT'}
    subtitle={olt ? `${oltTypeLabel(olt.olt_type)} — ${olt.host}:${olt.port}` : undefined}
    statusLabel={olt ? (olt.is_online ? 'Online' : 'Offline') : undefined}
    statusTone={olt ? (olt.is_online ? 'positive' : 'negative') : undefined}
    backHref={backTarget}
    backLabel="Kembali ke daftar OLT"
  >
    {#snippet actions()}
      <Button variant="ghost" icon="pin" onclick={openOnMap} disabled={!olt || olt.latitude == null}>
        Lihat di peta
      </Button>
      <Button variant="ghost" icon="refresh" onclick={() => void refresh(false)} disabled={refreshing}>
        Muat ulang
      </Button>
      <Button icon="zap" onclick={() => void forceRefreshStats()} disabled={refreshing || !id}>
        Segarkan dari perangkat
      </Button>
    {/snippet}
  </DetailHeader>

  {#if initialLoading}
    <Card>
      <div class="flex items-center gap-3 py-6 text-ink-500">
        <Icon name="clock" size={18} />
        Memuat data OLT…
      </div>
    </Card>
  {:else if !olt}
    <Card>
      <div class="py-6 text-center text-ink-500">
        OLT tidak ditemukan.
        <a class="text-brand-600 hover:underline" href={backTarget}>Kembali ke daftar</a>
      </div>
    </Card>
  {:else}
    {#if !hasOltDriver(olt.olt_type)}
      <div class="mb-4 rounded-xl bg-red-50 px-4 py-3 text-sm text-red-800 ring-1 ring-inset ring-red-200">
        Tipe <strong>{oltTypeLabel(olt.olt_type)}</strong> tidak punya driver di server —
        statistik, ONU, dan riwayat tidak akan pernah berhasil untuk perangkat ini.
      </div>
    {/if}
    {#if olt.last_error}
      <div class="mb-4 rounded-xl bg-amber-50 px-4 py-3 text-sm text-amber-900 ring-1 ring-inset ring-amber-200">
        Error koneksi terakhir: {olt.last_error}
      </div>
    {/if}

    <div class="mb-4">
      <Tabs items={tabItems} active={activeTab} onselect={switchTab} />
    </div>

    {#if activeTab === 'overview'}
      <div class="grid grid-cols-2 gap-6 sm:grid-cols-4">
        <StatTile
          label="Total ONU"
          value={String(stats?.total_onus ?? onuStats.total)}
          hint="terdaftar di perangkat"
        />
        <StatTile
          label="ONU Online"
          value={String(stats?.online_onus ?? onuStats.online)}
          hint="menjawab poll terakhir"
          tone={onuStats.online === onuStats.total && onuStats.total > 0 ? 'positive' : 'neutral'}
        />
        <StatTile
          label="ONU Offline"
          value={String(stats?.offline_onus ?? onuStats.offline)}
          hint="tidak menjawab poll terakhir"
          tone={stats?.offline_onus || onuStats.offline ? 'negative' : 'neutral'}
        />
        <StatTile
          label="Sinyal lemah (di bawah −27 dBm)"
          value={String(stats?.low_onus ?? onuStats.low)}
          hint="berisiko sering drop"
          tone={stats?.low_onus || onuStats.low ? 'warning' : 'neutral'}
        />
      </div>

      {#if details?.info}
        <div class="mt-4">
          <Card title="Info sistem">
            <div class="grid gap-x-8 sm:grid-cols-2">
              {#if details.info.model}
                <FieldRow label="Model" value={details.info.model} />
              {/if}
              {#if details.info.version}
                <FieldRow label="Versi firmware" value={details.info.version} mono />
              {/if}
              {#if details.info.address}
                <FieldRow label="Alamat" value={details.info.address} mono />
              {/if}
              {#each Object.entries(details.info).filter(([k]) => !['name', 'model', 'version', 'address'].includes(k)) as [key, val] (key)}
                {#if val != null}
                  <FieldRow label={key} value={String(val)} />
                {/if}
              {/each}
            </div>
          </Card>
        </div>
      {/if}

      <div class="mt-4">
        <Card title="Konfigurasi">
          <div class="grid gap-x-8 sm:grid-cols-2">
            <FieldRow label="Tipe" value={oltTypeLabel(olt.olt_type)} />
            <FieldRow label="Endpoint" value={`${olt.host}:${olt.port}`} mono />
            <FieldRow label="Username" value={olt.username} />
            <FieldRow
              label="Data terakhir"
              value={olt.last_polled_at ? `${timeAgo(olt.last_polled_at)} lalu` : 'belum pernah'}
            />
            <FieldRow
              label="Koordinat"
              value={olt.latitude != null && olt.longitude != null ? `${olt.latitude}, ${olt.longitude}` : null}
              mono
            />
            <FieldRow label="Alamat lokasi" value={olt.address_line} />
            <FieldRow label="Uplink router" value={olt.uplink_router_name || olt.uplink_router_id} />
            <FieldRow label="Uplink port" value={olt.uplink_port} mono />
          </div>
        </Card>
      </div>

      {#if stats?.pon_ports && stats.pon_ports.length > 0}
        <div class="mt-4">
          <Card title="Port PON">
            <DataTable
              columns={[
                { key: 'name', label: 'Port' },
                { key: 'total', label: 'ONU', align: 'right', num: true },
                { key: 'online', label: 'Online', align: 'right', num: true },
                { key: 'offline', label: 'Offline', align: 'right', num: true },
              ]}
              rows={stats.pon_ports}
              footNote={`${stats.pon_ports.length} port aktif`}
            >
              {#snippet cell(port, c)}
                {#if c.key === 'name'}
                  <span class="num text-ink-900">{port.name}</span>
                {:else if c.key === 'offline'}
                  <span class={port.offline > 0 ? 'text-red-700' : 'text-ink-500'}>{port.offline}</span>
                {/if}
              {/snippet}
            </DataTable>
          </Card>
        </div>
      {/if}
    {:else if activeTab === 'onus'}
      <Card>
        <DataTable
          columns={onuColumns}
          rows={sortedOnus}
          emptyTitle="Belum ada data ONU"
          emptyHint="Klik 'Segarkan dari perangkat' untuk menarik data langsung dari OLT."
          footNote={`${sortedOnus.length} ONU · ${onuStats.online} online · ${onuStats.low} sinyal lemah`}
        >
          {#snippet cell(o, c)}
            {#if c.key === 'onu'}
              <div class="min-w-0">
                <div class="font-medium text-ink-900">{o.name || o.onu_id}</div>
                {#if o.name}
                  <div class="num text-sm text-ink-400">{o.onu_id}</div>
                {/if}
              </div>
            {:else if c.key === 'port'}
              <span class="num text-ink-700">{o.pon}</span>
            {:else if c.key === 'status'}
              <Badge tone={onuStatusTone(o.status)} label={o.status || '—'} />
            {:else if c.key === 'signal'}
              {@const rx = parseDbm(o.rx)}
              {#if rx != null}
                <span class="num" style:color={signalColor(rx)} title={signalLabel(rx)}>
                  {o.rx}
                </span>
              {:else}
                <span class="text-ink-400">{o.rx || '—'}</span>
              {/if}
            {:else if c.key === 'distance'}
              <span class="text-ink-700">{o.distance || '—'}</span>
            {:else if c.key === 'mac'}
              <span class="num text-sm text-ink-500">{o.mac || '—'}</span>
            {:else if c.key === 'actions'}
              {#if $can('manage', 'router_inventory')}
                <Button size="sm" variant="secondary" onclick={() => promptReboot(o)}>Reboot</Button>
              {/if}
            {/if}
          {/snippet}
        </DataTable>
      </Card>
    {:else if activeTab === 'history'}
      <Card>
        <DataTable
          columns={historyColumns}
          rows={history}
          loading={historyLoading}
          emptyTitle="Belum ada riwayat"
          emptyHint="Riwayat terisi otomatis setiap poll."
          footNote={`${history.length} entri terakhir (maks 200)`}
        >
          {#snippet cell(h, c)}
            {#if c.key === 'time'}
              <span class="text-sm text-ink-700" title={formatDateTime(h.recorded_at, { timeZone: $appSettings.app_timezone })}>
                {formatDateTime(h.recorded_at, { timeZone: $appSettings.app_timezone })}
              </span>
            {:else if c.key === 'onu'}
              <span class="text-ink-900">{h.name || h.onu_id}</span>
            {:else if c.key === 'pon'}
              <span class="num text-ink-700">{h.pon}</span>
            {:else if c.key === 'status'}
              <Badge tone={onuStatusTone(h.status)} label={h.status} />
            {:else if c.key === 'rx'}
              {@const rx = h.rx_power}
              {#if rx != null}
                <span class="num" style:color={signalColor(rx)}>{rx.toFixed(1)}</span>
              {:else}
                <span class="text-ink-400">—</span>
              {/if}
            {:else if c.key === 'tx'}
              <span class="num text-ink-700">{h.tx_power != null ? h.tx_power.toFixed(1) : '—'}</span>
            {/if}
          {/snippet}
        </DataTable>
      </Card>
    {/if}
  {/if}
</AppShell>

<!-- Konfirmasi reboot ONU -->
<Modal bind:show={rebootOpen} title="Reboot ONU" width="480px">
  <p class="py-2 text-ink-700">
    Reboot ONU <strong>{rebootTarget?.name || rebootTarget?.onu_id}</strong> pada port
    {rebootTarget?.pon}? Pelanggan di belakang ONU ini akan kehilangan koneksi ±1–2 menit.
  </p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (rebootOpen = false)}>Batal</Button>
    <Button variant="danger" onclick={() => void confirmReboot()} disabled={rebooting}>
      {rebooting ? 'Mengirim…' : 'Reboot'}
    </Button>
  {/snippet}
</Modal>
