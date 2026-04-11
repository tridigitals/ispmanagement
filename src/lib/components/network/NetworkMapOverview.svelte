<script lang="ts">
  import type {
    NetworkMapSearchResultGroup,
    NetworkMapSearchResultItem,
  } from '$lib/components/network/networkMapInsights';
  import Icon from '$lib/components/ui/Icon.svelte';
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
    workspaceSearchQuery,
    title,
    subtitle,
    labels,
    searchGroups,
    searchSummary,
    onWorkspaceSearchChange,
    onWorkspaceSearchSelect,
    onSyncAssets,
    onRefresh,
  }: {
    compactMode: boolean;
    fromInstallation: boolean;
    installationReturnUrl: string;
    tenantPrefix: string;
    canManageTopology: boolean;
    syncingAssetNodes: boolean;
    refreshing: boolean;
    loading: boolean;
    workspaceSearchQuery: string;
    title: string;
    subtitle: string;
    labels: Record<string, string>;
    searchGroups: NetworkMapSearchResultGroup[];
    searchSummary: string;
    onWorkspaceSearchChange: (value: string) => void;
    onWorkspaceSearchSelect: (item: NetworkMapSearchResultItem) => void;
    onSyncAssets: () => void;
    onRefresh: () => void;
  } = $props();
</script>

{#if !compactMode}
  <div class="overview-shell">
    <NetworkPageHeader {title} {subtitle}>
      {#snippet actions()}
        {#if fromInstallation}
          <a class="btn btn-compact ghost" href={installationReturnUrl}>
            <Icon name="arrow-left" size={16} />
            {labels.backToInstallation}
          </a>
        {/if}
        <a class="btn btn-compact ghost" href={`${tenantPrefix}/admin/network/noc`}>
          <Icon name="arrow-left" size={16} />
          {labels.backToNoc}
        </a>
        {#if canManageTopology}
          <button
            class="btn btn-compact ghost"
            type="button"
            onclick={onSyncAssets}
            disabled={syncingAssetNodes || refreshing || loading}
          >
            <Icon name="git-merge" size={16} />
            {syncingAssetNodes ? labels.syncing : labels.syncAssets}
          </button>
        {/if}
        <button
          class="btn btn-compact"
          type="button"
          onclick={onRefresh}
          disabled={refreshing || loading}
        >
          <Icon name="refresh-cw" size={16} />
          {refreshing ? labels.loading : labels.refresh}
        </button>
      {/snippet}
    </NetworkPageHeader>

    <div class="workspace-composer">
      <div class="search-section">
        <div class="section-head compact-head">
          <div class="section-heading">
            <div class="section-kicker">{labels.searchKicker}</div>
            <div class="section-title">{labels.searchTitle}</div>
          </div>
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
    </div>
  </div>
{/if}

<style>
  .overview-shell {
    display: grid;
    gap: 12px;
  }

  .workspace-composer {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 12px;
    align-items: start;
    padding: 14px 16px;
    border-radius: 20px;
    border: 1px solid color-mix(in srgb, var(--border-color) 78%, transparent);
    background:
      radial-gradient(circle at top right, rgba(23, 37, 84, 0.28), transparent 34%),
      linear-gradient(180deg, color-mix(in srgb, var(--bg-card) 94%, #050d18 6%), var(--bg-card));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.04),
      0 30px 60px rgba(2, 6, 23, 0.12);
  }

  .section-kicker {
    font-size: 0.68rem;
    font-weight: 900;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-primary) 72%, white 28%);
  }

  .section-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .section-heading {
    display: grid;
    gap: 3px;
  }

  .search-section {
    display: grid;
    gap: 10px;
    min-width: 0;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .compact-head {
    min-height: 0;
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

  .btn-compact {
    min-height: 38px;
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 0.84rem;
    line-height: 1;
    white-space: nowrap;
  }

  .btn-compact :global(svg) {
    width: 14px;
    height: 14px;
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
      grid-template-columns: 1fr;
      padding: 12px;
      border-radius: 18px;
    }

    .section-head {
      align-items: start;
    }

    .section-title {
      font-size: 0.9rem;
    }

    .btn {
      width: 100%;
      justify-content: center;
      min-height: 40px;
    }

    .btn-compact {
      width: 100%;
      min-height: 38px;
    }
  }
</style>
