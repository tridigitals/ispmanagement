<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import type { NetworkAssetListItem } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { getNetworkAssetDetailSummary } from '$lib/utils/networkAssetDetails';
  import {
    getNetworkAssetGroupLabel,
    getNetworkAssetTypeLabel,
  } from '$lib/utils/networkAssetTypes';

  type Props = {
    assets?: NetworkAssetListItem[];
    loading?: boolean;
  };

  let { assets = [], loading = false }: Props = $props();

  function openAssetsPage() {
    goto(`${$page.url.pathname.split('/customers/')[0]}/network/assets`);
  }
</script>

<section class="card section asset-panel">
  <div class="asset-panel__head">
    <div>
      <h3>{$t('admin.customers.tabs.assets')}</h3>
      <p class="muted subtitle">
        {$t('admin.customers.assets.subtitle')}
      </p>
    </div>

    <button class="btn btn-secondary" type="button" onclick={openAssetsPage}>
      <Icon name="package" size={16} />
      {$t('admin.customers.assets.open_registry')}
    </button>
  </div>

  {#if loading}
    <div class="empty">{$t('common.loading')}</div>
  {:else if assets.length === 0}
    <div class="empty">
      {$t('admin.customers.assets.empty')}
    </div>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>{$t('admin.customers.assets.columns.name')}</th>
            <th>{$t('admin.customers.assets.columns.type')}</th>
            <th>{$t('admin.customers.assets.columns.serial')}</th>
            <th>{$t('admin.customers.assets.columns.status')}</th>
            <th>{$t('admin.customers.assets.columns.location')}</th>
            <th>{$t('admin.customers.assets.columns.parent')}</th>
            <th>{$t('admin.customers.assets.columns.updated')}</th>
          </tr>
        </thead>
        <tbody>
          {#each assets as asset}
            <tr>
              <td>
                <div class="asset-name">
                  <strong>{asset.name}</strong>
                  {#if asset.code}
                    <span class="muted mono">{asset.code}</span>
                  {/if}
                </div>
              </td>
              <td>
                <div class="asset-type">
                  <strong>{getNetworkAssetTypeLabel(asset.asset_type)}</strong>
                  <span class="muted">{getNetworkAssetGroupLabel(asset.asset_group)}</span>
                  {#if getNetworkAssetDetailSummary(asset).length > 0}
                    <span class="muted">{getNetworkAssetDetailSummary(asset).join(' • ')}</span>
                  {/if}
                </div>
              </td>
              <td>{asset.serial_number || '—'}</td>
              <td><span class="pill">{asset.status}</span></td>
              <td>{asset.location_label || '—'}</td>
              <td>{asset.parent_asset_name || '—'}</td>
              <td>{asset.updated_at}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>

<style>
  .section {
    padding: 1.1rem;
    background: var(--bg-surface);
  }

  .asset-panel {
    display: grid;
    gap: 1rem;
  }

  .asset-panel__head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }

  .muted {
    color: var(--text-secondary);
  }

  .subtitle {
    margin-top: 0.25rem;
  }

  .mono {
    font-family: var(--font-mono, monospace);
  }

  .btn {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    min-height: 42px;
    padding: 0.55rem 0.9rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    cursor: pointer;
    font-weight: 650;
    font-size: 0.9rem;
  }

  .empty {
    padding: 1rem;
    border: 1px dashed var(--border-subtle);
    color: var(--text-secondary);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
  }

  .table-wrap {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 0.85rem 0.75rem;
    border-bottom: 1px solid var(--border-subtle);
    text-align: left;
    vertical-align: top;
  }

  .asset-name {
    display: grid;
    gap: 0.25rem;
  }
  .asset-type {
    display: grid;
    gap: 0.2rem;
  }

  .pill {
    display: inline-flex;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    background: var(--bg-soft);
    border: 1px solid var(--border-subtle);
    font-size: 0.82rem;
  }

  @media (max-width: 900px) {
    .asset-panel__head {
      flex-direction: column;
      align-items: stretch;
    }

    .asset-panel__head .btn {
      width: 100%;
    }
  }
</style>
