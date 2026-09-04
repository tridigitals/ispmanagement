<script lang="ts">
  /*
    Layanan (paket ISP) v2.

    Versi lama: `(app)/admin/services/+page.svelte` (1.175 baris) +
    ServicesDialogs.svelte (930 baris) yang di-load dinamis dengan 40-an props.

    Temuan yang dikunci gelombang ini (dibuktikan di DB produksi 2026-09-04):

    1. HAPUS PAKET TERPAKAI -> 500 MENTAH.
       delete_package tidak mengecek referensi. customer_subscriptions dan
       dhcp_static_services ber-FK RESTRICT (error 23503 bocor apa adanya ke
       user — dibuktikan via psql); pppoe_accounts ber-FK SET NULL (paket
       546 akun terputus DIEM-DIEM); zone_offers CASCADE (penawaran raib).
       Kini server menolak lebih dulu dengan daftar pemakaian yang jelas.

    2. WILDCARD LIKE TIDAK DI-ESCAPE (bug yang sama dengan audit-logs).
       Mencari "%" dulu mencocokkan seluruh tabel. Pakai like_pattern() +
       description ikut dicari.

    Dialog dibuat ulang inline dengan ds/Field (stacked) — bukan meneruskan
    40 props ke komponen lama. Logika murni (tipe, tone, kurs, validasi,
    pesan error) pindah ke $lib/utils/serviceInsights (21 tes unit).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { IspPackage, IspPackageRouterMappingView } from '$lib/api/types';
  import { can, user } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { appSettings } from '$lib/stores/settings';
  import { formatMoney } from '$lib/utils/money';
  import { canAccessServiceCatalog } from '$lib/utils/serviceCatalogAccess';
  import { getAvailableRouterNameSuggestions } from '$lib/utils/packageRouterMeta';
  import {
    getPackageRouterMappingErrorFallback,
    getPackageRouterMappingReferenceError,
  } from '$lib/utils/packageRouterMapping';
  import Modal from '$lib/components/ui/Modal.svelte';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
  import {
    AppShell,
    AttentionPanel,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    Icon,
    PageHeader,
    RowActions,
    StatTile,
    type Column,
    type FieldOption,
  } from '$lib/components/ds';
  import type { RowAction } from '$lib/components/ds/RowActions.svelte';
  import {
    convertPrice,
    friendlyDeleteError,
    mappingAllowed,
    normalizeProvisioningType,
    normalizeServiceType,
    serviceTypeLabel,
    serviceTypeTone,
    validatePackageDraft,
    type ServiceType,
  } from '$lib/utils/serviceInsights';

  type RouterRow = { id: string; name: string };
  type Suggestion = { id: string; name: string };

  let rows = $state<IspPackage[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let pageNum = $state(1);
  const perPage = 20;
  let search = $state('');
  let sortBy = $state<'name' | 'type' | 'price' | 'status' | 'mappings'>('name');
  let sortDir = $state<'asc' | 'desc'>('asc');

  let routers = $state<RouterRow[]>([]);
  let mappings = $state<IspPackageRouterMappingView[]>([]);

  // kurs & mata uang (parity dengan legacy: tenant bisa beda dari basis)
  let baseCurrency = $state('IDR');
  let baseLocale = $state('en-US');
  let fxRate = $state<number | null>(null);
  const tenantCurrency = $derived(
    String($appSettings?.currency_code || baseCurrency).toUpperCase(),
  );

  function displayPrice(amount: number): string {
    const c = convertPrice(Number(amount || 0), baseCurrency, tenantCurrency, fxRate);
    return formatMoney(c.amount, { currency: c.currency, locale: baseLocale });
  }

  // ---- form create/edit ----
  let typePickerOpen = $state(false);
  let formOpen = $state(false);
  let editTarget = $state<IspPackage | null>(null);
  let saving = $state(false);
  let fType = $state<ServiceType>('internet_pppoe');
  let fProv = $state<'pppoe' | 'dhcp_static'>('pppoe');
  let fName = $state('');
  let fDesc = $state('');
  let fMonthly = $state(0);
  let fYearlyOn = $state(false);
  let fYearly = $state(0);
  let fActive = $state(true);
  let fFeatures = $state<string[]>([]);
  let fFeatureInput = $state('');
  let formErrors = $state<string[]>([]);

  // mapping inline saat create/edit (hanya Internet/PPPoE)
  let mapInline = $state(false);
  let mapRouter = $state('');
  let mapProfile = $state('');
  let mapPool = $state('');
  let mapIsolation = $state('');
  let profileSug = $state<Suggestion[]>([]);
  let poolSug = $state<Suggestion[]>([]);
  let loadingMeta = $state(false);

  // modal mapping berdiri sendiri
  let mapModalOpen = $state(false);
  let mapPkg = $state<IspPackage | null>(null);

  // hapus
  let deleteOpen = $state(false);
  let deleteTarget = $state<IspPackage | null>(null);

  const routerOptions: FieldOption[] = $derived(routers.map((r) => ({ label: r.name, value: r.id })));
  const profileOptions: FieldOption[] = $derived(sugOptions(profileSug, mapProfile));
  const poolOptions: FieldOption[] = $derived(sugOptions(poolSug, mapPool));
  function sugOptions(sug: Suggestion[], current: string): FieldOption[] {
    const base = sug.map((x) => ({ label: x.name, value: x.name }));
    const cur = current?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  }

  const canManage = $derived($can('manage', 'isp_packages'));

  const mappingCountFor = (pkgId: string) => mappings.filter((m) => m.package_id === pkgId).length;
  const firstMappingFor = (pkgId: string) => mappings.find((m) => m.package_id === pkgId) || null;

  const stats = $derived.by(() => {
    const aktif = rows.filter((p) => p.is_active).length;
    const pppoe = rows.filter((p) => normalizeServiceType(p.service_type) === 'internet_pppoe').length;
    const terpetakan = new Set(mappings.map((m) => m.package_id)).size;
    return { total, aktif, pppoe, terpetakan };
  });

  const attention = $derived.by(() => {
    const items: AttentionItem[] = [];
    const tanpaMap = rows.filter(
      (p) =>
        p.is_active &&
        mappingAllowed(p.service_type, p.provisioning_type) &&
        mappingCountFor(p.id) === 0,
    );
    if (tanpaMap.length) {
      items.push({
        icon: 'router',
        title: `${tanpaMap.length} paket PPPoE aktif belum dipetakan ke router`,
        detail: `Tanpa mapping, provisioning memakai profil default: ${tanpaMap
          .slice(0, 3)
          .map((p) => p.name)
          .join(', ')}${tanpaMap.length > 3 ? ', …' : ''}.`,
        action: 'Petakan sekarang',
      });
    }
    const nonaktif = rows.filter((p) => !p.is_active).length;
    if (nonaktif) {
      items.push({
        icon: 'zap',
        title: `${nonaktif} paket dinonaktifkan`,
        detail: 'Paket nonaktif tidak muncul di penawaran baru; langganan lama tetap berjalan.',
        action: 'Tinjau paket',
      });
    }
    return items;
  });

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  const columns: Column[] = [
    { key: 'name', label: 'Paket' },
    { key: 'type', label: 'Tipe', width: '170px' },
    { key: 'price', label: 'Harga', width: '190px' },
    { key: 'status', label: 'Status', width: '110px' },
    { key: 'mappings', label: 'Router', width: '110px' },
    { key: 'actions', label: '', width: '120px', align: 'right' },
  ];

  async function load() {
    loading = true;
    try {
      const pub = await api.settings.getPublicSettings();
      if (pub?.base_currency_code || pub?.currency_code) {
        baseCurrency = String(pub.base_currency_code || pub.currency_code).toUpperCase();
      }
      if (pub?.default_locale) baseLocale = String(pub.default_locale);
      fxRate = null;
      if (tenantCurrency && baseCurrency && tenantCurrency !== baseCurrency) {
        try {
          const res = await api.payment.getFxRate(baseCurrency, tenantCurrency);
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
    routers = (await api.mikrotik.routers.list()) as RouterRow[];
  }

  async function loadPackages() {
    const res = await api.ispPackages.packages.list({
      q: search.trim() || undefined,
      page: pageNum,
      per_page: perPage,
      sort_by: sortBy,
      sort_dir: sortDir,
    });
    rows = res.data || [];
    total = Number(res.total || 0);
  }

  async function loadMappings() {
    mappings = await api.ispPackages.routerMappings.list();
  }

  function resetForm(type: ServiceType) {
    fType = type;
    fProv = 'pppoe';
    fName = '';
    fDesc = '';
    fMonthly = 0;
    fYearlyOn = false;
    fYearly = 0;
    fActive = true;
    fFeatures = [];
    fFeatureInput = '';
    formErrors = [];
    mapInline = false;
    resetMappingFields();
  }

  function resetMappingFields() {
    mapRouter = '';
    mapProfile = '';
    mapPool = '';
    mapIsolation = '';
    profileSug = [];
    poolSug = [];
  }

  function openCreate() {
    if (!$can('manage', 'isp_packages')) return;
    typePickerOpen = true;
  }

  function startCreateWithType(type: ServiceType) {
    editTarget = null;
    resetForm(type);
    typePickerOpen = false;
    formOpen = true;
  }

  function openEdit(p: IspPackage) {
    if (!$can('manage', 'isp_packages')) return;
    editTarget = p;
    fType = normalizeServiceType(p.service_type);
    fProv = normalizeProvisioningType(p.provisioning_type);
    fName = p.name;
    fDesc = p.description || '';
    fMonthly = Number(p.price_monthly || 0);
    fYearly = Number(p.price_yearly || 0);
    fYearlyOn = fYearly > 0;
    fActive = Boolean(p.is_active);
    fFeatures = Array.isArray(p.features) ? [...p.features] : [];
    fFeatureInput = '';
    formErrors = [];
    const existing = firstMappingFor(p.id);
    if (mappingAllowed(p.service_type, p.provisioning_type) && existing) {
      mapInline = true;
      mapRouter = existing.router_id || '';
      mapProfile = existing.router_profile_name || '';
      mapPool = existing.address_pool || '';
      mapIsolation = existing.isolation_pool || '';
      if (mapRouter) void loadRouterMeta(mapRouter);
    } else {
      mapInline = false;
      resetMappingFields();
    }
    formOpen = true;
  }

  async function loadRouterMeta(routerId: string) {
    if (!routerId) {
      profileSug = [];
      poolSug = [];
      return;
    }
    loadingMeta = true;
    try {
      const [profiles, pools] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId),
        api.mikrotik.routers.ipPools(routerId),
      ]);
      profileSug = getAvailableRouterNameSuggestions(profiles || []).map((name, i) => ({
        id: `${i}:${name}`,
        name,
      }));
      poolSug = getAvailableRouterNameSuggestions(pools || []).map((name, i) => ({
        id: `${i}:${name}`,
        name,
      }));
    } finally {
      loadingMeta = false;
    }
  }

  function addFeature() {
    const v = fFeatureInput.trim();
    fFeatureInput = '';
    if (!v) return;
    if (fFeatures.some((x) => x.toLowerCase() === v.toLowerCase())) return;
    fFeatures = [...fFeatures, v];
  }

  function removeFeature(idx: number) {
    fFeatures = fFeatures.filter((_, i) => i !== idx);
  }

  async function savePackage() {
    if (saving) return;
    formErrors = validatePackageDraft({
      name: fName,
      priceMonthly: fMonthly,
      priceYearly: fYearly,
      yearlyEnabled: fYearlyOn,
    });
    if (formErrors.length) return;
    saving = true;
    try {
      const wasCreate = !editTarget;
      const payload = {
        service_type: fType,
        provisioning_type: fType === 'internet_pppoe' ? fProv : 'pppoe',
        name: fName.trim(),
        description: fDesc.trim() || null,
        features: fFeatures,
        is_active: fActive,
        price_monthly: Number(fMonthly),
        price_yearly: fYearlyOn ? Number(fYearly) : 0,
      };
      let pkg = editTarget;
      if (pkg) {
        pkg = await api.ispPackages.packages.update(pkg.id, payload);
        editTarget = pkg;
      } else {
        pkg = await api.ispPackages.packages.create(payload);
        // mapping gagal -> modal tetap terbuka dalam mode edit (tidak duplikat saat retry)
        editTarget = pkg;
      }

      if (mapInline && mappingAllowed(fType, fProv) && mapRouter && mapProfile.trim()) {
        const refErr = getPackageRouterMappingReferenceError({
          routerId: mapRouter,
          profileName: mapProfile,
          profileSuggestions: profileSug,
          poolName: mapPool,
          poolSuggestions: poolSug,
        });
        if (refErr) throw new Error(refErr);
        await api.ispPackages.routerMappings.upsert({
          router_id: mapRouter,
          package_id: pkg.id,
          router_profile_name: mapProfile.trim(),
          address_pool: mapPool.trim() || null,
          isolation_pool: mapIsolation.trim() || null,
        });
      }

      toast.success(wasCreate ? 'Paket dibuat' : 'Paket diperbarui');
      formOpen = false;
      await Promise.all([loadPackages(), loadMappings()]);
    } catch (e: any) {
      toast.error(
        getPackageRouterMappingErrorFallback(extractApiErrorMessage(e) || String(e?.message || e)),
      );
    } finally {
      saving = false;
    }
  }

  function openMapping(p: IspPackage) {
    if (!$can('manage', 'isp_packages')) return;
    if (!mappingAllowed(p.service_type, p.provisioning_type)) {
      toast.error('Pemetaan router hanya untuk layanan Internet / PPPoE.');
      return;
    }
    mapPkg = p;
    const existing = firstMappingFor(p.id);
    mapRouter = existing?.router_id || '';
    mapProfile = existing?.router_profile_name || '';
    mapPool = existing?.address_pool || '';
    mapIsolation = existing?.isolation_pool || '';
    profileSug = [];
    poolSug = [];
    if (mapRouter) void loadRouterMeta(mapRouter);
    mapModalOpen = true;
  }

  async function saveMapping() {
    if (saving || !mapPkg || !mapRouter || !mapProfile.trim()) return;
    saving = true;
    try {
      const refErr = getPackageRouterMappingReferenceError({
        routerId: mapRouter,
        profileName: mapProfile,
        profileSuggestions: profileSug,
        poolName: mapPool,
        poolSuggestions: poolSug,
      });
      if (refErr) throw new Error(refErr);
      await api.ispPackages.routerMappings.upsert({
        router_id: mapRouter,
        package_id: mapPkg.id,
        router_profile_name: mapProfile.trim(),
        address_pool: mapPool.trim() || null,
        isolation_pool: mapIsolation.trim() || null,
      });
      toast.success('Pemetaan tersimpan');
      mapModalOpen = false;
      await loadMappings();
    } catch (e: any) {
      toast.error(
        getPackageRouterMappingErrorFallback(extractApiErrorMessage(e) || String(e?.message || e)),
      );
    } finally {
      saving = false;
    }
  }

  function confirmDelete(p: IspPackage) {
    if (!$can('manage', 'isp_packages')) return;
    deleteTarget = p;
    deleteOpen = true;
  }

  async function handleDelete() {
    if (!deleteTarget) return;
    try {
      await api.ispPackages.packages.delete(deleteTarget.id);
      toast.success('Paket dihapus');
      await Promise.all([loadPackages(), loadMappings()]);
    } catch (e: any) {
      toast.error(friendlyDeleteError(extractApiErrorMessage(e) || String(e?.message || e)));
    } finally {
      deleteOpen = false;
      deleteTarget = null;
    }
  }

  function rowPrimary(p: IspPackage): RowAction {
    return { label: 'Sunting', icon: 'cog', onclick: () => openEdit(p) };
  }

  function rowRest(p: IspPackage): RowAction[] {
    const acts: RowAction[] = [];
    if (mappingAllowed(p.service_type, p.provisioning_type)) {
      acts.push({ label: 'Petakan router', icon: 'router', onclick: () => openMapping(p) });
    }
    acts.push({ label: 'Hapus', icon: 'close', danger: true, onclick: () => confirmDelete(p) });
    return acts;
  }

  const typeCards = [
    {
      value: 'internet_pppoe' as ServiceType,
      icon: 'router' as const,
      title: 'Internet / PPPoE',
      sub: 'Internet tetap dengan provisioning PPPoE atau DHCP Static, opsional pemetaan profil router.',
    },
    {
      value: 'hotspot' as ServiceType,
      icon: 'wifi' as const,
      title: 'Hotspot',
      sub: 'Layanan voucher / captive portal untuk zona akses bersama.',
    },
    {
      value: 'vpn' as ServiceType,
      icon: 'shield' as const,
      title: 'VPN',
      sub: 'Tunnel terenkripsi untuk kantor cabang atau akses privat.',
    },
  ];

  onMount(() => {
    if (
      !canAccessServiceCatalog($user, $can('read', 'isp_packages'), $can('manage', 'isp_packages'))
    ) {
      goto('/unauthorized');
      return;
    }
    void load();
  });
</script>

<AppShell title="Layanan">
  <PageHeader
    title="Layanan"
    eyebrow="Katalog"
    desc="Paket internet, hotspot, dan VPN yang bisa dilanggankan pelanggan."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void load()}>Muat ulang</Button>
      {#if canManage}
        <Button variant="primary" icon="plus" onclick={openCreate}>Tambah paket</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <Card>
    <div class="grid grid-cols-2 gap-6 sm:grid-cols-4">
      <StatTile label="Total paket" value={String(stats.total)} hint="di katalog tenant ini" />
      <StatTile
        label="Aktif"
        value={String(stats.aktif)}
        hint="bisa dilanggankan baru"
        tone={stats.aktif > 0 ? 'positive' : 'warning'}
      />
      <StatTile label="Internet/PPPoE" value={String(stats.pppoe)} hint="butuh pemetaan router agar profil bandwidth benar" />
      <StatTile
        label="Terpetakan"
        value={String(stats.terpetakan)}
        hint="paket dengan minimal satu mapping router"
        tone={stats.terpetakan < stats.pppoe ? 'warning' : 'positive'}
      />
    </div>
  </Card>

  {#if attention.length}
    <div class="mt-4">
      <AttentionPanel items={attention} title="Perlu perhatian" />
    </div>
  {/if}

  <div class="mt-4">
    <Card>
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <div class="relative min-w-[220px] flex-1">
          <Icon
            name="search"
            size={15}
            class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400"
          />
          <input
            bind:value={search}
            placeholder="Cari nama atau deskripsi paket"
            aria-label="Cari paket"
            class="focus-ring h-9 w-full rounded-lg border-0 bg-white pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
            oninput={() => {
              pageNum = 1;
              void loadPackages();
            }}
          />
        </div>
        <select
          bind:value={sortBy}
          aria-label="Urutkan"
          class="focus-ring h-9 rounded-lg border-0 bg-white px-2 text-base text-ink-900 ring-1 ring-inset ring-ink-200"
          onchange={() => {
            pageNum = 1;
            void loadPackages();
          }}
        >
          <option value="name">Nama</option>
          <option value="type">Tipe</option>
          <option value="price">Harga</option>
          <option value="status">Status</option>
          <option value="mappings">Jumlah mapping</option>
        </select>
        <button
          type="button"
          class="focus-ring h-9 rounded-lg px-2 text-sm font-medium text-ink-600 ring-1 ring-inset ring-ink-200 hover:bg-ink-50"
          onclick={() => {
            sortDir = sortDir === 'asc' ? 'desc' : 'asc';
            void loadPackages();
          }}
        >
          {sortDir === 'asc' ? 'Naik ↑' : 'Turun ↓'}
        </button>
      </div>

      <DataTable
        {columns}
        {rows}
        {loading}
        emptyTitle="Belum ada paket"
        emptyHint={search
          ? 'Coba kata kunci lain atau hapus pencarian.'
          : 'Tambahkan paket pertama agar pelanggan bisa dilanggankan.'}
        footNote={`${rows.length} dari ${total} paket · halaman ${pageNum}/${totalPages}`}
      >
        {#snippet cell(p, c)}
          {#if c.key === 'name'}
            <div class="min-w-0 max-w-[420px]">
              <div class="truncate font-medium text-ink-900">{p.name}</div>
              {#if p.description}
                <div class="truncate text-sm text-ink-500">{p.description}</div>
              {/if}
              {#if p.features?.length}
                <div class="mt-1 flex flex-wrap gap-1">
                  {#each p.features.slice(0, 3) as f}
                    <span class="rounded-full bg-ink-100 px-2 py-0.5 text-xs text-ink-600">{f}</span>
                  {/each}
                  {#if p.features.length > 3}
                    <span class="rounded-full bg-ink-100 px-2 py-0.5 text-xs text-ink-600">
                      +{p.features.length - 3}
                    </span>
                  {/if}
                </div>
              {/if}
            </div>
          {:else if c.key === 'type'}
            <div class="flex flex-col gap-0.5">
              <Badge
                label={serviceTypeLabel(p.service_type, p.provisioning_type)}
                tone={serviceTypeTone(p.service_type)}
              />
            </div>
          {:else if c.key === 'price'}
            <div class="num text-sm text-ink-900">
              <div>{displayPrice(p.price_monthly)}<span class="text-ink-400">/bln</span></div>
              {#if Number(p.price_yearly) > 0}
                <div class="text-ink-500">{displayPrice(p.price_yearly)}<span class="text-ink-400">/thn</span></div>
              {/if}
            </div>
          {:else if c.key === 'status'}
            <Badge label={p.is_active ? 'Aktif' : 'Nonaktif'} tone={p.is_active ? 'positive' : 'neutral'} />
          {:else if c.key === 'mappings'}
            {#if mappingAllowed(p.service_type, p.provisioning_type)}
              {@const n = mappingCountFor(p.id)}
              <span class="num text-sm {n === 0 ? 'font-medium text-amber-700' : 'text-ink-700'}">
                {n}
              </span>
            {:else}
              <span class="text-sm text-ink-400">—</span>
            {/if}
          {:else if c.key === 'actions'}
            {#if canManage}
              <RowActions primary={rowPrimary(p)} rest={rowRest(p)} />
            {/if}
          {/if}
        {/snippet}
      </DataTable>

      {#if totalPages > 1}
        <div class="mt-3 flex items-center justify-end gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={pageNum <= 1}
            onclick={() => {
              pageNum -= 1;
              void loadPackages();
            }}>Sebelumnya</Button
          >
          <span class="num text-sm text-ink-500">{pageNum} / {totalPages}</span>
          <Button
            variant="ghost"
            size="sm"
            disabled={pageNum >= totalPages}
            onclick={() => {
              pageNum += 1;
              void loadPackages();
            }}>Berikutnya</Button
          >
        </div>
      {/if}
    </Card>
  </div>
</AppShell>

<!-- Pilih tipe layanan dulu (parity dengan legacy), baru form. -->
<Modal bind:show={typePickerOpen} title="Pilih tipe layanan" width="640px">
  <div class="grid gap-3 py-1">
    {#each typeCards as card (card.value)}
      <button
        type="button"
        class="flex items-start gap-3 rounded-xl border border-ink-200 bg-white p-4 text-left transition hover:border-ink-400 focus-ring"
        onclick={() => startCreateWithType(card.value)}
      >
        <span class="mt-0.5 grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-ink-100 text-ink-700">
          <Icon name={card.icon} size={18} />
        </span>
        <span class="min-w-0">
          <span class="block font-medium text-ink-900">{card.title}</span>
          <span class="block text-sm text-ink-500">{card.sub}</span>
        </span>
      </button>
    {/each}
  </div>
</Modal>

<!-- Form create/edit paket. -->
<Modal
  bind:show={formOpen}
  title={editTarget ? `Sunting paket — ${editTarget.name}` : 'Paket baru'}
  width="720px"
>
  <div class="space-y-1 py-1">
    <div class="grid gap-x-6 sm:grid-cols-2">
      <Field
        stacked
        id="s-type"
        label="Tipe layanan"
        value={fType}
        type="select"
        options={[
          { value: 'internet_pppoe', label: 'Internet / PPPoE' },
          { value: 'hotspot', label: 'Hotspot' },
          { value: 'vpn', label: 'VPN' },
        ]}
        onchange={(v) => {
          fType = v as ServiceType;
          if (fType !== 'internet_pppoe') {
            fProv = 'pppoe';
            mapInline = false;
          }
        }}
      />
      {#if fType === 'internet_pppoe'}
        <Field
          stacked
          id="s-prov"
          label="Provisioning"
          value={fProv}
          type="select"
          options={[
            { value: 'pppoe', label: 'PPPoE' },
            { value: 'dhcp_static', label: 'DHCP Static' },
          ]}
          help="DHCP Static tidak memakai pemetaan profil router."
          onchange={(v) => {
            fProv = v as 'pppoe' | 'dhcp_static';
            if (fProv !== 'pppoe') mapInline = false;
          }}
        />
      {/if}
    </div>

    <Field
      stacked
      id="s-name"
      label="Nama paket"
      value={fName}
      placeholder="mis. Fiber 20 Mbps"
      error={formErrors.find((e) => /Nama/i.test(e)) ?? null}
      onchange={(v) => (fName = v)}
    />
    <Field
      stacked
      id="s-desc"
      label="Deskripsi"
      value={fDesc}
      type="textarea"
      rows={2}
      placeholder="Catatan singkat untuk staf dan portal pelanggan."
      onchange={(v) => (fDesc = v)}
    />

    <div class="grid gap-x-6 sm:grid-cols-2">
      <Field
        stacked
        id="s-monthly"
        label="Harga bulanan ({baseCurrency})"
        value={String(fMonthly)}
        type="number"
        min={0}
        error={formErrors.find((e) => /bulanan/i.test(e)) ?? null}
        onchange={(v) => (fMonthly = Number(v) || 0)}
      />
      <div>
        <Field
          stacked
          id="s-yearly-on"
          label="Tawarkan harga tahunan"
          value={String(fYearlyOn)}
          type="toggle"
          help="Berguna untuk promo bayar-12-gratis-2."
          onchange={(v) => (fYearlyOn = v === 'true')}
        />
        {#if fYearlyOn}
          <Field
            stacked
            id="s-yearly"
            label="Harga tahunan ({baseCurrency})"
            value={String(fYearly)}
            type="number"
            min={0}
            error={formErrors.find((e) => /tahunan/i.test(e)) ?? null}
            onchange={(v) => (fYearly = Number(v) || 0)}
          />
        {/if}
      </div>
    </div>

    <Field
      stacked
      id="s-active"
      label="Aktif"
      value={String(fActive)}
      type="toggle"
      help="Paket nonaktif tidak bisa dilanggungkan baru."
      onchange={(v) => (fActive = v === 'true')}
    />

    <!-- Fitur -->
    <div class="py-3">
      <span class="mb-1 block text-[13px] font-medium text-ink-700">Fitur</span>
      <div class="flex gap-2">
        <input
          bind:value={fFeatureInput}
          placeholder="mis. Bandwidth dedicated"
          class="focus-ring h-9 min-w-0 flex-1 rounded-lg border-0 bg-white px-3 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400"
          onkeydown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              addFeature();
            }
          }}
        />
        <Button variant="ghost" onclick={addFeature}>Tambah</Button>
      </div>
      {#if fFeatures.length}
        <div class="mt-2 flex flex-wrap gap-1.5">
          {#each fFeatures as f, i (f + i)}
            <span
              class="inline-flex items-center gap-1 rounded-full bg-ink-100 py-1 pr-1.5 pl-2.5 text-sm text-ink-700"
            >
              {f}
              <button
                type="button"
                class="grid h-5 w-5 place-items-center rounded-full text-ink-400 hover:bg-ink-200 hover:text-ink-700"
                aria-label="Hapus fitur {f}"
                onclick={() => removeFeature(i)}
              >
                <Icon name="close" size={12} />
              </button>
            </span>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Mapping router inline (hanya Internet/PPPoE) -->
    {#if mappingAllowed(fType, fProv)}
      <div class="rounded-xl border border-ink-200 bg-ink-50/60 p-3">
        <Field
          stacked
          id="s-map-on"
          label="Petakan ke router sekarang"
          value={String(mapInline)}
          type="toggle"
          help="Profil PPP & pool IP menentukan bandwidth pelanggan baru paket ini."
          onchange={(v) => (mapInline = v === 'true')}
        />
        {#if mapInline}
          <div class="mt-1 grid gap-x-6 sm:grid-cols-2">
            <Field
              stacked
              id="s-map-router"
              label="Router"
              value={mapRouter}
              type="select"
              options={routerOptions}
              onchange={(v) => {
                mapRouter = v;
                mapProfile = '';
                mapPool = '';
                void loadRouterMeta(v);
              }}
            />
            <Field
              stacked
              id="s-map-profile"
              label="Profil PPP"
              value={mapProfile}
              type="select"
              options={profileOptions}
              disabled={!mapRouter || loadingMeta}
              help={loadingMeta ? 'Mengambil profil dari router…' : 'Hanya profil yang masih ada di router.'}
              onchange={(v) => (mapProfile = v)}
            />
            <Field
              stacked
              id="s-map-pool"
              label="Address pool (opsional)"
              value={mapPool}
              type="select"
              options={poolOptions}
              disabled={!mapRouter || loadingMeta}
              onchange={(v) => (mapPool = v)}
            />
            <Field
              stacked
              id="s-map-iso"
              label="Isolation pool (opsional)"
              value={mapIsolation}
              placeholder="mis. iso-20m"
              onchange={(v) => (mapIsolation = v)}
            />
          </div>
        {/if}
      </div>
    {/if}

    {#if formErrors.length}
      <div class="mt-3 rounded-lg bg-red-50 p-3 text-sm text-red-700">
        {#each formErrors as e (e)}
          <div>{e}</div>
        {/each}
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={() => (formOpen = false)}>Batal</Button>
    <Button variant="primary" disabled={saving} onclick={() => void savePackage()}>
      {saving ? 'Menyimpan…' : editTarget ? 'Simpan perubahan' : 'Buat paket'}
    </Button>
  {/snippet}
</Modal>

<!-- Mapping berdiri sendiri -->
<Modal bind:show={mapModalOpen} title={mapPkg ? `Pemetaan router — ${mapPkg.name}` : 'Pemetaan router'} width="640px">
  <div class="grid gap-x-6 py-1 sm:grid-cols-2">
    <Field
      stacked
      id="m-router"
      label="Router"
      value={mapRouter}
      type="select"
      options={routerOptions}
      onchange={(v) => {
        mapRouter = v;
        mapProfile = '';
        mapPool = '';
        void loadRouterMeta(v);
      }}
    />
    <Field
      stacked
      id="m-profile"
      label="Profil PPP"
      value={mapProfile}
      type="select"
      options={profileOptions}
      disabled={!mapRouter || loadingMeta}
      help={loadingMeta ? 'Mengambil profil dari router…' : undefined}
      onchange={(v) => (mapProfile = v)}
    />
    <Field
      stacked
      id="m-pool"
      label="Address pool (opsional)"
      value={mapPool}
      type="select"
      options={poolOptions}
      disabled={!mapRouter || loadingMeta}
      onchange={(v) => (mapPool = v)}
    />
    <Field
      stacked
      id="m-iso"
      label="Isolation pool (opsional)"
      value={mapIsolation}
      placeholder="mis. iso-20m"
      onchange={(v) => (mapIsolation = v)}
    />
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={() => (mapModalOpen = false)}>Batal</Button>
    <Button
      variant="primary"
      disabled={saving || !mapRouter || !mapProfile.trim()}
      onclick={() => void saveMapping()}
    >
      {saving ? 'Menyimpan…' : 'Simpan pemetaan'}
    </Button>
  {/snippet}
</Modal>

<!-- Konfirmasi hapus -->
<Modal bind:show={deleteOpen} title="Hapus paket" width="480px">
  <p class="text-sm text-ink-700">
    Hapus paket <span class="font-medium text-ink-900">{deleteTarget?.name}</span>? Server akan
    menolak jika paket masih dipakai langganan, akun PPPoE, layanan DHCP static, atau penawaran zona.
  </p>

  {#snippet footer()}
    <Button variant="ghost" onclick={() => (deleteOpen = false)}>Batal</Button>
    <Button variant="primary" onclick={() => void handleDelete()}>Hapus paket</Button>
  {/snippet}
</Modal>
