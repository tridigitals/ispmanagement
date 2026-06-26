<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ResponsiveTabs from '$lib/components/ui/ResponsiveTabs.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';

  let {
    showServiceTypePicker = $bindable(false),
    showPkgModal = $bindable(false),
    showMapModal = $bindable(false),
    editingPkg,
    saving,
    serviceTypeCards,
    startCreateWithType,
    pkgFormTab = $bindable('details'),
    pkgFeatures = $bindable([]),
    serviceTypeLabel,
    pkgServiceType,
    pkgProvisioningType = $bindable('pppoe'),
    provisioningTypeLabel,
    pkgName = $bindable(''),
    pkgDesc = $bindable(''),
    tenantCurrencyCode,
    pkgPriceMonthly = $bindable(0),
    pkgYearlyEnabled = $bindable(false),
    pkgPriceYearly = $bindable(0),
    baseCurrencyCode,
    formatDisplayPrice,
    pkgActive = $bindable(true),
    isInternetType,
    isPppoeProvisioning,
    pkgMapEnabled = $bindable(false),
    routerOptions,
    pkgMapRouterId = $bindable(''),
    pkgMapProfile = $bindable(''),
    pkgMapPool = $bindable(''),
    pkgMapIsolationPool = $bindable(''),
    loadPkgRouterMeta,
    pkgProfileOptions,
    pkgPoolOptions,
    pkgLoadingMeta,
    serviceTypeFeatureSuggestions,
    addFeatureIfMissing,
    pkgFeatureInput = $bindable(''),
    addFeature,
    removeFeature,
    savePackage,
    mapPkg,
    mapRouterId = $bindable(''),
    mapProfile = $bindable(''),
    mapPool = $bindable(''),
    mapIsolationPool = $bindable(''),
    loadRouterMeta,
    mapProfileOptions,
    mapPoolOptions,
    loadingMeta,
    saveMapping,
  } = $props();

  let isMobile = $state(false);
  const packageTabs = $derived.by(() => [
    { id: 'details', label: $t('admin.network.packages.tabs.details') || 'Details' },
    {
      id: 'features',
      label: $t('admin.network.packages.tabs.features') || 'Features',
      count: pkgFeatures.length,
    },
  ]);

  onMount(() => {
    const mq = window.matchMedia('(max-width: 900px)');
    const updateViewport = () => {
      isMobile = mq.matches;
    };
    updateViewport();
    mq.addEventListener('change', updateViewport);

    return () => {
      mq.removeEventListener('change', updateViewport);
    };
  });
</script>

<Modal
  show={showServiceTypePicker}
  title={$t('admin.services.type_picker.title')}
  width="860px"
  onclose={() => (showServiceTypePicker = false)}
>
  <div class="type-picker-wrap">
    <p class="type-picker-subtitle">{$t('admin.services.category_hint')}</p>
    <div class="type-card-grid">
      {#each serviceTypeCards as card}
        <button type="button" class="type-card" onclick={() => startCreateWithType(card.value)}>
          <div class="type-card-head">
            <span class="type-card-icon">
              <Icon name={card.icon} size={18} />
            </span>
            <span class="type-card-title">{card.title}</span>
          </div>
          <p class="type-card-subtitle">{card.subtitle}</p>
          <div class="type-card-tags">
            {#each card.tags as tag}
              <span class="type-card-tag">{tag}</span>
            {/each}
          </div>
          <span class="type-card-cta">
            {$t('admin.services.type_picker.continue')}
            <Icon name="arrow-right" size={14} />
          </span>
        </button>
      {/each}
    </div>
  </div>
</Modal>

<Modal
  show={showPkgModal}
  title={editingPkg
    ? $t('admin.services.actions.edit') ||
      $t('admin.network.packages.actions.edit') ||
      'Edit service'
    : $t('admin.services.actions.add') || $t('admin.network.packages.actions.add') || 'Add service'}
  width="640px"
  onclose={() => (showPkgModal = false)}
>
  <div class="form">
    <ResponsiveTabs
      items={packageTabs}
      bind:activeId={pkgFormTab}
      {isMobile}
      priorityCount={2}
      ariaLabel="Package tabs"
    />

    {#if pkgFormTab === 'details'}
      <div class="selected-type-banner">
        <div class="selected-type-main">
          <span class="selected-type-label"
            >{$t('admin.services.fields.service_type')}</span
          >
          <span class="badge neutral">{serviceTypeLabel(pkgServiceType, pkgProvisioningType)}</span>
        </div>
        {#if !editingPkg}
          <button
            class="btn ghost btn-sm"
            type="button"
            onclick={() => {
              showPkgModal = false;
              showServiceTypePicker = true;
            }}
          >
            <Icon name="refresh-cw" size={14} />
            {$t('admin.services.type_picker.change')}
          </button>
        {/if}
      </div>

      <div class="type-hints">
        {#each serviceTypeFeatureSuggestions[pkgServiceType] as suggestion}
          <button
            type="button"
            class="hint-chip"
            onclick={() => addFeatureIfMissing(suggestion)}
            title={$t('admin.services.type_picker.add_as_feature')}
          >
            <Icon name="plus" size={12} />
            {suggestion}
          </button>
        {/each}
      </div>

      <label>
        <span>{$t('admin.network.packages.fields.name')}</span>
        <input class="input" bind:value={pkgName} />
      </label>

      <label>
        <span>{$t('admin.network.packages.fields.description')}</span>
        <input class="input" bind:value={pkgDesc} />
      </label>

      {#if isInternetType(pkgServiceType)}
        <label>
          <span>{$t('admin.services.fields.provisioning_type')}</span>
          <select class="input" bind:value={pkgProvisioningType}>
            <option value="pppoe">{provisioningTypeLabel('pppoe')}</option>
            <option value="dhcp_static">{provisioningTypeLabel('dhcp_static')}</option>
          </select>
          <div class="field-hint">
            {$t('admin.services.fields.provisioning_type_hint')}
          </div>
        </label>
      {/if}

      <label>
        <span
          >{$t('admin.network.packages.fields.price_monthly')} ({tenantCurrencyCode})</span
        >
        <div class="price-input-wrap">
          <input
            class="input mono with-addon"
            type="number"
            min="0"
            step="0.01"
            bind:value={pkgPriceMonthly}
            required
          />
          <span class="currency-addon">{tenantCurrencyCode}</span>
        </div>
      </label>

      <div class="toggle-row">
        <div class="toggle-text">
          <div class="toggle-title">
            {$t('admin.network.packages.fields.enable_yearly')}
          </div>
          <div class="toggle-sub">
            {$t('admin.network.packages.fields.enable_yearly_hint')}
          </div>
        </div>
        <Toggle
          bind:checked={pkgYearlyEnabled}
          ariaLabel={$t('admin.network.packages.fields.enable_yearly')}
        />
      </div>

      {#if pkgYearlyEnabled}
        <label>
          <span
            >{$t('admin.network.packages.fields.price_yearly')} ({tenantCurrencyCode})</span
          >
          <div class="price-input-wrap">
            <input
              class="input mono with-addon"
              type="number"
              min="0"
              step="0.01"
              bind:value={pkgPriceYearly}
            />
            <span class="currency-addon">{tenantCurrencyCode}</span>
          </div>
          <div class="field-hint">
            {$t('admin.network.packages.fields.currency_active')}:
            <strong>{tenantCurrencyCode}</strong>
            {#if tenantCurrencyCode !== baseCurrencyCode}
              · {$t('admin.network.packages.fields.currency_base')}:
              <strong>{baseCurrencyCode}</strong>
            {/if}
          </div>
          <div class="field-hint">
            {$t('admin.network.packages.fields.price_hint')}
            {#if tenantCurrencyCode !== baseCurrencyCode}
              <span class="hint-inline">
                Preview: {formatDisplayPrice(Number(pkgPriceMonthly || 0))}/mo, {formatDisplayPrice(
                  Number(pkgPriceYearly || 0),
                )}/yr
              </span>
            {/if}
          </div>
        </label>
      {/if}

      <div class="toggle-row">
        <div class="toggle-text">
          <div class="toggle-title">{$t('admin.network.packages.fields.active')}</div>
          <div class="toggle-sub">
            {$t('admin.network.packages.fields.active_hint')}
          </div>
        </div>
        <Toggle
          bind:checked={pkgActive}
          ariaLabel={$t('admin.network.packages.fields.active')}
        />
      </div>

      {#if isInternetType(pkgServiceType) && isPppoeProvisioning(pkgProvisioningType)}
        <div class="toggle-row">
          <div class="toggle-text">
            <div class="toggle-title">
              {$t('admin.network.packages.mapping.inline_title')}
            </div>
            <div class="toggle-sub">
              {$t('admin.network.packages.mapping.inline_hint')}
            </div>
          </div>
          <Toggle
            bind:checked={pkgMapEnabled}
            ariaLabel={$t('admin.network.packages.mapping.inline_title')}
          />
        </div>
      {:else if !isInternetType(pkgServiceType)}
        <div class="field-hint">
          {$t('admin.services.mapping.not_required')}
        </div>
      {:else}
        <div class="field-hint">
          {$t('admin.services.mapping.not_required_dhcp')}
        </div>
      {/if}

      {#if isInternetType(pkgServiceType) && isPppoeProvisioning(pkgProvisioningType) && pkgMapEnabled}
        <div class="grid2">
          <label>
            <span>{$t('admin.network.packages.mapping.router')}</span>
            <Select2
              bind:value={pkgMapRouterId}
              options={routerOptions}
              placeholder={($t('common.select') || 'Select') + '...'}
              width="100%"
              maxItems={5000}
              searchPlaceholder={$t('common.search')}
              noResultsText={$t('common.no_results')}
              onchange={() => {
                pkgMapProfile = '';
                pkgMapPool = '';
                pkgMapIsolationPool = '';
                void loadPkgRouterMeta(pkgMapRouterId);
              }}
            />
          </label>
          <label>
            <span>{$t('admin.network.packages.mapping.profile')}</span>
            <Select2
              bind:value={pkgMapProfile}
              options={pkgProfileOptions}
              placeholder={($t('common.select') || 'Select') + '...'}
              width="100%"
              disabled={!pkgMapRouterId || pkgProfileOptions.length === 0}
              maxItems={5000}
              searchPlaceholder={$t('common.search')}
              noResultsText={$t('common.no_results')}
            />
          </label>
        </div>

        <div class="grid2">
          <label>
            <span>{$t('admin.network.packages.mapping.pool')}</span>
            <Select2
              bind:value={pkgMapPool}
              options={pkgPoolOptions}
              placeholder={($t('common.select') || 'Select') + '...'}
              width="100%"
              disabled={!pkgMapRouterId || pkgPoolOptions.length === 0}
              maxItems={5000}
              searchPlaceholder={$t('common.search')}
              noResultsText={$t('common.no_results')}
            />
          </label>
          <label>
            <span
              >{$t('admin.services.mapping.isolation_pool')}</span
            >
            <Select2
              bind:value={pkgMapIsolationPool}
              options={pkgPoolOptions}
              placeholder={($t('common.select') || 'Select') + '...'}
              width="100%"
              disabled={!pkgMapRouterId || pkgPoolOptions.length === 0}
              maxItems={5000}
              searchPlaceholder={$t('common.search')}
              noResultsText={$t('common.no_results')}
            />
          </label>
        </div>

        {#if pkgMapRouterId && !pkgLoadingMeta && pkgProfileOptions.length === 0}
          <div class="field-hint">
            {$t('admin.network.packages.mapping.profile_empty')}
          </div>
        {/if}

        {#if pkgMapRouterId && !pkgLoadingMeta && pkgPoolOptions.length === 0}
          <div class="field-hint">
            {$t('admin.network.packages.mapping.pool_empty')}
          </div>
        {/if}

        {#if pkgLoadingMeta}
          <div class="hint">
            <span class="spin"><Icon name="refresh-cw" size={14} /></span>
            <span>{$t('common.loading')} suggestions…</span>
          </div>
        {/if}
      {/if}
    {:else}
      <label>
        <span>{$t('admin.network.packages.fields.features')}</span>
        <div class="feature-input-row">
          <input
            class="input"
            bind:value={pkgFeatureInput}
            placeholder={$t('admin.network.packages.fields.feature_placeholder')}
            onkeydown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                addFeature();
              }
            }}
          />
          <button class="btn ghost" type="button" onclick={addFeature}>
            <Icon name="plus" size={14} />
            {$t('admin.network.packages.actions.add_feature')}
          </button>
        </div>
        {#if pkgFeatures.length > 0}
          <div class="feature-list">
            {#each pkgFeatures as f, i}
              <span class="feature-chip">
                {f}
                <button
                  type="button"
                  class="feature-remove"
                  onclick={() => removeFeature(i)}
                  aria-label="remove feature"
                >
                  <Icon name="x" size={12} />
                </button>
              </span>
            {/each}
          </div>
        {:else}
          <div class="field-hint">
            {$t('admin.network.packages.fields.features_empty')}
          </div>
        {/if}
      </label>
    {/if}

    <div class="actions">
      <button
        class="btn ghost"
        type="button"
        onclick={() => (showPkgModal = false)}
        disabled={saving}
      >
        {$t('common.cancel')}
      </button>
      <button
        class="btn"
        type="button"
        onclick={savePackage}
        disabled={saving ||
          !pkgName.trim() ||
          !(Number(pkgPriceMonthly) > 0) ||
          (pkgYearlyEnabled && !(Number(pkgPriceYearly) > 0)) ||
          (isInternetType(pkgServiceType) &&
            pkgMapEnabled &&
            (!pkgMapRouterId || !pkgMapProfile.trim()))}
      >
        <Icon name="save" size={16} />
        {$t('common.save')}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showMapModal}
  title={$t('admin.network.packages.mapping.title')}
  width="760px"
  onclose={() => (showMapModal = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.network.packages.mapping.package')}</span>
        <input class="input" value={mapPkg?.name || ''} disabled />
      </label>
      <label>
        <span>{$t('admin.network.packages.mapping.router')}</span>
        <Select2
          bind:value={mapRouterId}
          options={routerOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          maxItems={5000}
          searchPlaceholder={$t('common.search')}
          noResultsText={$t('common.no_results')}
          onchange={() => void loadRouterMeta(mapRouterId)}
        />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.network.packages.mapping.profile')}</span>
        <Select2
          bind:value={mapProfile}
          options={mapProfileOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!mapRouterId || mapProfileOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search')}
          noResultsText={$t('common.no_results')}
        />
      </label>
      <label>
        <span>{$t('admin.network.packages.mapping.pool')}</span>
        <Select2
          bind:value={mapPool}
          options={mapPoolOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!mapRouterId || mapPoolOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search')}
          noResultsText={$t('common.no_results')}
        />
      </label>
      <label>
        <span
          >{$t('admin.services.mapping.isolation_pool')}</span
        >
        <Select2
          bind:value={mapIsolationPool}
          options={mapPoolOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!mapRouterId || mapPoolOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search')}
          noResultsText={$t('common.no_results')}
        />
      </label>
    </div>

    {#if mapRouterId && !loadingMeta && mapProfileOptions.length === 0}
      <div class="field-hint">
        {$t('admin.network.packages.mapping.profile_empty')}
      </div>
    {/if}

    {#if mapRouterId && !loadingMeta && mapPoolOptions.length === 0}
      <div class="field-hint">
        {$t('admin.network.packages.mapping.pool_empty')}
      </div>
    {/if}

    {#if loadingMeta}
      <div class="hint">
        <span class="spin"><Icon name="refresh-cw" size={14} /></span>
        <span>{$t('common.loading')} suggestions…</span>
      </div>
    {/if}

    <div class="actions">
      <button
        class="btn ghost"
        type="button"
        onclick={() => (showMapModal = false)}
        disabled={saving}
      >
        {$t('common.cancel')}
      </button>
      <button
        class="btn"
        type="button"
        onclick={saveMapping}
        disabled={saving || !mapPkg || !mapRouterId || !mapProfile.trim()}
      >
        <Icon name="save" size={16} />
        {$t('common.save')}
      </button>
    </div>
  </div>
</Modal>

<style>
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
  .btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }
  .btn-sm {
    padding: 0.5rem 0.7rem;
    border-radius: 10px;
    font-size: 0.82rem;
  }
  .field-hint {
    margin-top: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.35;
  }
  .hint-inline {
    margin-left: 0.35rem;
    color: var(--text-primary);
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 999px;
    font-weight: 900;
    font-size: 0.78rem;
    border: 1px solid var(--border-color);
  }
  .badge.neutral {
    background: rgba(99, 102, 241, 0.12);
    color: rgba(199, 210, 254, 0.98);
    border-color: rgba(99, 102, 241, 0.32);
  }
  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }
  .form {
    display: grid;
    gap: 0.9rem;
  }
  .type-picker-wrap {
    display: grid;
    gap: 1rem;
  }
  .type-picker-subtitle {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.95rem;
    line-height: 1.5;
  }
  .type-card-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.8rem;
  }
  .type-card {
    text-align: left;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    border-radius: 14px;
    padding: 0.95rem;
    color: var(--text-primary);
    display: grid;
    gap: 0.65rem;
    cursor: pointer;
    transition:
      border-color 0.2s ease,
      transform 0.2s ease,
      background 0.2s ease;
  }
  .type-card:hover {
    border-color: rgba(99, 102, 241, 0.45);
    transform: translateY(-2px);
    background: var(--bg-surface);
  }
  .type-card-head {
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }
  .type-card-icon {
    width: 32px;
    height: 32px;
    border-radius: 10px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(99, 102, 241, 0.35);
    background: rgba(99, 102, 241, 0.15);
    color: rgba(199, 210, 254, 0.98);
  }
  .type-card-title {
    font-weight: 900;
    letter-spacing: 0.01em;
  }
  .type-card-subtitle {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.45;
    min-height: 3.7em;
  }
  .type-card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }
  .type-card-tag {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.22rem 0.5rem;
    font-size: 0.72rem;
    font-weight: 750;
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.05);
  }
  .type-card-cta {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-weight: 850;
    color: var(--text-primary);
    font-size: 0.85rem;
  }
  .selected-type-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 0.8rem 0.9rem;
    background: rgba(255, 255, 255, 0.03);
  }
  .selected-type-main {
    display: inline-flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
  }
  .selected-type-label {
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-size: 0.72rem;
    font-weight: 850;
  }
  .type-hints {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
  }
  .hint-chip {
    border: 1px dashed rgba(99, 102, 241, 0.45);
    border-radius: 999px;
    padding: 0.35rem 0.62rem;
    background: rgba(99, 102, 241, 0.08);
    color: var(--text-primary);
    font-weight: 750;
    font-size: 0.78rem;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    cursor: pointer;
  }
  .hint-chip:hover {
    background: rgba(99, 102, 241, 0.15);
    border-style: solid;
  }
  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8rem;
  }
  label {
    display: grid;
    gap: 0.35rem;
  }
  label > span {
    color: var(--text-secondary);
    font-weight: 850;
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .input {
    width: 100%;
    padding: 0.85rem 0.95rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
    outline: none;
    font-weight: 650;
  }
  .feature-input-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.5rem;
    align-items: center;
  }
  .feature-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.4rem;
  }
  .feature-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.3rem 0.55rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-hover), transparent 30%);
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
    line-height: 1;
  }
  .feature-remove {
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .feature-remove:hover {
    color: var(--text-primary);
  }
  .price-input-wrap {
    position: relative;
  }
  .input.with-addon {
    padding-right: 5.2rem;
  }
  .currency-addon {
    position: absolute;
    right: 0.6rem;
    top: 50%;
    transform: translateY(-50%);
    border: 1px solid var(--border-color);
    border-radius: 9px;
    padding: 0.22rem 0.5rem;
    font-size: 0.72rem;
    letter-spacing: 0.05em;
    font-weight: 800;
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.06);
    pointer-events: none;
    user-select: none;
  }
  :global([data-theme='light']) .currency-addon {
    background: rgba(0, 0, 0, 0.06);
  }
  :global([data-theme='light']) .input {
    background: rgba(0, 0, 0, 0.03);
  }
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.9rem;
    padding: 0.9rem 1rem;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
  }
  :global([data-theme='light']) .toggle-row {
    background: rgba(0, 0, 0, 0.02);
  }
  .toggle-text {
    min-width: 0;
    display: grid;
    gap: 0.15rem;
  }
  .toggle-title {
    color: var(--text-primary);
    font-weight: 900;
  }
  .toggle-sub {
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.92rem;
    line-height: 1.35;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    padding-top: 0.25rem;
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.9rem;
  }
  .spin {
    display: inline-flex;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
  @media (max-width: 768px) {
    .grid2 {
      grid-template-columns: 1fr;
    }
    .type-card-grid {
      grid-template-columns: 1fr;
    }
    .selected-type-banner {
      flex-direction: column;
      align-items: stretch;
    }
    .selected-type-main {
      width: 100%;
      flex-wrap: wrap;
    }
  }
</style>
