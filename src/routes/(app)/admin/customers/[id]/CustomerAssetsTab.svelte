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

<section class="asset-panel">
  <div class="asset-panel__head">
    <div>
      <h3>{$t('admin.customers.tabs.assets') || 'FTTH Assets'}</h3>
      <p class="muted">
        {$t('admin.customers.assets.subtitle') || 'Perangkat FTTH yang terhubung ke pelanggan ini.'}
      </p>
    </div>

    <button class="btn ghost" type="button" onclick={openAssetsPage}>
      <Icon name="box" size={16} />
      {$t('admin.customers.assets.open_registry') || 'Open registry'}
    </button>
  </div>

  {#if loading}
    <div class="empty">{$t('common.loading') || 'Loading...'}</div>
  {:else if assets.length === 0}
    <div class="empty">
      {$t('admin.customers.assets.empty') || 'Belum ada asset FTTH yang ditautkan ke pelanggan ini.'}
    </div>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>{$t('admin.customers.assets.columns.name') || 'Asset'}</th>
            <th>{$t('admin.customers.assets.columns.type') || 'Type'}</th>
            <th>{$t('admin.customers.assets.columns.serial') || 'Serial'}</th>
            <th>{$t('admin.customers.assets.columns.status') || 'Status'}</th>
            <th>{$t('admin.customers.assets.columns.location') || 'Location'}</th>
            <th>{$t('admin.customers.assets.columns.parent') || 'Parent'}</th>
            <th>{$t('admin.customers.assets.columns.updated') || 'Updated'}</th>
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

  .mono {
    font-family: var(--font-mono, monospace);
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
</style>
