<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can, user, tenant } from '$lib/stores/auth';
  import { api, type IspPackage, type IspPackageRouterMappingView } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
import { formatMoney } from '$lib/utils/money';
import { appSettings } from '$lib/stores/settings';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import TableToolbar from '$lib/components/ui/TableToolbar.svelte';
  import { getSlugFromDomain } from '$lib/utils/domain';

  type RouterRow = { id: string; name: string };
  type ProfileSuggestion = { id: string; name: string };
  type PoolSuggestion = { id: string; name: string };

  const domainSlug = $derived(getSlugFromDomain($page.url.hostname));
  const effectiveTenantSlug = $derived(
    ($tenant?.slug || $user?.tenant_slug || String($page.params.tenant || '')).trim(),
  );
  const isCustomDomain = $derived(domainSlug && domainSlug === effectiveTenantSlug);
  const tenantPrefix = $derived(
    effectiveTenantSlug && !isCustomDomain ? `/${effectiveTenantSlug}` : '',
  );

  let loading = $state(true);
  let saving = $state(false);

  let baseCurrencyCode = $state('IDR');
  let baseLocale = $state('en-US');
  let fxRate = $state<number | null>(null);

  const tenantCurrencyCode = $derived.by(() => String($appSettings?.currency_code || baseCurrencyCode).toUpperCase());

  let q = $state('');
  let packages = $state<IspPackage[]>([]);
  let total = $state(0);
  let packagePage = $state(0);
  let packagePageSize = $state(10);
  let packageTableVersion = $state(0);

  let routers = $state<RouterRow[]>([]);
  let mappings = $state<IspPackageRouterMappingView[]>([]);

  // Create/Edit package
  let showPkgModal = $state(false);
  let editingPkg = $state<IspPackage | null>(null);
  let pkgName = $state('');
  let pkgDesc = $state('');
  let pkgFeatures = $state<string[]>([]);
  let pkgFeatureInput = $state('');
  let pkgActive = $state(true);
  let pkgPriceMonthly = $state(0);
  let pkgPriceYearly = $state(0);
  let pkgYearlyEnabled = $state(false);
  let pkgFormTab = $state<'details' | 'features'>('details');

  // Optional inline mapping when creating/editing a package
  let pkgMapEnabled = $state(false);
  let pkgMapRouterId = $state('');
  let pkgMapProfile = $state('');
  let pkgMapPool = $state('');
  let pkgProfileSuggestions = $state<ProfileSuggestion[]>([]);
  let pkgPoolSuggestions = $state<PoolSuggestion[]>([]);
  let pkgLoadingMeta = $state(false);

  // Router mapping modal
  let showMapModal = $state(false);
  let mapPkg = $state<IspPackage | null>(null);
  let mapRouterId = $state('');
  let mapProfile = $state('');
  let mapPool = $state('');
  let profileSuggestions = $state<ProfileSuggestion[]>([]);
  let poolSuggestions = $state<PoolSuggestion[]>([]);
  let loadingMeta = $state(false);

  const routerOptions = $derived.by(() => routers.map((r) => ({ label: r.name, value: r.id })));

  const pkgProfileOptions = $derived.by(() => {
    const base = (pkgProfileSuggestions || []).map((x) => ({ label: x.name, value: x.name }));
    const cur = pkgMapProfile?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  });

  const pkgPoolOptions = $derived.by(() => {
    const base = (pkgPoolSuggestions || []).map((x) => ({ label: x.name, value: x.name }));
    const cur = pkgMapPool?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  });

  const mapProfileOptions = $derived.by(() => {
    const base = (profileSuggestions || []).map((x) => ({ label: x.name, value: x.name }));
    const cur = mapProfile?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  });

  const mapPoolOptions = $derived.by(() => {
    const base = (poolSuggestions || []).map((x) => ({ label: x.name, value: x.name }));
    const cur = mapPool?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  });

  const columns = $derived.by(() => [
    { key: 'name', label: $t('admin.network.packages.columns.name') || 'Name' },
    { key: 'price', label: $t('admin.network.packages.columns.price') || 'Price', width: '160px' },
    { key: 'status', label: $t('admin.network.packages.columns.status') || 'Status', width: '120px' },
    { key: 'mappings', label: $t('admin.network.packages.columns.mappings') || 'Mapped', width: '140px' },
    { key: 'actions', label: '', align: 'right' as const, width: '220px' },
  ]);

  function roundForCurrency(amount: number, currencyCode: string): number {
    const c = currencyCode.toUpperCase();
    const digits = c === 'IDR' || c === 'JPY' || c === 'KRW' ? 0 : 2;
    const factor = Math.pow(10, digits);
    return Math.round(amount * factor) / factor;
  }

  function formatBasePrice(amount: number): string {
    return formatMoney(amount, { currency: baseCurrencyCode, locale: baseLocale });
  }

  function formatDisplayPrice(amount: number): string {
    if (!amount) return formatMoney(0, { currency: tenantCurrencyCode, locale: baseLocale });
    if (tenantCurrencyCode === baseCurrencyCode) {
      return formatMoney(amount, { currency: baseCurrencyCode, locale: baseLocale });
    }
    if (!fxRate) {
      return formatMoney(amount, { currency: baseCurrencyCode, locale: baseLocale });
    }
    const converted = roundForCurrency(amount * fxRate, tenantCurrencyCode);
    return formatMoney(converted, { currency: tenantCurrencyCode, locale: baseLocale });
  }

  const mappingCountFor = (packageId: string) =>
    mappings.filter((m) => m.package_id === packageId).length;

  onMount(() => {
    if (!$can('read', 'isp_packages') && !$can('manage', 'isp_packages')) {
      goto('/unauthorized');
      return;
    }
    void load();
  });

  async function load() {
    loading = true;
    try {
      const publicSettings = await api.settings.getPublicSettings();
      if (publicSettings?.base_currency_code || publicSettings?.currency_code) {
        baseCurrencyCode = String(publicSettings.base_currency_code || publicSettings.currency_code).toUpperCase();
      }
      if (publicSettings?.default_locale) baseLocale = String(publicSettings.default_locale);

      fxRate = null;
      if (tenantCurrencyCode && baseCurrencyCode && tenantCurrencyCode !== baseCurrencyCode) {
        try {
          const res = await api.payment.getFxRate(baseCurrencyCode, tenantCurrencyCode);
          fxRate = Number(res.rate) || null;
        } catch {
          fxRate = null;
        }
      }

      await Promise.all([loadRouters(), loadPackages(), loadMappings()]);
    } finally {
      loading = false;
    }
  }

  async function loadRouters() {
    routers = (await api.mikrotik.routers.list()) as any;
  }

  async function loadPackages() {
    const res = await api.ispPackages.packages.list({
      q: q.trim() || undefined,
      page: packagePage + 1,
      per_page: packagePageSize,
    });
    packages = res.data || [];
    total = Number(res.total || 0);
  }

  async function loadMappings() {
    mappings = await api.ispPackages.routerMappings.list();
  }

  function openCreate() {
    if (!$can('manage', 'isp_packages')) return;
    editingPkg = null;
    pkgName = '';
    pkgDesc = '';
    pkgFeatures = [];
    pkgFeatureInput = '';
    pkgActive = true;
    pkgPriceMonthly = 0;
    pkgPriceYearly = 0;
    pkgYearlyEnabled = false;

    pkgMapEnabled = false;
    pkgMapRouterId = '';
    pkgMapProfile = '';
    pkgMapPool = '';
    pkgProfileSuggestions = [];
    pkgPoolSuggestions = [];
    pkgFormTab = 'details';

    showPkgModal = true;
  }

  function openEdit(p: IspPackage) {
    if (!$can('manage', 'isp_packages')) return;
    editingPkg = p;
    pkgName = p.name;
    pkgDesc = p.description || '';
    pkgFeatures = Array.isArray(p.features) ? [...p.features] : [];
    pkgFeatureInput = '';
    pkgActive = Boolean(p.is_active);
    pkgPriceMonthly = Number(p.price_monthly || 0);
    pkgPriceYearly = Number(p.price_yearly || 0);
    pkgYearlyEnabled = Number(p.price_yearly || 0) > 0;

    pkgMapEnabled = false;
    pkgMapRouterId = '';
    pkgMapProfile = '';
    pkgMapPool = '';
    pkgProfileSuggestions = [];
    pkgPoolSuggestions = [];
    pkgFormTab = 'details';

    showPkgModal = true;
  }

  async function savePackage() {
    if (saving) return;
    if (!pkgName.trim()) return;
    if (!(Number(pkgPriceMonthly) > 0)) {
      toast.error($t('admin.network.packages.validation.monthly_required') || 'Monthly price is required and must be greater than 0.');
      return;
    }
    if (pkgYearlyEnabled && !(Number(pkgPriceYearly) > 0)) {
      toast.error($t('admin.network.packages.validation.yearly_required') || 'Yearly price must be greater than 0 when enabled.');
      return;
    }
    saving = true;

    try {
      const wasCreate = !editingPkg;
      let pkg = editingPkg;
      const payload = {
        name: pkgName.trim(),
        description: pkgDesc.trim() || null,
        features: pkgFeatures,
        is_active: pkgActive,
        price_monthly: Number(pkgPriceMonthly),
        price_yearly: pkgYearlyEnabled ? Number(pkgPriceYearly) : 0,
      };
      if (pkg) {
        pkg = await api.ispPackages.packages.update(pkg.id, payload);
        editingPkg = pkg;
      } else {
        pkg = await api.ispPackages.packages.create(payload);
        // If mapping fails, keep modal open but switch to edit mode so we don't create duplicates on retry.
        editingPkg = pkg;
      }

      if (pkgMapEnabled && pkgMapRouterId && pkgMapProfile.trim()) {
        await api.ispPackages.routerMappings.upsert({
          router_id: pkgMapRouterId,
          package_id: pkg.id,
          router_profile_name: pkgMapProfile.trim(),
          address_pool: pkgMapPool.trim() || null,
        });
      }

      toast.success(
        wasCreate
          ? ($t('admin.network.packages.toasts.created') || 'Package created')
          : ($t('admin.network.packages.toasts.updated') || 'Package updated'),
      );

      showPkgModal = false;
      await Promise.all([loadPackages(), loadMappings()]);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      saving = false;
    }
  }

  async function deletePackage(p: IspPackage) {
    if (!$can('manage', 'isp_packages')) return;
    if (!confirm($t('admin.network.packages.confirm_delete') || 'Delete this package?')) return;
    try {
      await api.ispPackages.packages.delete(p.id);
      toast.success($t('common.deleted') || 'Deleted');
      await Promise.all([loadPackages(), loadMappings()]);
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function openMapping(p: IspPackage) {
    if (!$can('manage', 'isp_packages')) return;
    mapPkg = p;
    mapRouterId = '';
    mapProfile = '';
    mapPool = '';
    profileSuggestions = [];
    poolSuggestions = [];
    showMapModal = true;
  }

  async function loadPkgRouterMeta(routerId: string) {
    if (!routerId) {
      pkgProfileSuggestions = [];
      pkgPoolSuggestions = [];
      return;
    }
    pkgLoadingMeta = true;
    try {
      const [profiles, pools] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId),
        api.mikrotik.routers.ipPools(routerId),
      ]);
      pkgProfileSuggestions = (profiles || []).map((x: any) => ({ id: x.id, name: x.name }));
      pkgPoolSuggestions = (pools || []).map((x: any) => ({ id: x.id, name: x.name }));
    } finally {
      pkgLoadingMeta = false;
    }
  }

  async function loadRouterMeta(routerId: string) {
    if (!routerId) {
      profileSuggestions = [];
      poolSuggestions = [];
      return;
    }
    loadingMeta = true;
    try {
      const [profiles, pools] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId),
        api.mikrotik.routers.ipPools(routerId),
      ]);
      profileSuggestions = (profiles || []).map((p: any) => ({ id: p.id, name: p.name }));
      poolSuggestions = (pools || []).map((p: any) => ({ id: p.id, name: p.name }));
    } finally {
      loadingMeta = false;
    }
  }

  async function saveMapping() {
    if (saving) return;
    if (!mapPkg || !mapRouterId || !mapProfile.trim()) return;
    saving = true;
    try {
      await api.ispPackages.routerMappings.upsert({
        router_id: mapRouterId,
        package_id: mapPkg.id,
        router_profile_name: mapProfile.trim(),
        address_pool: mapPool.trim() || null,
      });
      toast.success($t('admin.network.packages.toasts.mapping_saved') || 'Mapping saved');
      showMapModal = false;
      await loadMappings();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      saving = false;
    }
  }

  function addFeature() {
    const v = pkgFeatureInput.trim();
    if (!v) return;
    if (pkgFeatures.some((x) => x.toLowerCase() === v.toLowerCase())) {
      pkgFeatureInput = '';
      return;
    }
    pkgFeatures = [...pkgFeatures, v];
    pkgFeatureInput = '';
  }

  function removeFeature(idx: number) {
    pkgFeatures = pkgFeatures.filter((_, i) => i !== idx);
  }
</script>

<div class="page-content fade-in">
  <section class="hero">
    <div class="hero-bg"></div>
    <div class="hero-inner">
      <div class="hgroup">
        <div class="kicker">
          <span class="dot"></span>
          {$t('sidebar.sections.network') || 'Network'}
        </div>
        <h1 class="h1">{$t('admin.network.packages.title') || 'Packages'}</h1>
        <div class="sub">
          {$t('admin.network.packages.subtitle') ||
            'Create internet packages and map them to router PPP profiles (per-router).'}
        </div>
      </div>

      <div class="hero-actions">
        <div class="search">
          <Icon name="search" size={16} />
          <input
            class="search-input"
            value={q}
            oninput={(e) => {
              q = (e.currentTarget as HTMLInputElement).value;
              packagePage = 0;
              packageTableVersion += 1;
              void loadPackages();
            }}
            placeholder={$t('admin.network.packages.search') || 'Search packages...'}
          />
          {#if q.trim()}
            <button class="clear" type="button" onclick={() => { q = ''; packagePage = 0; packageTableVersion += 1; void loadPackages(); }}>
              <Icon name="x" size={14} />
            </button>
          {/if}
        </div>

        <div class="hero-buttons">
          <button class="btn btn-secondary" onclick={load} disabled={loading}>
            <Icon name="refresh-cw" size={16} />
            {$t('common.refresh') || 'Refresh'}
          </button>
          {#if $can('manage', 'isp_packages')}
            <button class="btn btn-primary" onclick={openCreate}>
              <Icon name="plus" size={16} />
              {$t('admin.network.packages.actions.add') || 'Add package'}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </section>

  <div class="card table-card">
    <TableToolbar showSearch={false}>
      {#snippet actions()}
        <span class="muted">{packages.length} {$t('common.results') || 'results'}</span>
      {/snippet}
    </TableToolbar>

    {#key packageTableVersion}
      <Table
        columns={columns}
        data={packages}
        loading={loading}
        emptyText={$t('admin.network.packages.empty') || 'No packages.'}
        pagination
        serverSide
        count={total}
        pageSize={packagePageSize}
        onchange={(nextPage) => {
          packagePage = nextPage;
          void loadPackages();
        }}
        onpageSizeChange={(nextSize) => {
          packagePageSize = nextSize;
          packagePage = 0;
          packageTableVersion += 1;
          void loadPackages();
        }}
      >
        {#snippet cell({ item, key })}
        {@const row = item as IspPackage}
        {#if key === 'name'}
          <div class="stack">
            <div class="name">{row.name}</div>
            {#if row.description}
              <div class="meta">{row.description}</div>
            {/if}
            {#if row.features?.length}
              <div class="feature-list">
                {#each row.features.slice(0, 4) as f}
                  <span class="feature-chip">{f}</span>
                {/each}
                {#if row.features.length > 4}
                  <span class="feature-chip more">+{row.features.length - 4}</span>
                {/if}
              </div>
            {/if}
          </div>
        {:else if key === 'price'}
          <div class="stack">
            <div class="mono">{formatDisplayPrice(Number(row.price_monthly || 0))}<span class="unit">/mo</span></div>
            <div class="mono">{formatDisplayPrice(Number(row.price_yearly || 0))}<span class="unit">/yr</span></div>
            {#if tenantCurrencyCode !== baseCurrencyCode}
              <div class="meta">{formatBasePrice(Number(row.price_monthly || 0))}/mo</div>
              <div class="meta">{formatBasePrice(Number(row.price_yearly || 0))}/yr</div>
            {/if}
          </div>
        {:else if key === 'status'}
          {#if row.is_active}
            <span class="badge ok">{$t('common.active') || 'Active'}</span>
          {:else}
            <span class="badge warn">{$t('common.disabled') || 'Disabled'}</span>
          {/if}
        {:else if key === 'mappings'}
          <span class="pill mono">{mappingCountFor(row.id)}</span>
        {:else if key === 'actions'}
          <div class="row-actions">
            {#if $can('manage', 'isp_packages')}
              <button class="btn-icon" title={$t('admin.network.packages.actions.map') || 'Map to router'} onclick={() => openMapping(row)}>
                <Icon name="router" size={16} />
              </button>
              <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => openEdit(row)}>
                <Icon name="edit" size={16} />
              </button>
              <button class="btn-icon danger" title={$t('common.delete') || 'Delete'} onclick={() => deletePackage(row)}>
                <Icon name="trash-2" size={16} />
              </button>
            {/if}
          </div>
        {:else}
          {item[key] ?? ''}
        {/if}
        {/snippet}
      </Table>
    {/key}
  </div>
</div>

<Modal
  show={showPkgModal}
  title={editingPkg ? ($t('admin.network.packages.actions.edit') || 'Edit package') : ($t('admin.network.packages.actions.add') || 'Add package')}
  width="640px"
  onclose={() => (showPkgModal = false)}
>
  <div class="form">
    <div class="form-tabs" role="tablist" aria-label="Package tabs">
      <button
        class="tab-btn"
        class:active={pkgFormTab === 'details'}
        type="button"
        onclick={() => (pkgFormTab = 'details')}
      >
        {$t('admin.network.packages.tabs.details') || 'Details'}
      </button>
      <button
        class="tab-btn"
        class:active={pkgFormTab === 'features'}
        type="button"
        onclick={() => (pkgFormTab = 'features')}
      >
        {$t('admin.network.packages.tabs.features') || 'Features'} ({pkgFeatures.length})
      </button>
    </div>

    {#if pkgFormTab === 'details'}
      <label>
        <span>{$t('admin.network.packages.fields.name') || 'Name'}</span>
        <input class="input" bind:value={pkgName} />
      </label>

      <label>
        <span>{$t('admin.network.packages.fields.description') || 'Description'}</span>
        <input class="input" bind:value={pkgDesc} />
      </label>

      <label>
        <span>{$t('admin.network.packages.fields.price_monthly') || 'Monthly price'} ({tenantCurrencyCode})</span>
        <div class="price-input-wrap">
          <input class="input mono with-addon" type="number" min="0" step="0.01" bind:value={pkgPriceMonthly} required />
          <span class="currency-addon">{tenantCurrencyCode}</span>
        </div>
      </label>

      <div class="toggle-row">
        <div class="toggle-text">
          <div class="toggle-title">{$t('admin.network.packages.fields.enable_yearly') || 'Enable yearly price'}</div>
          <div class="toggle-sub">
            {$t('admin.network.packages.fields.enable_yearly_hint') || 'Turn on if this package has yearly billing.'}
          </div>
        </div>
        <Toggle bind:checked={pkgYearlyEnabled} ariaLabel={$t('admin.network.packages.fields.enable_yearly') || 'Enable yearly price'} />
      </div>

      <label>
        <span>{$t('admin.network.packages.fields.price_yearly') || 'Yearly price'} ({tenantCurrencyCode})</span>
        <div class="price-input-wrap">
          <input class="input mono with-addon" type="number" min="0" step="0.01" bind:value={pkgPriceYearly} disabled={!pkgYearlyEnabled} />
          <span class="currency-addon">{tenantCurrencyCode}</span>
        </div>
        <div class="field-hint">
          {$t('admin.network.packages.fields.currency_active') || 'Active currency'}: <strong>{tenantCurrencyCode}</strong>
          {#if tenantCurrencyCode !== baseCurrencyCode}
            · {$t('admin.network.packages.fields.currency_base') || 'Base currency'}: <strong>{baseCurrencyCode}</strong>
          {/if}
        </div>
        <div class="field-hint">
          {$t('admin.network.packages.fields.price_hint') || 'Stored in base currency; displayed in your tenant currency when possible.'}
          {#if tenantCurrencyCode !== baseCurrencyCode}
            <span class="hint-inline">Preview: {formatDisplayPrice(Number(pkgPriceMonthly || 0))}/mo, {formatDisplayPrice(Number(pkgPriceYearly || 0))}/yr</span>
          {/if}
        </div>
      </label>

      <div class="toggle-row">
        <div class="toggle-text">
          <div class="toggle-title">{$t('admin.network.packages.fields.active') || 'Active'}</div>
          <div class="toggle-sub">
            {$t('admin.network.packages.fields.active_hint') || 'Inactive packages will be hidden from selection.'}
          </div>
        </div>
        <Toggle bind:checked={pkgActive} ariaLabel={$t('admin.network.packages.fields.active') || 'Active'} />
      </div>

      <div class="toggle-row">
        <div class="toggle-text">
          <div class="toggle-title">{$t('admin.network.packages.mapping.inline_title') || 'Map to router now'}</div>
          <div class="toggle-sub">
            {$t('admin.network.packages.mapping.inline_hint') || 'Optional: prefill router profile/pool for this package (per-router).'}
          </div>
        </div>
        <Toggle bind:checked={pkgMapEnabled} ariaLabel={$t('admin.network.packages.mapping.inline_title') || 'Map to router now'} />
      </div>

      {#if pkgMapEnabled}
        <div class="grid2">
          <label>
            <span>{$t('admin.network.packages.mapping.router') || 'Router'}</span>
            <Select2
              bind:value={pkgMapRouterId}
              options={routerOptions}
              placeholder={($t('common.select') || 'Select') + '...'}
              width="100%"
              maxItems={5000}
              searchPlaceholder={$t('common.search') || 'Search'}
              noResultsText={$t('common.no_results') || 'No results'}
              onchange={() => {
                pkgMapProfile = '';
                pkgMapPool = '';
                void loadPkgRouterMeta(pkgMapRouterId);
              }}
            />
          </label>
          <label>
            <span>{$t('admin.network.packages.mapping.profile') || 'Router PPP Profile'}</span>
            <Select2
              bind:value={pkgMapProfile}
              options={pkgProfileOptions}
              placeholder={($t('common.select') || 'Select') + '...'}
              width="100%"
              disabled={!pkgMapRouterId || pkgProfileOptions.length === 0}
              maxItems={5000}
              searchPlaceholder={$t('common.search') || 'Search'}
              noResultsText={$t('common.no_results') || 'No results'}
            />
          </label>
        </div>

        <div class="grid2">
          <label>
            <span>{$t('admin.network.packages.mapping.pool') || 'Address pool (optional)'}</span>
            <Select2
              bind:value={pkgMapPool}
              options={pkgPoolOptions}
              placeholder={($t('common.select') || 'Select') + '...'}
              width="100%"
              disabled={!pkgMapRouterId || pkgPoolOptions.length === 0}
              maxItems={5000}
              searchPlaceholder={$t('common.search') || 'Search'}
              noResultsText={$t('common.no_results') || 'No results'}
            />
          </label>
          <div></div>
        </div>

        {#if pkgLoadingMeta}
          <div class="hint">
            <span class="spin"><Icon name="refresh-cw" size={14} /></span>
            <span>{$t('common.loading') || 'Loading...'} suggestions…</span>
          </div>
        {/if}
      {/if}
    {:else}
      <label>
        <span>{$t('admin.network.packages.fields.features') || 'Features'}</span>
        <div class="feature-input-row">
          <input
            class="input"
            bind:value={pkgFeatureInput}
            placeholder={$t('admin.network.packages.fields.feature_placeholder') || 'Add feature and press Enter'}
            onkeydown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                addFeature();
              }
            }}
          />
          <button class="btn btn-secondary" type="button" onclick={addFeature}>
            <Icon name="plus" size={14} />
            {$t('admin.network.packages.actions.add_feature') || 'Add feature'}
          </button>
        </div>
        {#if pkgFeatures.length > 0}
          <div class="feature-list">
            {#each pkgFeatures as f, i}
              <span class="feature-chip">
                {f}
                <button type="button" class="feature-remove" onclick={() => removeFeature(i)} aria-label="remove feature">
                  <Icon name="x" size={12} />
                </button>
              </span>
            {/each}
          </div>
        {:else}
          <div class="field-hint">{$t('admin.network.packages.fields.features_empty') || 'No features yet.'}</div>
        {/if}
      </label>
    {/if}

    <div class="actions">
      <button class="btn btn-secondary" type="button" onclick={() => (showPkgModal = false)} disabled={saving}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn btn-primary" type="button" onclick={savePackage} disabled={saving || !pkgName.trim() || !(Number(pkgPriceMonthly) > 0) || (pkgYearlyEnabled && !(Number(pkgPriceYearly) > 0)) || (pkgMapEnabled && (!pkgMapRouterId || !pkgMapProfile.trim()))}>
        <Icon name="save" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<Modal
  show={showMapModal}
  title={$t('admin.network.packages.mapping.title') || 'Router Mapping'}
  width="760px"
  onclose={() => (showMapModal = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.network.packages.mapping.package') || 'Package'}</span>
        <input class="input" value={mapPkg?.name || ''} disabled />
      </label>
      <label>
        <span>{$t('admin.network.packages.mapping.router') || 'Router'}</span>
        <Select2
          bind:value={mapRouterId}
          options={routerOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => void loadRouterMeta(mapRouterId)}
        />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.network.packages.mapping.profile') || 'Router PPP Profile'}</span>
        <Select2
          bind:value={mapProfile}
          options={mapProfileOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!mapRouterId || mapProfileOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
      <label>
        <span>{$t('admin.network.packages.mapping.pool') || 'Address pool (optional)'}</span>
        <Select2
          bind:value={mapPool}
          options={mapPoolOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!mapRouterId || mapPoolOptions.length === 0}
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
    </div>

    {#if loadingMeta}
      <div class="hint">
        <span class="spin"><Icon name="refresh-cw" size={14} /></span>
        <span>{$t('common.loading') || 'Loading...'} suggestions…</span>
      </div>
    {/if}

    <div class="actions">
      <button class="btn btn-secondary" type="button" onclick={() => (showMapModal = false)} disabled={saving}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn btn-primary" type="button" onclick={saveMapping} disabled={saving || !mapPkg || !mapRouterId || !mapProfile.trim()}>
        <Icon name="save" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .hero {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 22px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.02);
    box-shadow: var(--shadow-md);
    margin-bottom: 1rem;
  }

  .hero-bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(900px 220px at 10% 0%, rgba(99, 102, 241, 0.26), transparent 60%),
      radial-gradient(900px 220px at 85% 30%, rgba(16, 185, 129, 0.12), transparent 60%),
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0.01));
  }

  :global([data-theme='light']) .hero-bg {
    background:
      radial-gradient(900px 220px at 10% 0%, rgba(99, 102, 241, 0.18), transparent 60%),
      radial-gradient(900px 220px at 85% 30%, rgba(16, 185, 129, 0.08), transparent 60%),
      linear-gradient(180deg, rgba(0, 0, 0, 0.02), rgba(0, 0, 0, 0.01));
  }

  .hero-inner {
    position: relative;
    padding: 22px 22px 18px;
    display: grid;
    gap: 14px;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-weight: 850;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
    font-size: 0.75rem;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: rgba(99, 102, 241, 0.9);
    box-shadow: 0 0 0 4px rgba(99, 102, 241, 0.14);
  }

  .h1 {
    margin: 6px 0 0;
    font-size: 2rem;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .sub {
    margin-top: 6px;
    color: var(--text-secondary);
    max-width: 760px;
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

  .unit {
    margin-left: 0.25rem;
    color: var(--text-secondary);
    font-size: 0.85em;
  }

  .hero-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px 12px;
    min-width: min(560px, 100%);
    color: var(--text-secondary);
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.95rem;
  }

  .clear {
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--text-secondary);
    display: grid;
    place-items: center;
  }

  .hero-buttons {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
  }

  .stack {
    display: grid;
    gap: 4px;
  }

  .name {
    font-weight: 900;
    color: var(--text-primary);
  }

  .meta {
    color: var(--text-secondary);
    font-size: 0.9rem;
    max-width: 720px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .badge.ok {
    background: rgba(34, 197, 94, 0.12);
    color: rgba(34, 197, 94, 0.95);
    border-color: rgba(34, 197, 94, 0.28);
  }

  .badge.warn {
    background: rgba(245, 158, 11, 0.12);
    color: rgba(245, 158, 11, 0.95);
    border-color: rgba(245, 158, 11, 0.28);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 6px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-hover), transparent 18%);
    color: var(--text-secondary);
    font-weight: 800;
    font-size: 0.78rem;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
      'Courier New', monospace;
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .btn-icon {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.45rem 0.5rem;
    cursor: pointer;
  }

  .btn-icon:hover {
    background: var(--bg-hover);
  }

  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }

  .form {
    display: grid;
    gap: 0.9rem;
  }

  .form-tabs {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.2rem;
  }

  .tab-btn {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-secondary);
    border-radius: 10px;
    padding: 0.5rem 0.85rem;
    font-weight: 800;
    cursor: pointer;
  }

  .tab-btn.active {
    color: var(--text-primary);
    border-color: rgba(99, 102, 241, 0.45);
    background: rgba(99, 102, 241, 0.12);
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

  .feature-chip.more {
    color: var(--text-primary);
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
    .hero-buttons {
      width: 100%;
    }
    .hero-buttons :global(.btn) {
      width: 100%;
      justify-content: center;
    }
  }
</style>
