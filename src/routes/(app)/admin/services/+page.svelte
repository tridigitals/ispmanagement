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
  import Table from '$lib/components/ui/Table.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { loadServicesRouterMappingHelpers } from './servicesPageDeferredModules';
  import { loadServicesDialogs } from './servicesPageModules';
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
  let servicesDialogsLoading = $state(false);
  let ServicesDialogsComponent = $state<any>(null);

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

  async function ensureServicesDialogsLoaded() {
    if (ServicesDialogsComponent || servicesDialogsLoading) return;

    servicesDialogsLoading = true;
    try {
      const modules = await loadServicesDialogs();
      ServicesDialogsComponent = modules.ServicesDialogsComponent;
    } finally {
      servicesDialogsLoading = false;
    }
  }

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

  async function openCreate() {
    if (!$can('manage', 'isp_packages')) return;
    await ensureServicesDialogsLoaded();
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

  async function openEdit(p: IspPackage) {
    if (!$can('manage', 'isp_packages')) return;
    await ensureServicesDialogsLoaded();
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
    const {
      getPackageRouterMappingErrorFallback,
      getPackageRouterMappingReferenceError,
    } = await loadServicesRouterMappingHelpers();

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
        const mappingReferenceError = getPackageRouterMappingReferenceError({
          routerId: pkgMapRouterId,
          profileName: pkgMapProfile,
          profileSuggestions: pkgProfileSuggestions,
          poolName: pkgMapPool,
          poolSuggestions: pkgPoolSuggestions,
        });
        if (mappingReferenceError) {
          throw new Error(mappingReferenceError);
        }

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
      toast.error(getPackageRouterMappingErrorFallback(e?.message || e));
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
    await ensureServicesDialogsLoaded();
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
      const { getAvailableRouterNameSuggestions } = await loadServicesRouterMappingHelpers();
      const [profiles, pools] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId),
        api.mikrotik.routers.ipPools(routerId),
      ]);
      pkgProfileSuggestions = getAvailableRouterNameSuggestions(profiles || []).map((name, index) => ({
        id: `${index}:${name}`,
        name,
      }));
      pkgPoolSuggestions = getAvailableRouterNameSuggestions(pools || []).map((name, index) => ({
        id: `${index}:${name}`,
        name,
      }));
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
      const { getAvailableRouterNameSuggestions } = await loadServicesRouterMappingHelpers();
      const [profiles, pools] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId),
        api.mikrotik.routers.ipPools(routerId),
      ]);
      profileSuggestions = getAvailableRouterNameSuggestions(profiles || []).map((name, index) => ({
        id: `${index}:${name}`,
        name,
      }));
      poolSuggestions = getAvailableRouterNameSuggestions(pools || []).map((name, index) => ({
        id: `${index}:${name}`,
        name,
      }));
    } finally {
      loadingMeta = false;
    }
  }

  async function saveMapping() {
    if (saving) return;
    if (!mapPkg || !mapRouterId || !mapProfile.trim()) return;
    saving = true;
    const {
      getPackageRouterMappingErrorFallback,
      getPackageRouterMappingReferenceError,
    } = await loadServicesRouterMappingHelpers();
    try {
      const mappingReferenceError = getPackageRouterMappingReferenceError({
        routerId: mapRouterId,
        profileName: mapProfile,
        profileSuggestions,
        poolName: mapPool,
        poolSuggestions,
      });
      if (mappingReferenceError) {
        throw new Error(mappingReferenceError);
      }

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
      toast.error(getPackageRouterMappingErrorFallback(e?.message || e));
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
        <button class="btn" type="button" onclick={() => void openCreate()}>
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
                <button class="btn-icon" title={$t('admin.network.packages.actions.map') || 'Map to router'} onclick={() => void openMapping(row)}>
                  <Icon name="router" size={16} />
                </button>
              {/if}
              <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => void openEdit(row)}>
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

{#if showServiceTypePicker || showPkgModal || showMapModal}
  {#if ServicesDialogsComponent}
    <ServicesDialogsComponent
      bind:showServiceTypePicker
      bind:showPkgModal
      bind:showMapModal
      {editingPkg}
      {saving}
      {serviceTypeCards}
      {startCreateWithType}
      bind:pkgFormTab
      bind:pkgFeatures
      {serviceTypeLabel}
      {pkgServiceType}
      bind:pkgName
      bind:pkgDesc
      {tenantCurrencyCode}
      bind:pkgPriceMonthly
      bind:pkgYearlyEnabled
      bind:pkgPriceYearly
      {baseCurrencyCode}
      {formatDisplayPrice}
      bind:pkgActive
      {isInternetType}
      bind:pkgMapEnabled
      {routerOptions}
      bind:pkgMapRouterId
      bind:pkgMapProfile
      bind:pkgMapPool
      {loadPkgRouterMeta}
      {pkgProfileOptions}
      {pkgPoolOptions}
      {pkgLoadingMeta}
      {serviceTypeFeatureSuggestions}
      {addFeatureIfMissing}
      bind:pkgFeatureInput
      {addFeature}
      {removeFeature}
      {savePackage}
      {mapPkg}
      bind:mapRouterId
      bind:mapProfile
      bind:mapPool
      {loadRouterMeta}
      {mapProfileOptions}
      {mapPoolOptions}
      {loadingMeta}
      {saveMapping}
    />
  {:else}
    <div class="modal-loading-shell" aria-busy={servicesDialogsLoading}>
      {$t('common.loading') || 'Loading...'}
    </div>
  {/if}
{/if}

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

  .modal-loading-shell {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(2, 6, 23, 0.56);
    color: var(--text-primary);
    font-weight: 800;
    backdrop-filter: blur(8px);
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

  label {
    display: grid;
    gap: 0.35rem;
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

  @media (max-width: 768px) {
    .page-content {
      padding: 16px;
    }
  }
</style>
