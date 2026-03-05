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
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

  type RouterRow = { id: string; name: string };
  type ProfileSuggestion = { id: string; name: string };
  type PoolSuggestion = { id: string; name: string };
  type ServiceType = 'internet_pppoe' | 'hotspot' | 'vpn';
  type PackageSortBy = 'name' | 'type' | 'price' | 'status' | 'mappings';

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);

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
  let packageSortBy = $state<PackageSortBy>('name');
  let packageSortDirection = $state<'asc' | 'desc'>('asc');

  let routers = $state<RouterRow[]>([]);
  let mappings = $state<IspPackageRouterMappingView[]>([]);

  // Create/Edit package
  let showServiceTypePicker = $state(false);
  let showPkgModal = $state(false);
  let editingPkg = $state<IspPackage | null>(null);
  let pkgServiceType = $state<ServiceType>('internet_pppoe');
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
    {
      key: 'name',
      label: $t('admin.services.columns.name') || $t('admin.network.packages.columns.name') || 'Name',
      sortable: true,
    },
    { key: 'type', label: $t('admin.services.columns.type') || 'Type', width: '140px', sortable: true },
    {
      key: 'price',
      label: $t('admin.services.columns.price') || $t('admin.network.packages.columns.price') || 'Price',
      width: '160px',
      sortable: true,
    },
    {
      key: 'status',
      label: $t('admin.services.columns.status') || $t('admin.network.packages.columns.status') || 'Status',
      width: '120px',
      sortable: true,
    },
    {
      key: 'mappings',
      label: $t('admin.services.columns.mappings') || $t('admin.network.packages.columns.mappings') || 'Mapped',
      width: '140px',
      sortable: true,
    },
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
  const firstMappingFor = (packageId: string) =>
    mappings.find((m) => m.package_id === packageId) || null;
  const normalizeServiceType = (value?: string | null): ServiceType => {
    const key = String(value || 'internet_pppoe').toLowerCase();
    if (key === 'hotspot') return 'hotspot';
    if (key === 'vpn') return 'vpn';
    return 'internet_pppoe';
  };
  const isInternetType = (value?: string | null) => normalizeServiceType(value) === 'internet_pppoe';
  const serviceTypeLabel = (value?: string | null) => {
    const key = String(value || 'internet_pppoe').toLowerCase();
    if (key === 'hotspot') return $t('admin.services.types.hotspot') || 'Hotspot';
    if (key === 'vpn') return $t('admin.services.types.vpn') || 'VPN';
    return $t('admin.services.types.internet_pppoe') || 'Internet / PPPoE';
  };
  const serviceTypeCards = $derived.by(() => [
    {
      value: 'internet_pppoe' as ServiceType,
      icon: 'router',
      title: $t('admin.services.types.internet_pppoe') || 'Internet / PPPoE',
      subtitle:
        $t('admin.services.type_picker.internet_subtitle') ||
        'Fixed internet service with PPPoE provisioning and optional router profile mapping.',
      tags: [
        $t('admin.services.type_picker.tag_provisioning') || 'Provisioning',
        $t('admin.services.type_picker.tag_mapping') || 'Router mapping',
      ],
    },
    {
      value: 'hotspot' as ServiceType,
      icon: 'wifi',
      title: $t('admin.services.types.hotspot') || 'Hotspot',
      subtitle:
        $t('admin.services.type_picker.hotspot_subtitle') ||
        'Voucher or captive portal service for shared/public wireless access zones.',
      tags: [
        $t('admin.services.type_picker.tag_shared_access') || 'Shared access',
        $t('admin.services.type_picker.tag_portal_ready') || 'Portal ready',
      ],
    },
    {
      value: 'vpn' as ServiceType,
      icon: 'shield',
      title: $t('admin.services.types.vpn') || 'VPN',
      subtitle:
        $t('admin.services.type_picker.vpn_subtitle') ||
        'Secure tunnel service for branch office, remote team, or dedicated private access.',
      tags: [
        $t('admin.services.type_picker.tag_secure_tunnel') || 'Secure tunnel',
        $t('admin.services.type_picker.tag_private_access') || 'Private access',
      ],
    },
  ]);
  const serviceTypeFeatureSuggestions: Record<ServiceType, string[]> = {
    internet_pppoe: ['PPPoE authentication', 'Dedicated bandwidth', '24/7 monitoring'],
    hotspot: ['Captive portal login', 'Voucher support', 'Session/time limit'],
    vpn: ['Encrypted tunnel', 'Site-to-site ready', 'Private subnet routing'],
  };

  function addFeatureIfMissing(value: string) {
    const trimmed = value.trim();
    if (!trimmed) return;
    if (pkgFeatures.some((x) => x.toLowerCase() === trimmed.toLowerCase())) return;
    pkgFeatures = [...pkgFeatures, trimmed];
  }

  function handlePackageSort(key: string) {
    const allowed: PackageSortBy[] = ['name', 'type', 'price', 'status', 'mappings'];
    if (!allowed.includes(key as PackageSortBy)) return;
    const typed = key as PackageSortBy;
    if (packageSortBy === typed) {
      packageSortDirection = packageSortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      packageSortBy = typed;
      packageSortDirection = typed === 'price' || typed === 'mappings' ? 'desc' : 'asc';
    }
    packagePage = 0;
    packageTableVersion += 1;
    void loadPackages();
  }

  function resetCreateForm(type: ServiceType) {
    pkgServiceType = type;
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
  }

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
      sort_by: packageSortBy,
      sort_dir: packageSortDirection,
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
    resetCreateForm('internet_pppoe');
    showServiceTypePicker = true;
  }

  function startCreateWithType(type: ServiceType) {
    editingPkg = null;
    resetCreateForm(type);
    showServiceTypePicker = false;
    showPkgModal = true;
  }

  function openEdit(p: IspPackage) {
    if (!$can('manage', 'isp_packages')) return;
    showServiceTypePicker = false;
    editingPkg = p;
    pkgServiceType = normalizeServiceType(p.service_type);
    pkgName = p.name;
    pkgDesc = p.description || '';
    pkgFeatures = Array.isArray(p.features) ? [...p.features] : [];
    pkgFeatureInput = '';
    pkgActive = Boolean(p.is_active);
    pkgPriceMonthly = Number(p.price_monthly || 0);
    pkgPriceYearly = Number(p.price_yearly || 0);
    pkgYearlyEnabled = Number(p.price_yearly || 0) > 0;
    pkgProfileSuggestions = [];
    pkgPoolSuggestions = [];

    const existing = firstMappingFor(p.id);
    if (isInternetType(pkgServiceType) && existing) {
      pkgMapEnabled = true;
      pkgMapRouterId = existing.router_id || '';
      pkgMapProfile = existing.router_profile_name || '';
      pkgMapPool = existing.address_pool || '';
      if (pkgMapRouterId) void loadPkgRouterMeta(pkgMapRouterId);
    } else {
      pkgMapEnabled = false;
      pkgMapRouterId = '';
      pkgMapProfile = '';
      pkgMapPool = '';
    }
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
      if (!isInternetType(pkgServiceType)) {
        pkgMapEnabled = false;
        pkgMapRouterId = '';
        pkgMapProfile = '';
        pkgMapPool = '';
      }
      const payload = {
        service_type: pkgServiceType,
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

      if (isInternetType(pkgServiceType) && pkgMapEnabled && pkgMapRouterId && pkgMapProfile.trim()) {
        await api.ispPackages.routerMappings.upsert({
          router_id: pkgMapRouterId,
          package_id: pkg.id,
          router_profile_name: pkgMapProfile.trim(),
          address_pool: pkgMapPool.trim() || null,
        });
      }

      toast.success(
        wasCreate
          ? ($t('admin.services.toasts.created') || $t('admin.network.packages.toasts.created') || 'Service created')
          : ($t('admin.services.toasts.updated') || $t('admin.network.packages.toasts.updated') || 'Service updated'),
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
    if (!confirm($t('admin.services.confirm_delete') || $t('admin.network.packages.confirm_delete') || 'Delete this service?')) return;
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
    if (!isInternetType(p.service_type)) {
      toast.error($t('admin.services.mapping.only_internet') || 'Router mapping is only available for Internet / PPPoE services.');
      return;
    }
    mapPkg = p;
    const existing = firstMappingFor(p.id);
    mapRouterId = existing?.router_id || '';
    mapProfile = existing?.router_profile_name || '';
    mapPool = existing?.address_pool || '';
    profileSuggestions = [];
    poolSuggestions = [];
    if (mapRouterId) await loadRouterMeta(mapRouterId);
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
  <NetworkPageHeader
    title={$t('admin.services.title') || $t('admin.network.packages.title') || 'Services'}
    subtitle={$t('admin.services.subtitle') || 'Create services and configure service-specific options.'}
  >
    {#snippet actions()}
      <button class="btn ghost" type="button" onclick={() => void load()} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      {#if $can('manage', 'isp_packages')}
        <button class="btn" type="button" onclick={openCreate}>
          <Icon name="plus" size={16} />
          {$t('admin.services.actions.add') || $t('admin.network.packages.actions.add') || 'Add service'}
        </button>
      {/if}
    {/snippet}
  </NetworkPageHeader>

  <div class="filters-wrap">
    <NetworkFilterPanel>
      <div class="control control-wide">
        <label for="packages-search">{$t('common.search') || 'Search'}</label>
        <label class="search-wrap" for="packages-search">
          <Icon name="search" size={14} />
          <input
            id="packages-search"
            type="text"
            placeholder={$t('admin.services.search') || $t('admin.network.packages.search') || 'Search services...'}
            value={q}
            oninput={(e) => {
              q = (e.currentTarget as HTMLInputElement).value;
              packagePage = 0;
              packageTableVersion += 1;
              void loadPackages();
            }}
          />
          {#if q.trim()}
            <button
              class="clear"
              type="button"
              onclick={() => {
                q = '';
                packagePage = 0;
                packageTableVersion += 1;
                void loadPackages();
              }}
              aria-label={$t('common.clear') || 'Clear'}
            >
              <Icon name="x" size={14} />
            </button>
          {/if}
        </label>
      </div>
    </NetworkFilterPanel>
  </div>

  <div class="table-wrap">
    <div class="table-top">
      <span class="muted">{total >= 0 ? total : packages.length} {$t('common.results') || 'results'}</span>
    </div>

    {#key packageTableVersion}
      <Table
        columns={columns}
        data={packages}
        loading={loading}
        emptyText={$t('admin.services.empty') || $t('admin.network.packages.empty') || 'No services.'}
        pagination
        serverSide
        count={total}
        pageSize={packagePageSize}
        sortKey={packageSortBy}
        sortDirection={packageSortDirection}
        onsort={handlePackageSort}
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
        {:else if key === 'type'}
          <span class="badge neutral">{serviceTypeLabel(row.service_type)}</span>
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
          {#if isInternetType(row.service_type)}
            <span class="pill mono">{mappingCountFor(row.id)}</span>
          {:else}
            <span class="meta">-</span>
          {/if}
        {:else if key === 'actions'}
          <div class="row-actions">
            {#if $can('manage', 'isp_packages')}
              {#if isInternetType(row.service_type)}
                <button class="btn-icon" title={$t('admin.network.packages.actions.map') || 'Map to router'} onclick={() => openMapping(row)}>
                  <Icon name="router" size={16} />
                </button>
              {/if}
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
  show={showServiceTypePicker}
  title={$t('admin.services.type_picker.title') || 'Choose Service Type'}
  width="860px"
  onclose={() => (showServiceTypePicker = false)}
>
  <div class="type-picker-wrap">
    <p class="type-picker-subtitle">
      {$t('admin.services.type_picker.subtitle') ||
        'Select the service category first, then continue to the detailed service form.'}
    </p>
    <div class="type-card-grid">
      {#each serviceTypeCards as card}
        <button
          type="button"
          class="type-card"
          onclick={() => startCreateWithType(card.value)}
        >
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
            {$t('admin.services.type_picker.continue') || 'Continue'}
            <Icon name="arrow-right" size={14} />
          </span>
        </button>
      {/each}
    </div>
  </div>
</Modal>

<Modal
  show={showPkgModal}
  title={editingPkg ? ($t('admin.services.actions.edit') || $t('admin.network.packages.actions.edit') || 'Edit service') : ($t('admin.services.actions.add') || $t('admin.network.packages.actions.add') || 'Add service')}
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
      <div class="selected-type-banner">
        <div class="selected-type-main">
          <span class="selected-type-label">{$t('admin.services.fields.service_type') || 'Service type'}</span>
          <span class="badge neutral">{serviceTypeLabel(pkgServiceType)}</span>
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
            {$t('admin.services.type_picker.change') || 'Change type'}
          </button>
        {/if}
      </div>

      <div class="type-hints">
        {#each serviceTypeFeatureSuggestions[pkgServiceType] as suggestion}
          <button
            type="button"
            class="hint-chip"
            onclick={() => addFeatureIfMissing(suggestion)}
            title={$t('admin.services.type_picker.add_as_feature') || 'Add as feature'}
          >
            <Icon name="plus" size={12} />
            {suggestion}
          </button>
        {/each}
      </div>

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

      {#if pkgYearlyEnabled}
        <label>
          <span>{$t('admin.network.packages.fields.price_yearly') || 'Yearly price'} ({tenantCurrencyCode})</span>
          <div class="price-input-wrap">
            <input class="input mono with-addon" type="number" min="0" step="0.01" bind:value={pkgPriceYearly} />
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
      {/if}

      <div class="toggle-row">
        <div class="toggle-text">
          <div class="toggle-title">{$t('admin.network.packages.fields.active') || 'Active'}</div>
          <div class="toggle-sub">
            {$t('admin.network.packages.fields.active_hint') || 'Inactive packages will be hidden from selection.'}
          </div>
        </div>
        <Toggle bind:checked={pkgActive} ariaLabel={$t('admin.network.packages.fields.active') || 'Active'} />
      </div>

      {#if isInternetType(pkgServiceType)}
        <div class="toggle-row">
          <div class="toggle-text">
            <div class="toggle-title">{$t('admin.network.packages.mapping.inline_title') || 'Map to router now'}</div>
            <div class="toggle-sub">
              {$t('admin.network.packages.mapping.inline_hint') || 'Optional: prefill router profile/pool for this package (per-router).'}
            </div>
          </div>
          <Toggle bind:checked={pkgMapEnabled} ariaLabel={$t('admin.network.packages.mapping.inline_title') || 'Map to router now'} />
        </div>
      {:else}
        <div class="field-hint">
          {$t('admin.services.mapping.not_required') || 'Router PPP profile mapping is not required for this service type.'}
        </div>
      {/if}

      {#if isInternetType(pkgServiceType) && pkgMapEnabled}
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
          <button class="btn ghost" type="button" onclick={addFeature}>
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
      <button class="btn ghost" type="button" onclick={() => (showPkgModal = false)} disabled={saving}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn" type="button" onclick={savePackage} disabled={saving || !pkgName.trim() || !(Number(pkgPriceMonthly) > 0) || (pkgYearlyEnabled && !(Number(pkgPriceYearly) > 0)) || (isInternetType(pkgServiceType) && pkgMapEnabled && (!pkgMapRouterId || !pkgMapProfile.trim()))}>
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
      <button class="btn ghost" type="button" onclick={() => (showMapModal = false)} disabled={saving}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn" type="button" onclick={saveMapping} disabled={saving || !mapPkg || !mapRouterId || !mapProfile.trim()}>
        <Icon name="save" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .page-content {
    padding: 28px;
    max-width: 1460px;
    margin: 0 auto;
  }

  .filters-wrap {
    margin-bottom: 12px;
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

  .search-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 10%);
  }

  .search-wrap input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.95rem;
    min-width: 0;
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

  .clear {
    border: 1px solid var(--border-color);
    background: transparent;
    cursor: pointer;
    color: var(--text-secondary);
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 10px;
  }

  .table-wrap {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 18px;
    overflow: hidden;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.2);
  }

  .table-top {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 82%, transparent);
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

  .badge.neutral {
    background: rgba(99, 102, 241, 0.12);
    color: rgba(199, 210, 254, 0.98);
    border-color: rgba(99, 102, 241, 0.32);
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
    background: linear-gradient(165deg, rgba(99, 102, 241, 0.1), rgba(15, 23, 42, 0.45));
    border-radius: 14px;
    padding: 0.95rem;
    color: var(--text-primary);
    display: grid;
    gap: 0.65rem;
    cursor: pointer;
    transition: border-color 0.2s ease, transform 0.2s ease, background 0.2s ease;
  }

  .type-card:hover {
    border-color: rgba(99, 102, 241, 0.45);
    transform: translateY(-2px);
    background: linear-gradient(165deg, rgba(99, 102, 241, 0.16), rgba(15, 23, 42, 0.6));
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
    .page-content {
      padding: 16px;
    }

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
  }
</style>
