<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import type { CustomerLocation, PppoeAccountPublic } from '$lib/api/client';

  let {
    pppoeToolbar,
    pppoeQuery = $bindable(),
    onRefresh,
    loadingPppoe,
    pppoeColumns,
    pppoeAccounts,
    pppoeRouters,
    locations,
    getPppoeSyncDisplay,
    getPppoeAccessState,
    getPppoeProvisioningTargetFallback,
    getPppoeApplyActionFallback,
    timeAgo,
    canManagePppoe,
    onApplyPppoe,
    onEditPppoe,
    onDeletePppoe,
  } = $props();
</script>

<div class="card section">
  <div class="section-head">
    <div>
      <h3>{$t('admin.customers.pppoe.title')}</h3>
      <p class="subtitle">{$t('admin.customers.pppoe.subtitle')}</p>
    </div>
    <div class="pppoe-toolbar">
      {#if pppoeToolbar.showSearch}
        <label class="pppoe-search" for="customer-pppoe-search">
          <Icon name="search" size={16} />
          <span class="sr-only">{$t('common.search')}</span>
          <input
            id="customer-pppoe-search"
            class="pppoe-search-input"
            bind:value={pppoeQuery}
            placeholder={$t('admin.customers.pppoe.search')}
            oninput={() => void onRefresh()}
          />
        </label>
      {/if}
      {#if pppoeToolbar.showRefresh}
        <button class="btn btn-secondary" onclick={onRefresh} disabled={loadingPppoe}>
          <Icon name="refresh-cw" size={16} />
          {$t('common.refresh')}
        </button>
      {/if}
    </div>
  </div>

  <Table
    columns={pppoeColumns}
    data={pppoeAccounts}
    loading={loadingPppoe}
    emptyText={$t('admin.customers.pppoe.empty')}
    pagination
  >
    {#snippet cell({ item, key })}
      {@const row = item as PppoeAccountPublic}
      {@const routerName = pppoeRouters.find((r: any) => r.id === row.router_id)?.name || '-'}
      {@const locName = locations.find((l: CustomerLocation) => l.id === row.location_id)?.label || '-'}
      {@const syncMeta = getPppoeSyncDisplay(row)}
      {#if key === 'username'}
        <div class="name">{row.username}</div>
        <div class="sub mono">{row.disabled ? $t('common.disabled') || 'Disabled' : $t('common.active') || 'Active'}</div>
        <div class="sub mono">{getPppoeProvisioningTargetFallback(row.account_source)}</div>
      {:else if key === 'router'}
        <div class="name">{routerName}</div>
        <div class="sub mono">{row.router_id}</div>
      {:else if key === 'location'}
        <div class="name">{locName}</div>
        <div class="sub mono">{row.location_id}</div>
      {:else if key === 'assignment'}
        {@const accessState = getPppoeAccessState(row)}
        <div class="sub">
          <span class="pill">{$t('admin.customers.pppoe.fields.profile')}: {row.router_profile_name || '-'}</span>
          <span class="pill">{$t('admin.customers.pppoe.fields.remote_address')}: {row.remote_address || row.address_pool || '-'}</span>
        </div>
        {#if accessState}
          <div class="sub access-state">
            <span class={`access-badge ${accessState.tone}`}>{accessState.label}</span>
            <span>{accessState.detail}</span>
          </div>
        {/if}
      {:else if key === 'sync'}
        <div class="sub">
          <span class={`badge ${syncMeta.tone === 'ok' ? 'ok' : 'warn'}`}>{syncMeta.label}</span>
          <span class="mono">{syncMeta.syncedAt ? timeAgo(syncMeta.syncedAt) : '-'}</span>
        </div>
        {#if syncMeta.error}
          <div class="sub error">{syncMeta.error}</div>
        {/if}
        {#if row.account_source === 'managed_radius' && row.radius_identity}
          <div class="sub mono">Identity: {row.radius_identity}</div>
        {/if}
      {:else if key === 'actions'}
        <div class="row-actions">
          {#if canManagePppoe}
            <button class="btn-icon" title={$t('admin.customers.pppoe.actions.apply') || getPppoeApplyActionFallback(row.account_source)} onclick={() => onApplyPppoe(row)}>
              <Icon name="send" size={16} />
            </button>
            <button class="btn-icon" title={$t('common.edit')} onclick={() => onEditPppoe(row)}>
              <Icon name="edit" size={16} />
            </button>
            <button class="btn-icon danger" title={$t('common.delete')} onclick={() => onDeletePppoe(row)}>
              <Icon name="trash-2" size={16} />
            </button>
          {/if}
        </div>
      {:else}
        {item[key] ?? ''}
      {/if}
    {/snippet}
  </Table>
</div>

<style>
  .section {
    padding: 1.1rem;
    background: var(--bg-surface);
  }
  .section-head,
  .pppoe-toolbar {
    display: flex;
    gap: 1rem;
  }
  .section-head {
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
  }
  .subtitle,
  .sub {
    color: var(--text-secondary);
  }
  .subtitle {
    margin-top: 0.25rem;
  }
  .pppoe-toolbar {
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    width: min(100%, 36rem);
    padding: 0.85rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 16%);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg-surface), transparent 5%);
  }
  .pppoe-search {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: min(100%, 18rem);
    flex: 1 1 18rem;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 12%);
    border-radius: 14px;
    padding: 0.72rem 0.9rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 4%);
    color: var(--text-secondary);
  }
  .pppoe-search-input {
    width: 100%;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    padding: 0;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  .name {
    font-weight: 650;
  }
  .sub {
    font-size: 0.85rem;
    margin-top: 0.15rem;
  }
  .mono {
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 0.28rem 0.62rem;
    background: color-mix(in srgb, var(--bg-surface), transparent 12%);
    margin-right: 0.35rem;
  }
  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .access-state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.45rem;
  }

  .access-badge {
    display: inline-flex;
    align-items: center;
    width: fit-content;
    border-radius: 999px;
    padding: 0.24rem 0.58rem;
    font-size: 0.74rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    border: 1px solid color-mix(in srgb, var(--border-color), transparent 10%);
  }

  .access-badge.warning {
    color: rgb(245, 158, 11);
    background: rgba(245, 158, 11, 0.1);
    border-color: rgba(245, 158, 11, 0.24);
  }

  .access-badge.danger {
    color: rgb(239, 68, 68);
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.24);
  }

  .access-badge.neutral {
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-surface), transparent 5%);
  }
  .btn,
  .btn-icon {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    cursor: pointer;
  }
  .btn {
    border-radius: 12px;
    min-height: 42px;
    padding: 0.55rem 0.9rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-weight: 650;
    font-size: 0.9rem;
  }
  .btn-icon {
    border-radius: 10px;
    width: 38px;
    height: 38px;
    padding: 0;
  }
  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }
  @media (max-width: 900px) {
    .section-head {
      flex-direction: column;
      align-items: stretch;
    }
    .pppoe-toolbar {
      width: 100%;
      justify-content: stretch;
    }
    .pppoe-search {
      min-width: 0;
      width: 100%;
    }
    .pppoe-toolbar .btn {
      width: 100%;
    }
    .row-actions {
      justify-content: flex-start;
    }
  }
</style>
