<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';

  export let show = false;
  export let saving = false;
  export let assetName = '';
  export let assetTypeLabel = '';
  export let supportsCustomerDrop = false;
  export let loadingCustomers = false;
  export let loadingLocations = false;
  export let draft: {
    parentAssetId: string;
    customerId: string;
    locationId: string;
  };
  export let parentOptions: Array<{ value: string; label: string }> = [];
  export let customerOptions: Array<{ value: string; label: string }> = [];
  export let locationOptions: Array<{ value: string; label: string }> = [];
  export let onClose: () => void;
  export let onSubmit: () => void;
</script>

<Modal {show} title={$t('network.asset.connect_ftth')} width="620px" onclose={() => !saving && onClose()}>
  <div class="asset-connect-shell">
    <div class="asset-connect-intro">
      <div class="asset-connect-kicker">{$t('network.asset.current_asset')}</div>
      <div class="asset-connect-title">{assetName}</div>
      <div class="asset-connect-subtitle">{assetTypeLabel}</div>
    </div>

    <div class="asset-connect-grid">
      <label class="field span-2">
        <span>{$t('network.asset.upstream_parent')}</span>
        <select class="input" bind:value={draft.parentAssetId} disabled={saving}>
          <option value="">{$t('network.asset.no_parent')}</option>
          {#each parentOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
        <small>{$t('network.asset.parent_hint')}</small>
      </label>

      {#if supportsCustomerDrop}
        <label class="field">
          <span>{$t('common.customer')}</span>
          <select class="input" bind:value={draft.customerId} disabled={saving || loadingCustomers}>
            <option value="">{loadingCustomers ? ($t('common.loading') || 'Loading customers...') : ($t('network.asset.no_customer') || 'No customer')}</option>
            {#each customerOptions as option}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>{$t('common.location')}</span>
          <select
            class="input"
            bind:value={draft.locationId}
            disabled={saving || !draft.customerId || loadingLocations}
          >
            <option value="">
              {!draft.customerId
                ? ($t('network.asset.select_customer_first') || 'Select customer first')
                : loadingLocations
                  ? ($t('common.loading') || 'Loading locations...')
                  : ($t('network.asset.no_location') || 'No location')}
            </option>
            {#each locationOptions as option}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
          <small>{$t('network.asset.location_hint')}</small>
        </label>
      {/if}
    </div>

    <div class="asset-connect-actions">
      <button class="btn ghost" type="button" onclick={onClose} disabled={saving}>{$t('common.cancel')}</button>
      <button class="btn" type="button" onclick={onSubmit} disabled={saving}>
        {saving ? ($t('common.saving') || 'Saving...') : ($t('network.asset.save_connection') || 'Save Connection')}
      </button>
    </div>
  </div>
</Modal>

<style>
  .asset-connect-shell {
    display: grid;
    gap: 14px;
  }

  .asset-connect-intro {
    padding: 12px 14px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--surface-color, #111827) 92%, white 8%);
  }

  .asset-connect-kicker {
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .asset-connect-title {
    margin-top: 4px;
    font-size: 1rem;
    font-weight: 800;
    color: var(--text-primary);
  }

  .asset-connect-subtitle {
    margin-top: 2px;
    color: var(--text-secondary);
    font-size: 0.84rem;
  }

  .asset-connect-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .field.span-2 {
    grid-column: 1 / -1;
  }

  .field span {
    font-size: 0.82rem;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .field small {
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.35;
  }

  .input {
    width: 100%;
    min-height: 42px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--surface-color);
    color: var(--text-primary);
    padding: 0 12px;
  }

  .asset-connect-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 124px;
    min-height: 40px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: #fff;
    font-weight: 800;
    cursor: pointer;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  @media (max-width: 720px) {
    .asset-connect-grid {
      grid-template-columns: 1fr;
    }

    .field.span-2 {
      grid-column: auto;
    }
  }
</style>
