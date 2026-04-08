<script lang="ts">
  import type {
    NetworkMapInsightCard,
    NetworkMapSearchResultGroup,
    NetworkMapSearchResultItem,
  } from '$lib/components/network/networkMapInsights';
  import type { NetworkMapQuickModeOption } from '$lib/components/network/NetworkMapQuickModes.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkMapInsightStrip from '$lib/components/network/NetworkMapInsightStrip.svelte';
  import NetworkMapQuickModes from '$lib/components/network/NetworkMapQuickModes.svelte';
  import NetworkMapSearchBar from '$lib/components/network/NetworkMapSearchBar.svelte';
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
    filterQuery,
    workspaceSearchQuery,
    status,
    kind,
    nodesVisible,
    linksVisible,
    zonesVisible,
    routersVisible,
    showRoutersToggle = true,
    customersVisible,
    myLocationError,
    title,
    subtitle,
    labels,
    insightCards,
    insightScopeLabel,
    searchGroups,
    searchSummary,
    quickModes,
    activeQuickMode,
    onFilterQueryChange,
    onWorkspaceSearchChange,
    onWorkspaceSearchSelect,
    onStatusChange,
    onKindChange,
    onQuickModeSelect,
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
    filterQuery: string;
    workspaceSearchQuery: string;
    status: string;
    kind: string;
    nodesVisible: boolean;
    linksVisible: boolean;
    zonesVisible: boolean;
    routersVisible: boolean;
    showRoutersToggle?: boolean;
    customersVisible: boolean;
    myLocationError: string;
    title: string;
    subtitle: string;
    labels: Record<string, string>;
    insightCards: NetworkMapInsightCard[];
    insightScopeLabel: string;
    searchGroups: NetworkMapSearchResultGroup[];
    searchSummary: string;
    quickModes: readonly NetworkMapQuickModeOption[];
    activeQuickMode: string;
    onFilterQueryChange: (value: string) => void;
    onWorkspaceSearchChange: (value: string) => void;
    onWorkspaceSearchSelect: (item: NetworkMapSearchResultItem) => void;
    onStatusChange: (value: string) => void;
    onKindChange: (value: string) => void;
    onQuickModeSelect: (key: string) => void;
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
  <div class="overview-shell">
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

    <div class="workspace-composer">
      <div class="workspace-intro">
        <div class="workspace-kicker">{labels.workspaceKicker}</div>
        <h2 class="workspace-title">{labels.workspaceTitle}</h2>
        <p class="workspace-copy">{labels.workspaceCopy}</p>
      </div>

      <NetworkMapInsightStrip
        cards={insightCards}
        scopeLabel={insightScopeLabel}
        emptyLabel={labels.insightEmpty}
      />

      <div class="search-section">
        <div class="section-head">
          <div>
            <div class="section-kicker">{labels.searchKicker}</div>
            <div class="section-title">{labels.searchTitle}</div>
          </div>
          <div class="section-copy">{labels.searchHint}</div>
        </div>

        <NetworkMapSearchBar
          query={workspaceSearchQuery}
          groups={searchGroups}
          summary={searchSummary}
          placeholder={labels.searchPlaceholder}
          emptyTitle={labels.searchEmptyTitle}
          emptyHint={labels.searchEmptyHint}
          onQueryChange={onWorkspaceSearchChange}
          onSelect={onWorkspaceSearchSelect}
        />
      </div>

      <div class="quick-mode-section">
        <div class="section-head">
          <div>
            <div class="section-kicker">{labels.quickModesKicker}</div>
            <div class="section-title">{labels.quickModesTitle}</div>
          </div>
          <div class="section-copy">{labels.quickModesHint}</div>
        </div>

        <NetworkMapQuickModes
          modes={quickModes}
          activeKey={activeQuickMode}
          onSelect={onQuickModeSelect}
        />
      </div>
    </div>

    <div class="secondary-controls">
      <NetworkFilterPanel>
        <div class="control">
          <label for="nm-filter-search">{labels.filterSearch}</label>
          <input
            id="nm-filter-search"
            class="input"
            type="text"
            value={filterQuery}
            placeholder={labels.filterSearchPlaceholder}
            oninput={(event) =>
              onFilterQueryChange((event.currentTarget as HTMLInputElement).value)}
            onkeydown={(event) => event.key === 'Enter' && onApplyFilters()}
          />
        </div>

        <div class="control">
          <label for="nm-status">{labels.status}</label>
          <select
            id="nm-status"
            class="input"
            value={status}
            onchange={(event) => onStatusChange((event.currentTarget as HTMLSelectElement).value)}
          >
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
          <select
            id="nm-kind"
            class="input"
            value={kind}
            onchange={(event) => onKindChange((event.currentTarget as HTMLSelectElement).value)}
          >
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
          <button
            class="btn"
            type="button"
            onclick={onApplyFilters}
            disabled={refreshing || loading}
          >
            <Icon name="check" size={14} />
            {labels.apply}
          </button>
          <button
            class="btn ghost"
            type="button"
            onclick={onResetFilters}
            disabled={refreshing || loading}
          >
            <Icon name="x-circle" size={14} />
            {labels.reset}
          </button>
        </div>
      </NetworkFilterPanel>

      {#if myLocationError}
        <div class="location-error">
          <Icon name="alert-triangle" size={14} />
          <span>{myLocationError}</span>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overview-shell {
    display: grid;
    gap: 16px;
  }

  .workspace-composer {
    display: grid;
    gap: 16px;
    padding: 18px;
    border-radius: 24px;
    border: 1px solid color-mix(in srgb, var(--border-color) 78%, transparent);
    background:
      radial-gradient(circle at top right, rgba(23, 37, 84, 0.28), transparent 34%),
      linear-gradient(180deg, color-mix(in srgb, var(--bg-card) 94%, #050d18 6%), var(--bg-card));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.04),
      0 30px 60px rgba(2, 6, 23, 0.12);
  }

  .workspace-intro {
    display: grid;
    gap: 8px;
  }

  .workspace-kicker,
  .section-kicker {
    font-size: 0.72rem;
    font-weight: 900;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-primary) 72%, white 28%);
  }

  .workspace-title,
  .section-title {
    margin: 0;
    font-size: 1.02rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .workspace-copy,
  .section-copy {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .search-section,
  .quick-mode-section {
    display: grid;
    gap: 12px;
  }

  .section-head {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 14px;
    flex-wrap: wrap;
  }

  .secondary-controls {
    display: grid;
    gap: 12px;
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

  @media (max-width: 768px) {
    .workspace-composer {
      padding: 14px;
      border-radius: 20px;
    }

    .section-head {
      align-items: start;
    }

    .workspace-title,
    .section-title {
      font-size: 0.96rem;
    }

    .workspace-copy,
    .section-copy {
      font-size: 0.84rem;
    }

    .btn {
      width: 100%;
      justify-content: center;
      min-height: 40px;
    }
  }
</style>
