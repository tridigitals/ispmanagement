<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
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
    title,
    subtitle,
    labels,
    onSyncAssets,
  }: {
    compactMode: boolean;
    fromInstallation: boolean;
    installationReturnUrl: string;
    tenantPrefix: string;
    canManageTopology: boolean;
    syncingAssetNodes: boolean;
    refreshing: boolean;
    loading: boolean;
    title: string;
    subtitle: string;
    labels: Record<string, string>;
    onSyncAssets: () => void;
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
            <Icon name="refresh-cw" size={16} />
            {syncingAssetNodes || refreshing ? labels.syncing : labels.sync}
          </button>
        {/if}
      {/snippet}
    </NetworkPageHeader>
  </div>
{/if}

<style>
  .overview-shell {
    display: grid;
    gap: 8px;
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
