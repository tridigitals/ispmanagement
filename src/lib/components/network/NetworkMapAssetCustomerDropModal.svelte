<script lang="ts">
  import { t } from 'svelte-i18n';

  import Icon from '$lib/components/ui/Icon.svelte';
  import type { TopologyAssetCustomerDropItem } from '$lib/components/network/networkMapAssetConnections';

  export let show = false;
  export let title = '';
  export let items: TopologyAssetCustomerDropItem[] = [];
  export let onClose: () => void;
  export let onView: (item: TopologyAssetCustomerDropItem) => void;

  function stateLabel(state: TopologyAssetCustomerDropItem['visualState']) {
    if (state === 'suspended')
      return $t('admin.network.map.asset_customer_drop.state_suspended') || 'Suspended';
    if (state === 'internet_disconnected') {
      return (
        $t('admin.network.map.asset_customer_drop.state_internet_disconnected') || 'Internet Off'
      );
    }
    return $t('admin.network.map.asset_customer_drop.state_normal') || 'Normal';
  }
</script>

{#if show}
  <div
    class="asset-customer-drop-modal-backdrop"
    role="presentation"
    tabindex="-1"
    onclick={onClose}
    onkeydown={(event) => {
      if (event.key === 'Escape' || event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        onClose();
      }
    }}
  >
    <div
      class="asset-customer-drop-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="asset-customer-drop-modal-title"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <div class="asset-customer-drop-modal-head">
        <div>
          <div class="asset-customer-drop-modal-kicker">
            {$t('admin.network.map.asset_customer_drop.title')}
          </div>
          <h3 id="asset-customer-drop-modal-title">{title}</h3>
        </div>
        <button class="asset-customer-drop-modal-close" type="button" onclick={onClose}>
          <Icon name="x" size={16} />
        </button>
      </div>

      {#if items.length > 0}
        <div class="asset-customer-drop-list">
          {#each items as item (item.key)}
            <article class="asset-customer-drop-card">
              <div class="asset-customer-drop-card-main">
                <div class="asset-customer-drop-card-copy">
                  <div class="asset-customer-drop-card-head">
                    <div class="asset-customer-drop-name">{item.customerName}</div>
                    <span class={`asset-customer-drop-status ${item.visualState}`}>
                      {stateLabel(item.visualState)}
                    </span>
                  </div>
                  {#if item.locationLabel}
                    <div class="asset-customer-drop-location">{item.locationLabel}</div>
                  {/if}
                  {#if item.serviceName}
                    <div class="asset-customer-drop-service">{item.serviceName}</div>
                  {/if}
                </div>
                <div class="asset-customer-drop-actions">
                  <button class="btn ghost btn-xs" type="button" onclick={() => onView(item)}>
                    {$t('admin.network.map.asset_customer_drop.view')}
                  </button>
                </div>
              </div>
            </article>
          {/each}
        </div>
      {:else}
        <div class="asset-customer-drop-empty">
          <Icon name="users" size={18} />
          <span>
            {$t('admin.network.map.asset_customer_drop.empty')}
          </span>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* Button styles replicated from the page's scoped block: global.css has
     `.btn`/`.btn-ghost` but NOT the compound `.btn.ghost` or `.btn-xs` used here. */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 12px;
    border-radius: 10px;
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

  .btn-xs {
    padding: 5px 9px;
    font-size: 0.78rem;
    border-radius: 8px;
  }

  .asset-customer-drop-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(15, 23, 42, 0.72);
  }

  .asset-customer-drop-modal {
    width: min(460px, calc(100vw - 24px));
    max-height: min(72vh, 680px);
    overflow: auto;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(148, 163, 184, 0.18);
    background: rgba(15, 23, 42, 0.97);
    box-shadow: 0 24px 64px rgba(2, 6, 23, 0.4);
    padding: 14px;
    color: #e2e8f0;
  }

  .asset-customer-drop-modal-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }

  .asset-customer-drop-modal-kicker {
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #93c5fd;
    margin-bottom: 4px;
  }

  .asset-customer-drop-modal-head h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 800;
    color: #f8fafc;
  }

  .asset-customer-drop-modal-close {
    width: 34px;
    height: 34px;
    display: inline-grid;
    place-items: center;
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 999px;
    background: rgba(30, 41, 59, 0.84);
    color: #cbd5e1;
    cursor: pointer;
    flex-shrink: 0;
  }

  .asset-customer-drop-list {
    display: grid;
    gap: 8px;
  }

  .asset-customer-drop-card {
    padding: 11px 12px;
    border-radius: 14px;
    border: 1px solid rgba(59, 130, 246, 0.12);
    background: rgba(15, 23, 42, 0.72);
  }

  .asset-customer-drop-card-main {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
  }

  .asset-customer-drop-card-copy {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .asset-customer-drop-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .asset-customer-drop-name {
    font-size: 0.88rem;
    font-weight: 800;
    color: #f8fafc;
    line-height: 1.2;
  }

  .asset-customer-drop-location {
    font-size: 0.76rem;
    color: rgba(226, 232, 240, 0.72);
    line-height: 1.25;
  }

  .asset-customer-drop-status {
    display: inline-flex;
    align-items: center;
    min-height: 24px;
    padding: 0 9px;
    border-radius: 999px;
    border: 1px solid rgba(251, 191, 36, 0.28);
    background: rgba(30, 41, 59, 0.82);
    color: #f8fafc;
    font-size: 0.68rem;
    font-weight: 800;
    text-transform: uppercase;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .asset-customer-drop-status.normal {
    color: #22c55e;
    border-color: rgba(34, 197, 94, 0.34);
    background: rgba(34, 197, 94, 0.12);
  }

  .asset-customer-drop-status.suspended {
    color: #f59e0b;
    border-color: rgba(245, 158, 11, 0.3);
    background: rgba(245, 158, 11, 0.12);
  }

  .asset-customer-drop-status.internet_disconnected {
    color: #f87171;
    border-color: rgba(248, 113, 113, 0.32);
    background: rgba(248, 113, 113, 0.12);
  }

  .asset-customer-drop-service {
    font-size: 0.77rem;
    color: #cbd5e1;
    line-height: 1.25;
  }

  .asset-customer-drop-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .asset-customer-drop-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 140px;
    border-radius: var(--radius-lg);
    border: 1px dashed rgba(148, 163, 184, 0.22);
    color: rgba(226, 232, 240, 0.78);
    text-align: center;
  }
</style>
