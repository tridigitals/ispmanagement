<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';

  let {
    compactMode,
    fromInstallation,
    installationReturnUrl,
    tenantPrefix,
    canManageTopology,
    syncingAssetNodes,
    refreshing,
    loading,
    nodeCount,
    linkCount,
    zoneCount,
    q,
    status,
    kind,
    nodesVisible,
    linksVisible,
    zonesVisible,
    routersVisible,
    customersVisible,
    myLocationError,
    title,
    subtitle,
    labels,
    onQChange,
    onStatusChange,
    onKindChange,
    onApplyFilters,
    onResetFilters,
    onSyncAssets,
    onRefresh,
    onNodesVisibleChange,
    onLinksVisibleChange,
    onZonesVisibleChange,
    onRoutersVisibleChange,
    onCustomersVisibleChange,
  }: {
    compactMode: boolean;
    fromInstallation: boolean;
    installationReturnUrl: string;
    tenantPrefix: string;
    canManageTopology: boolean;
    syncingAssetNodes: boolean;
    refreshing: boolean;
    loading: boolean;
    nodeCount: number;
    linkCount: number;
    zoneCount: number;
    q: string;
    status: string;
    kind: string;
    nodesVisible: boolean;
    linksVisible: boolean;
    zonesVisible: boolean;
    routersVisible: boolean;
    customersVisible: boolean;
    myLocationError: string;
    title: string;
    subtitle: string;
    labels: Record<string, string>;
    onQChange: (value: string) => void;
    onStatusChange: (value: string) => void;
    onKindChange: (value: string) => void;
    onApplyFilters: () => void;
    onResetFilters: () => void;
    onSyncAssets: () => void;
    onRefresh: () => void;
    onNodesVisibleChange: (checked: boolean) => void;
    onLinksVisibleChange: (checked: boolean) => void;
    onZonesVisibleChange: (checked: boolean) => void;
    onRoutersVisibleChange: (checked: boolean) => void;
    onCustomersVisibleChange: (checked: boolean) => void;
  } = $props();
</script>

{#if !compactMode}
  <NetworkPageHeader {title} {subtitle}>
    {#snippet actions()}
      {#if fromInstallation}
        <a class="btn ghost" href={installationReturnUrl}>
          <Icon name="arrow-left" size={16} />
          {labels.backToInstallation}
        </a>
      {/if}
      <a class="btn ghost" href={`${tenantPrefix}/admin/network/noc`}>
        <Icon name="arrow-left" size={16} />
        {labels.backToNoc}
      </a>
      {#if canManageTopology}
        <button
          class="btn ghost"
          type="button"
          onclick={onSyncAssets}
          disabled={syncingAssetNodes || refreshing || loading}
        >
          <Icon name="git-merge" size={16} />
          {syncingAssetNodes ? labels.syncing : labels.syncAssets}
        </button>
      {/if}
      <button class="btn" type="button" onclick={onRefresh} disabled={refreshing || loading}>
        <Icon name="refresh-cw" size={16} />
        {refreshing ? labels.loading : labels.refresh}
      </button>
    {/snippet}
  </NetworkPageHeader>

  <div class="stats">
    <div class="stat-card">
      <div class="stat-top">
        <span>{labels.nodes}</span>
        <Icon name="map-pin" size={16} />
      </div>
      <div class="stat-value">{nodeCount}</div>
    </div>
    <div class="stat-card tone-ok">
      <div class="stat-top">
        <span>{labels.links}</span>
        <Icon name="git-merge" size={16} />
      </div>
      <div class="stat-value">{linkCount}</div>
    </div>
    <div class="stat-card tone-warn">
      <div class="stat-top">
        <span>{labels.zones}</span>
        <Icon name="layers" size={16} />
      </div>
      <div class="stat-value">{zoneCount}</div>
    </div>
  </div>

  <div class="filters-wrap">
    <NetworkFilterPanel>
      <div class="control">
        <label for="nm-search">{labels.search}</label>
        <input
          id="nm-search"
          class="input"
          type="text"
          value={q}
          placeholder={labels.searchPlaceholder}
          oninput={(e) => onQChange((e.currentTarget as HTMLInputElement).value)}
          onkeydown={(e) => e.key === 'Enter' && onApplyFilters()}
        />
      </div>

      <div class="control">
        <label for="nm-status">{labels.status}</label>
        <select id="nm-status" class="input" value={status} onchange={(e) => onStatusChange((e.currentTarget as HTMLSelectElement).value)}>
          <option value="">{labels.anyStatus}</option>
          <option value="active">Active</option>
          <option value="inactive">Inactive</option>
          <option value="maintenance">Maintenance</option>
          <option value="up">Up</option>
          <option value="down">Down</option>
          <option value="degraded">Degraded</option>
        </select>
      </div>

      <div class="control">
        <label for="nm-kind">{labels.kind}</label>
        <select id="nm-kind" class="input" value={kind} onchange={(e) => onKindChange((e.currentTarget as HTMLSelectElement).value)}>
          <option value="">{labels.anyKind}</option>
          <option value="core">Core</option>
          <option value="pop">POP</option>
          <option value="olt">OLT</option>
          <option value="router">Router</option>
          <option value="switch">Switch</option>
          <option value="tower">Tower</option>
          <option value="ap">AP</option>
          <option value="odc">ODC</option>
          <option value="odp">ODP</option>
          <option value="splitter">Splitter</option>
          <option value="junction">Junction</option>
          <option value="customer_premise">Customer Premise</option>
          <option value="fiber">Fiber</option>
          <option value="lan">LAN</option>
          <option value="wireless">Wireless</option>
          <option value="ptp_radio">PTP Radio</option>
        </select>
      </div>

      <div class="control control-actions">
        <div class="control-spacer" aria-hidden="true"></div>
        <button class="btn" type="button" onclick={onApplyFilters} disabled={refreshing || loading}>
          <Icon name="check" size={14} />
          {labels.apply}
        </button>
        <button class="btn ghost" type="button" onclick={onResetFilters} disabled={refreshing || loading}>
          <Icon name="x-circle" size={14} />
          {labels.reset}
        </button>
      </div>
    </NetworkFilterPanel>
  </div>

  <div class="toolbar-wrap">
    <div class="map-toolbar">
      <div class="layer-toggles">
        <label class="toggle">
          <input type="checkbox" checked={nodesVisible} onchange={(e) => onNodesVisibleChange((e.currentTarget as HTMLInputElement).checked)} />
          <span>{labels.nodes}</span>
        </label>
        <label class="toggle">
          <input type="checkbox" checked={linksVisible} onchange={(e) => onLinksVisibleChange((e.currentTarget as HTMLInputElement).checked)} />
          <span>{labels.links}</span>
        </label>
        <label class="toggle">
          <input type="checkbox" checked={zonesVisible} onchange={(e) => onZonesVisibleChange((e.currentTarget as HTMLInputElement).checked)} />
          <span>{labels.zones}</span>
        </label>
        <label class="toggle">
          <input type="checkbox" checked={routersVisible} onchange={(e) => onRoutersVisibleChange((e.currentTarget as HTMLInputElement).checked)} />
          <span>Routers</span>
        </label>
        <label class="toggle">
          <input type="checkbox" checked={customersVisible} onchange={(e) => onCustomersVisibleChange((e.currentTarget as HTMLInputElement).checked)} />
          <span>Customers</span>
        </label>
      </div>
    </div>

    {#if myLocationError}
      <div class="location-error">
        <Icon name="alert-triangle" size={14} />
        <span>{myLocationError}</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .filters-wrap {
    margin-bottom: 12px;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }

  .stat-card {
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--bg-card) 86%, #16213f 14%) 0%,
        var(--bg-card) 100%
      );
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 14px 14px 12px;
    box-shadow: inset 0 1px 0 rgba(148, 163, 184, 0.08);
  }

  .stat-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-secondary);
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .stat-value {
    margin-top: 10px;
    font-size: 1.6rem;
    font-weight: 950;
    color: var(--text-primary);
  }

  .tone-ok {
    border-color: color-mix(in srgb, #1fbf75 55%, var(--border-color));
  }

  .tone-warn {
    border-color: color-mix(in srgb, #ffcc66 55%, var(--border-color));
  }

  .toolbar-wrap {
    display: grid;
    gap: 8px;
  }

  .map-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px 12px;
    background: var(--bg-card);
    box-shadow: inset 0 1px 0 rgba(148, 163, 184, 0.08);
  }

  .layer-toggles {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.86rem;
    color: var(--text-secondary);
  }

  .control {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .control label {
    font-size: 0.78rem;
    color: #cbd5e1;
    font-weight: 700;
  }

  .input {
    width: 100%;
    border: 1px solid #334155;
    border-radius: 10px;
    background: #111827;
    color: #e5e7eb;
    padding: 10px 12px;
    font-size: 0.9rem;
    outline: none;
  }

  .input:focus {
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-color));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 22%, transparent);
  }

  .location-error {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: #fbbf24;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: white;
    font-weight: 800;
    cursor: pointer;
    text-decoration: none;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }
</style>
