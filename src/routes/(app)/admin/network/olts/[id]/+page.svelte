<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import type { Olt, OltStats, OltDetails, OnuDetail, OltOnuHistoryEntry, OltPublicToken } from '$lib/api/olt';
  import { toast } from '$lib/stores/toast';
  import { resolveBackTarget } from '$lib/utils/backNavigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

  const OLT_TYPE_MAP: Record<string, string> = {
    hioso_ha7302cst: 'HIOSO HA-7302CST (EPON)',
    vsol_epon: 'VSOL (EPON)',
  };

  function friendlyOltType(t: string): string {
    return OLT_TYPE_MAP[t] || t;
  }

  function signalColor(dbm: number | null | undefined): string {
    if (dbm == null) return 'var(--text-secondary)';
    if (dbm > -20) return '#22c55e';
    if (dbm >= -24) return '#eab308';
    if (dbm >= -27) return '#f97316';
    return '#ef4444';
  }

  function signalLabel(dbm: number | null | undefined): string {
    if (dbm == null) return '—';
    if (dbm > -20) return 'Baik';
    if (dbm >= -24) return 'Cukup';
    if (dbm >= -27) return 'Lemah';
    return 'Sangat Lemah';
  }

  function formatUptime(seconds?: number | null): string {
    if (seconds == null) return '—';
    const d = Math.floor(seconds / 86400);
    const h = Math.floor((seconds % 86400) / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    if (d > 0) return `${d}h ${h}j ${m}m`;
    if (h > 0) return `${h}j ${m}m`;
    return `${m}m`;
  }

  function formatBytes(bytes?: number | null): string {
    if (bytes == null) return '—';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  let initialLoading = $state(true);
  let refreshing = $state(false);
  let olt = $state<Olt | null>(null);
  let stats = $state<OltStats | null>(null);
  let onus = $state<OnuDetail[]>([]);
  let history = $state<OltOnuHistoryEntry[]>([]);
  let publicTokens = $state<OltPublicToken[]>([]);
  let isMobile = $state(false);

  let activeTab = $state<'overview' | 'onus' | 'history' | 'tokens'>('overview');
  let onuSearch = $state('');
  let historyLoading = $state(false);
  let tokensLoading = $state(false);
  let refreshInFlight = $state(false);
  let forceRefresh = $state(false);

  // Token form
  let showTokenModal = $state(false);
  let tokenDesc = $state('');
  let tokenEnabled = $state(true);
  let tokenExpiry = $state('');
  let savingToken = $state(false);

  // Reboot confirm
  let showRebootConfirm = $state(false);
  let rebootTarget = $state<OnuDetail | null>(null);

  // Token delete confirm
  let showTokenDeleteConfirm = $state(false);
  let tokenDeleteTarget = $state<OltPublicToken | null>(null);

  const oltListPath = $derived($page.url.pathname.replace(/\/[^/]+\/?$/, ''));
  const backTarget = $derived(resolveBackTarget($page.url, oltListPath));

  const filteredOnus = $derived.by(() => {
    const q = onuSearch.trim().toLowerCase();
    if (!q) return onus;
    return onus.filter((o) => {
      const hay = `${o.onu_id} ${o.onu_name || ''} ${o.serial_number || ''} ${o.pon_port || ''}`.toLowerCase();
      return hay.includes(q);
    });
  });

  const onuStats = $derived.by(() => {
    const total = onus.length;
    const online = onus.filter((o) => o.status === 'online').length;
    const offline = total - online;
    const low = onus.filter((o) => o.rx_power != null && o.rx_power < -27).length;
    return { total, online, offline, low };
  });

  const tabItems = $derived.by(() => [
    { id: 'overview', label: 'Ringkasan' },
    { id: 'onus', label: 'ONU', count: onus.length },
    { id: 'history', label: 'Riwayat ONU' },
    { id: 'tokens', label: 'Token Publik', count: publicTokens.length },
  ]);

  const onuColumns = $derived.by(() => [
    { key: 'onu', label: 'ONU' },
    { key: 'port', label: 'PON Port' },
    { key: 'status', label: 'Status' },
    { key: 'signal', label: 'Sinyal (dBm)' },
    { key: 'distance', label: 'Jarak' },
    { key: 'uptime', label: 'Uptime' },
    { key: 'actions', label: '', align: 'right' as const, width: '60px' },
  ]);

  const historyColumns = $derived.by(() => [
    { key: 'time', label: 'Waktu' },
    { key: 'onu', label: 'ONU' },
    { key: 'event', label: 'Event' },
    { key: 'message', label: 'Pesan' },
  ]);

  const tokenColumns = $derived.by(() => [
    { key: 'token', label: 'Token' },
    { key: 'description', label: 'Deskripsi' },
    { key: 'status', label: 'Status' },
    { key: 'expires', label: 'Kadaluarsa' },
    { key: 'actions', label: '', align: 'right' as const, width: '60px' },
  ]);

  let refreshHandle: any = null;

  onMount(() => {
    if (!$can('read', 'router_inventory') && !$can('manage', 'router_inventory')) {
      goto('/unauthorized');
      return;
    }
    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 1024px)');
      const sync = () => (isMobile = mq.matches);
      sync();
      try {
        mq.addEventListener('change', sync);
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
      }
    }
    void refresh({ silent: false });
    refreshHandle = setInterval(() => void refresh({ silent: true }), 10000);
  });

  onDestroy(() => {
    if (refreshHandle) clearInterval(refreshHandle);
  });

  async function refresh(opts?: { silent?: boolean }) {
    if (refreshInFlight) return;
    refreshInFlight = true;
    if (!opts?.silent) {
      if (!olt) initialLoading = true;
      else refreshing = true;
    }
    const id = $page.params.id || '';
    if (!id) {
      initialLoading = false;
      refreshing = false;
      refreshInFlight = false;
      return;
    }
    try {
      const [details] = await Promise.all([
        api.olt.details(id) as any,
      ]);
      olt = details?.stats ? { id: details.olt_id, name: details.olt_name, is_online: details.is_online, ...((olt || {}) as any) } as any : olt;
      // Fetch basic OLT info too
      try {
        const oltInfo = await api.olt.get(id) as any;
        if (oltInfo) olt = oltInfo;
      } catch { /* use details data */ }
      stats = details?.stats || null;
      onus = details?.onus || [];
    } catch (e: any) {
      if (!opts?.silent) toast.error(e?.message || e);
    } finally {
      initialLoading = false;
      refreshing = false;
      refreshInFlight = false;
    }
  }

  async function loadHistory() {
    const id = $page.params.id || '';
    if (!id) return;
    historyLoading = true;
    try {
      history = (await api.olt.onuHistory(id)) as any;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      historyLoading = false;
    }
  }

  async function loadTokens() {
    const id = $page.params.id || '';
    if (!id) return;
    tokensLoading = true;
    try {
      publicTokens = (await api.olt.listPublicTokens(id)) as any;
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      tokensLoading = false;
    }
  }

  function switchTab(tabId: string) {
    activeTab = tabId as any;
    if (tabId === 'history' && history.length === 0) void loadHistory();
    if (tabId === 'tokens' && publicTokens.length === 0) void loadTokens();
  }

  async function forceRefreshStats() {
    const id = $page.params.id || '';
    if (!id) return;
    refreshing = true;
    try {
      const details = await api.olt.details(id) as any;
      stats = details?.stats || null;
      onus = details?.onus || [];
      toast.success('Data diperbarui.');
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      refreshing = false;
    }
  }

  function promptReboot(onu: OnuDetail) {
    rebootTarget = onu;
    showRebootConfirm = true;
  }

  async function confirmReboot() {
    if (!rebootTarget) return;
    const id = $page.params.id || '';
    const onu = rebootTarget;
    rebootTarget = null;
    try {
      await api.olt.rebootOnu(id, onu.onu_id, onu.onu_name || onu.onu_id);
      toast.success(`ONU ${onu.onu_name || onu.onu_id} sedang di-reboot.`);
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  function openTokenModal() {
    tokenDesc = '';
    tokenEnabled = true;
    tokenExpiry = '';
    showTokenModal = true;
  }

  async function createToken() {
    const id = $page.params.id || '';
    if (!id) return;
    savingToken = true;
    try {
      const expiryIso = tokenExpiry ? new Date(tokenExpiry).toISOString() : null;
      await api.olt.createPublicToken(id, {
        description: tokenDesc.trim() || null,
        enabled: tokenEnabled,
        expires_at: expiryIso,
      });
      toast.success('Token berhasil dibuat.');
      showTokenModal = false;
      await loadTokens();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      savingToken = false;
    }
  }

  function promptDeleteToken(token: OltPublicToken) {
    tokenDeleteTarget = token;
    showTokenDeleteConfirm = true;
  }

  async function confirmDeleteToken() {
    if (!tokenDeleteTarget) return;
    const id = $page.params.id || '';
    const token = tokenDeleteTarget;
    tokenDeleteTarget = null;
    try {
      await api.olt.deletePublicToken(id, token.id);
      toast.success('Token berhasil dihapus.');
      await loadTokens();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function copyToken(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      toast.success('Token disalin.');
    } catch {
      toast.error('Gagal menyalin token.');
    }
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={olt?.name || 'Detail OLT'}
    subtitle={olt ? `${friendlyOltType(olt.olt_type)} — ${olt.host}:${olt.port}` : ''}
  >
    {#snippet actions()}
      <a class="btn ghost" href={backTarget}>
        <Icon name="arrow-left" size={16} />
        Kembali
      </a>
      <button class="btn ghost" type="button" onclick={() => void refresh({ silent: false })} disabled={refreshing}>
        <Icon name="refresh-cw" size={16} />
        Refresh
      </button>
      <button class="btn" type="button" onclick={forceRefreshStats} disabled={refreshing}>
        <Icon name="zap" size={16} />
        Refresh Stats
      </button>
    {/snippet}
  </NetworkPageHeader>

  {#if initialLoading}
    <div class="loading-wrap">
      <Icon name="loader" size={24} />
      <span>Memuat data OLT...</span>
    </div>
  {:else if olt}
    <!-- Status bar -->
    <div class="status-bar">
      <span class="badge" class:online={olt.is_online} class:offline={!olt.is_online}>
        {olt.is_online ? 'Online' : 'Offline'}
      </span>
      {#if olt.last_seen_at}
        <span class="muted">Terakhir dilihat: {timeAgo(olt.last_seen_at)}</span>
      {/if}
      {#if olt.last_error}
        <span class="error-text">{olt.last_error}</span>
      {/if}
    </div>

    <!-- Tabs -->
    <div class="tabs">
      {#each tabItems as tab}
        <button
          class="tab"
          class:active={activeTab === tab.id}
          type="button"
          onclick={() => switchTab(tab.id)}
        >
          {tab.label}
          {#if tab.count != null}
            <span class="tab-count">{tab.count}</span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- Overview Tab -->
    {#if activeTab === 'overview'}
      <!-- Summary cards -->
      <div class="stat-grid">
        <div class="stat-card">
          <div class="stat-top"><span class="stat-label">ONU Total</span><Icon name="list" size={14} /></div>
          <div class="stat-value">{stats?.onu_summary?.total ?? onuStats.total}</div>
        </div>
        <div class="stat-card tone-ok">
          <div class="stat-top"><span class="stat-label">ONU Online</span><Icon name="check-circle" size={14} /></div>
          <div class="stat-value">{stats?.onu_summary?.online ?? onuStats.online}</div>
        </div>
        <div class="stat-card tone-bad">
          <div class="stat-top"><span class="stat-label">ONU Offline</span><Icon name="alert-circle" size={14} /></div>
          <div class="stat-value">{stats?.onu_summary?.offline ?? onuStats.offline}</div>
        </div>
        <div class="stat-card tone-warn">
          <div class="stat-top"><span class="stat-label">Sinyal Lemah</span><Icon name="alert-triangle" size={14} /></div>
          <div class="stat-value">{stats?.onu_summary?.low_signal ?? onuStats.low}</div>
        </div>
      </div>

      {#if stats?.system_info}
        <div class="section-card">
          <h4 class="section-title">Informasi Sistem</h4>
          <div class="info-grid">
            {#if stats.system_info.model}
              <div class="info-item">
                <span class="info-label">Model</span>
                <span class="info-value">{stats.system_info.model}</span>
              </div>
            {/if}
            {#if stats.system_info.serial_number}
              <div class="info-item">
                <span class="info-label">Serial Number</span>
                <span class="info-value mono">{stats.system_info.serial_number}</span>
              </div>
            {/if}
            {#if stats.system_info.firmware_version}
              <div class="info-item">
                <span class="info-label">Firmware</span>
                <span class="info-value mono">{stats.system_info.firmware_version}</span>
              </div>
            {/if}
            {#if stats.system_info.uptime}
              <div class="info-item">
                <span class="info-label">Uptime</span>
                <span class="info-value">{stats.system_info.uptime}</span>
              </div>
            {/if}
            {#each Object.entries(stats.system_info) as [key, val]}
              {#if !['model', 'serial_number', 'firmware_version', 'uptime'].includes(key) && val != null}
                <div class="info-item">
                  <span class="info-label">{key}</span>
                  <span class="info-value">{val}</span>
                </div>
              {/if}
            {/each}
          </div>
        </div>
      {/if}

      {#if stats?.pon_ports && stats.pon_ports.length > 0}
        <div class="section-card">
          <h4 class="section-title">Port PON</h4>
          <div class="table-wrap">
            <Table
              columns={[
                { key: 'port_name', label: 'Port' },
                { key: 'status', label: 'Status' },
                { key: 'onu_count', label: 'ONU' },
                { key: 'online_count', label: 'Online' },
                { key: 'offline_count', label: 'Offline' },
              ]}
              data={stats.pon_ports}
              loading={false}
              emptyText="Tidak ada port PON"
              mobileView={isMobile ? 'card' : 'scroll'}
            >
              {#snippet cell({ item, key }: any)}
                {#if key === 'port_name'}
                  <span class="name">{item.port_name}</span>
                {:else if key === 'status'}
                  <span class="badge" class:online={item.status === 'active' || item.status === 'online'} class:offline={item.status !== 'active' && item.status !== 'online'}>
                    {item.status || '—'}
                  </span>
                {:else}
                  <span class="mono">{item[key] ?? '—'}</span>
                {/if}
              {/snippet}
            </Table>
          </div>
        </div>
      {/if}
    {/if}

    <!-- ONU Tab -->
    {#if activeTab === 'onus'}
      <div class="toolbar">
        <div class="search">
          <Icon name="search" size={16} />
          <input class="search-input" bind:value={onuSearch} placeholder="Cari ONU..." />
          {#if onuSearch}
            <button class="clear" type="button" onclick={() => (onuSearch = '')}>
              <Icon name="x" size={14} />
            </button>
          {/if}
        </div>
      </div>

      <div class="table-wrap">
        <Table
          columns={onuColumns}
          data={filteredOnus}
          loading={false}
          emptyText="Tidak ada ONU"
          mobileView={isMobile ? 'card' : 'scroll'}
        >
          {#snippet cell({ item, key }: any)}
            {#if key === 'onu'}
              <div class="name-cell">
                <span class="name">{item.onu_name || item.onu_id}</span>
                <div class="muted">ID: {item.onu_id}</div>
                {#if item.serial_number}
                  <div class="muted mono">SN: {item.serial_number}</div>
                {/if}
                {#if item.model}
                  <div class="muted">{item.model}</div>
                {/if}
              </div>
            {:else if key === 'port'}
              <span class="mono">{item.pon_port || '—'}</span>
            {:else if key === 'status'}
              <span class="badge" class:online={item.status === 'online'} class:offline={item.status !== 'online'}>
                {item.status || '—'}
              </span>
            {:else if key === 'signal'}
              {#if item.rx_power != null}
                <div class="signal-cell">
                  <span class="signal-value" style="color: {signalColor(item.rx_power)}">
                    {item.rx_power.toFixed(2)}
                  </span>
                  <span class="signal-label" style="color: {signalColor(item.rx_power)}">
                    {signalLabel(item.rx_power)}
                  </span>
                </div>
              {:else}
                <span class="muted">—</span>
              {/if}
            {:else if key === 'distance'}
              {#if item.distance_m != null}
                <span class="mono">{item.distance_m}m</span>
              {:else}
                <span class="muted">—</span>
              {/if}
            {:else if key === 'uptime'}
              <span class="muted">{formatUptime(item.uptime_seconds)}</span>
            {:else if key === 'actions'}
              {#if $can('manage', 'router_inventory')}
                <button class="icon-btn danger" type="button" onclick={() => promptReboot(item)} title="Reboot ONU">
                  <Icon name="power" size={14} />
                </button>
              {/if}
            {/if}
          {/snippet}
        </Table>
      </div>
    {/if}

    <!-- History Tab -->
    {#if activeTab === 'history'}
      <div class="table-wrap">
        <Table
          columns={historyColumns}
          data={history}
          loading={historyLoading}
          emptyText="Tidak ada riwayat"
          mobileView={isMobile ? 'card' : 'scroll'}
        >
          {#snippet cell({ item, key }: any)}
            {#if key === 'time'}
              <span class="muted" title={formatDateTime(item.created_at, { timeZone: $appSettings.app_timezone })}>
                {timeAgo(item.created_at)}
              </span>
            {:else if key === 'onu'}
              <span class="name">{item.onu_name || item.onu_id}</span>
            {:else if key === 'event'}
              <span class="chip">{item.event_type}</span>
            {:else if key === 'message'}
              <span class="muted">{item.message || '—'}</span>
            {/if}
          {/snippet}
        </Table>
      </div>
    {/if}

    <!-- Tokens Tab -->
    {#if activeTab === 'tokens'}
      <div class="toolbar">
        <div></div>
        {#if $can('manage', 'router_inventory')}
          <button class="btn" type="button" onclick={openTokenModal}>
            <Icon name="plus" size={16} />
            Buat Token
          </button>
        {/if}
      </div>

      <div class="table-wrap">
        <Table
          columns={tokenColumns}
          data={publicTokens}
          loading={tokensLoading}
          emptyText="Tidak ada token publik"
          mobileView={isMobile ? 'card' : 'scroll'}
        >
          {#snippet cell({ item, key }: any)}
            {#if key === 'token'}
              <div class="token-cell">
                <code class="token-value">{item.token}</code>
                <button class="icon-btn mini" type="button" onclick={() => copyToken(item.token)} title="Salin">
                  <Icon name="copy" size={12} />
                </button>
              </div>
            {:else if key === 'description'}
              <span class="muted">{item.description || '—'}</span>
            {:else if key === 'status'}
              <span class="badge" class:online={item.enabled} class:offline={!item.enabled}>
                {item.enabled ? 'Aktif' : 'Nonaktif'}
              </span>
            {:else if key === 'expires'}
              {#if item.expires_at}
                <span class="muted">{formatDateTime(item.expires_at, { timeZone: $appSettings.app_timezone })}</span>
              {:else}
                <span class="muted">Tidak ada</span>
              {/if}
            {:else if key === 'actions'}
              {#if $can('manage', 'router_inventory')}
                <button class="icon-btn danger" type="button" onclick={() => promptDeleteToken(item)} title="Hapus Token">
                  <Icon name="trash-2" size={14} />
                </button>
              {/if}
            {/if}
          {/snippet}
        </Table>
      </div>
    {/if}
  {:else}
    <div class="empty-state">
      <Icon name="alert-circle" size={32} />
      <p>OLT tidak ditemukan.</p>
    </div>
  {/if}
</div>

<!-- Token Modal -->
{#if showTokenModal}
  <div class="modal-backdrop" onclick={() => (showTokenModal = false)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>Buat Token Publik</h3>
        <button class="icon-btn" type="button" onclick={() => (showTokenModal = false)}>
          <Icon name="x" size={18} />
        </button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label for="tok-desc">Deskripsi</label>
          <input id="tok-desc" type="text" bind:value={tokenDesc} placeholder="Monitoring eksternal" />
        </div>
        <div class="form-group">
          <label for="tok-exp">Kadaluarsa</label>
          <input id="tok-exp" type="datetime-local" bind:value={tokenExpiry} />
        </div>
        <div class="form-group row">
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={tokenEnabled} />
            Aktif
          </label>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn ghost" type="button" onclick={() => (showTokenModal = false)}>Batal</button>
        <button class="btn" type="button" onclick={createToken} disabled={savingToken}>
          {savingToken ? 'Menyimpan...' : 'Buat Token'}
        </button>
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:show={showRebootConfirm}
  title="Konfirmasi Reboot ONU"
  message={rebootTarget ? `Yakin ingin me-reboot ONU "${rebootTarget.onu_name || rebootTarget.onu_id}"? ONU akan restart.` : ''}
  confirmText="Reboot"
  cancelText="Batal"
  type="danger"
  onconfirm={confirmReboot}
  oncancel={() => { rebootTarget = null; }}
/>

<ConfirmDialog
  bind:show={showTokenDeleteConfirm}
  title="Konfirmasi Hapus Token"
  message="Yakin ingin menghapus token ini? Akses publik akan dicabut."
  confirmText="Hapus"
  cancelText="Batal"
  type="danger"
  onconfirm={confirmDeleteToken}
  oncancel={() => { tokenDeleteTarget = null; }}
/>

<style>
  .page-content {
    padding: 24px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 9px 13px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: white;
    font-weight: 700;
    cursor: pointer;
    transition: transform 0.12s ease, filter 0.12s ease;
    text-decoration: none;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:hover { filter: brightness(1.05); }
  .btn:disabled { opacity: 0.6; cursor: not-allowed; }

  .loading-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px;
    color: var(--text-secondary);
  }

  .status-bar {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 16px;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 9px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.74rem;
    border: 1px solid var(--border-color);
  }

  .badge.online {
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
    border-color: rgba(34, 197, 94, 0.28);
  }

  .badge.offline {
    background: rgba(239, 68, 68, 0.12);
    color: rgba(239, 68, 68, 0.95);
    border-color: rgba(239, 68, 68, 0.28);
  }

  .muted {
    color: var(--text-secondary);
    font-size: 0.86rem;
  }

  .error-text {
    color: #ef4444;
    font-size: 0.86rem;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
    color: var(--text-primary);
  }

  /* Tabs */
  .tabs {
    display: flex;
    gap: 2px;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--border-color);
    overflow-x: auto;
  }

  .tab {
    padding: 10px 16px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-weight: 700;
    font-size: 0.86rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    border-bottom: 2px solid transparent;
    white-space: nowrap;
  }

  .tab:hover { color: var(--text-primary); }
  .tab.active {
    color: var(--text-primary);
    border-bottom-color: var(--color-primary);
  }

  .tab-count {
    font-size: 0.72rem;
    font-weight: 900;
    background: var(--bg-hover);
    border: 1px solid var(--border-color);
    padding: 1px 6px;
    border-radius: 999px;
  }

  /* Stat cards */
  .stat-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 16px;
  }

  .stat-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 12px 13px 11px;
  }

  .stat-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-secondary);
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .stat-value {
    margin-top: 8px;
    font-size: 1.42rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .tone-ok { box-shadow: 0 0 0 1px rgba(34, 197, 94, 0.15) inset; }
  .tone-bad { box-shadow: 0 0 0 1px rgba(239, 68, 68, 0.16) inset; }
  .tone-warn { box-shadow: 0 0 0 1px rgba(245, 158, 11, 0.15) inset; }

  /* Section cards */
  .section-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 16px 20px;
    margin-bottom: 16px;
  }

  .section-title {
    margin: 0 0 12px;
    font-size: 0.9rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }

  .info-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .info-label {
    font-size: 0.76rem;
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .info-value {
    font-size: 0.92rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  /* Table */
  .toolbar {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 9px 11px;
    min-width: min(500px, 100%);
    color: var(--text-secondary);
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .clear {
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--text-secondary);
    display: grid;
    place-items: center;
  }

  .table-wrap {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .name-cell .name {
    font-weight: 900;
    color: var(--text-primary);
    display: block;
  }

  .chip {
    font-size: 0.7rem;
    font-weight: 800;
    padding: 2px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-hover), transparent 20%);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  .signal-cell {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .signal-value {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-weight: 900;
    font-size: 0.88rem;
  }

  .signal-label {
    font-size: 0.72rem;
    font-weight: 700;
  }

  .token-cell {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .token-value {
    font-size: 0.82rem;
    background: var(--bg-hover);
    padding: 2px 6px;
    border-radius: 6px;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icon-btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 10px;
    padding: 7px;
    cursor: pointer;
    display: grid;
    place-items: center;
  }

  .icon-btn:hover { background: var(--bg-hover); }
  .icon-btn.danger { color: rgba(239, 68, 68, 0.95); border-color: rgba(239, 68, 68, 0.28); }
  .icon-btn.mini { padding: 4px; border-radius: 6px; }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px;
    color: var(--text-secondary);
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: grid;
    place-items: center;
    z-index: 1000;
    padding: 24px;
  }

  .modal {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    width: 100%;
    max-width: 440px;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .form-group label {
    font-size: 0.82rem;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .form-group input {
    padding: 9px 11px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.9rem;
    outline: none;
  }

  .form-group input:focus {
    border-color: var(--color-primary);
  }

  .form-group.row {
    flex-direction: row;
    align-items: center;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.9rem;
    cursor: pointer;
    color: var(--text-primary);
  }

  @media (max-width: 900px) {
    .page-content { padding: 18px; }
    .stat-grid { grid-template-columns: repeat(2, 1fr); }
    .search { min-width: 0; width: 100%; }
    .info-grid { grid-template-columns: 1fr; }
  }

  @media (max-width: 640px) {
    .stat-grid { grid-template-columns: 1fr; }
  }
</style>
