<script lang="ts">
  /*
    Langganan Platform v2.

    Versi lama: `(app)/admin/subscription/+page.svelte` (1.092 baris).

    Temuan yang dikunci gelombang ini:

    1. DAFTAR FITUR HARDCODE. FE lama merangkai fitur dari switch slug
       ('free' | 'pro' | 'enterprise') — copy marketing, bukan entitlement
       nyata. DB punya 17 fitur dengan nilai per plan (pro: max_members 10,
       custom_domain true). v2 memakai `features` baru di payload
       subscription/details.
    2. Hapus paket (superadmin) = 500 mentah saat FK menabrak -> guard 409
       dengan daftar tenant (backend).
    3. Plan yatim membuat SELURUH halaman 500 (fetch_one) -> fallback Free.
    4. Invoice untuk plan gratis Rp0 menggantung pending selamanya -> ditolak
       400 (backend).
    5. assign_plan .unwrap() = potensi panic -> 404 jujur (backend).
  */
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { user, tenant, can } from '$lib/stores/auth';
  import { api, type Invoice, type TenantSubscriptionDetails } from '$lib/api/client';
  import { formatMoney } from '$lib/utils/money';
  import { appSettings } from '$lib/stores/settings';
  import { formatDate } from '$lib/utils/date';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { getAdminBillingNavigation } from '$lib/utils/adminBillingNavigation';
  import {
    featureIsOn,
    featureValueLabel,
    formatBytesIEC,
    friendlyPlanError,
    groupFeaturesByCategory,
    usagePercent,
  } from '$lib/utils/subscriptionInsights';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import type { Column } from '$lib/components/ds/table-types';
  import type { StatusTone } from '$lib/components/ds/tokens';

  type PlanRow = {
    id: string;
    name: string;
    slug: string;
    description: string | null;
    price_monthly: number;
    price_yearly: number;
    is_active: boolean;
  };

  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let subscription = $state<TenantSubscriptionDetails | null>(null);
  let availablePlans = $state<PlanRow[]>([]);
  let invoices = $state<Invoice[]>([]);
  let upgrading = $state(false);
  let activeTab = $state<'overview' | 'plans' | 'history'>('overview');

  let baseCurrencyCode = $state('IDR');
  let baseLocale = $state('en-US');
  let fxRate = $state<number | null>(null);
  let fxSource = $state<string | null>(null);
  let fxLoading = $state(false);
  let fxError = $state<string | null>(null);

  const billingNav = $derived.by(() =>
    getAdminBillingNavigation({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );

  const tenantCurrencyCode = $derived.by(() =>
    String($appSettings?.currency_code || baseCurrencyCode).toUpperCase(),
  );

  const currentPlanInfo = $derived(
    availablePlans.find((p) => p.slug === subscription?.plan_slug) ?? null,
  );

  const featureGroups = $derived(groupFeaturesByCategory(subscription?.features ?? []));

  onMount(() => {
    void (async () => {
      if (!$can('read', 'billing') && !$can('manage', 'billing')) {
        goto('/unauthorized');
        return;
      }
      try {
        const [subRes, plansRes, invoicesRes, publicSettings] = await Promise.all([
          api.plans.getSubscriptionDetails(),
          api.plans.list(),
          api.payment.listInvoices(),
          api.settings.getPublicSettings(),
        ]);
        subscription = subRes;
        availablePlans = ((plansRes as PlanRow[]) || []).filter((p) => p.is_active);
        invoices = invoicesRes || [];
        if (publicSettings?.base_currency_code || publicSettings?.currency_code) {
          baseCurrencyCode = String(
            publicSettings.base_currency_code || publicSettings.currency_code,
          ).toUpperCase();
        }
        if (publicSettings?.default_locale) baseLocale = String(publicSettings.default_locale);
      } catch (e) {
        loadError = friendlyPlanError(extractApiErrorMessage(e));
      } finally {
        loading = false;
      }
    })();
  });

  $effect(() => {
    fxError = null;
    fxRate = null;
    fxSource = null;
    if (!baseCurrencyCode || !tenantCurrencyCode) return;
    if (baseCurrencyCode === tenantCurrencyCode) return;
    fxLoading = true;
    api.payment
      .getFxRate(baseCurrencyCode, tenantCurrencyCode)
      .then((res) => {
        fxRate = Number(res.rate) || null;
        fxSource = res.source || null;
      })
      .catch((e: any) => {
        fxError = e?.message || String(e);
        fxRate = null;
        fxSource = null;
      })
      .finally(() => {
        fxLoading = false;
      });
  });

  function storageToneClass(pct: number): string {
    if (pct >= 100) return 'bg-red-500';
    if (pct > 80) return 'bg-amber-500';
    return 'bg-emerald-500';
  }

  function roundForCurrency(amount: number, currencyCode: string): number {
    const c = currencyCode.toUpperCase();
    const digits = c === 'IDR' || c === 'JPY' || c === 'KRW' ? 0 : 2;
    const factor = Math.pow(10, digits);
    return Math.round(amount * factor) / factor;
  }

  function formatBasePrice(amount: number): string {
    return formatMoney(amount, { currency: baseCurrencyCode, locale: baseLocale });
  }

  function formatPlanPrice(amount: number): string {
    if (tenantCurrencyCode === baseCurrencyCode || !fxRate) {
      return formatMoney(amount, { currency: baseCurrencyCode, locale: baseLocale });
    }
    const converted = roundForCurrency(amount * fxRate, tenantCurrencyCode);
    return formatMoney(converted, { currency: tenantCurrencyCode, locale: baseLocale });
  }

  async function handleUpgrade(plan: PlanRow) {
    if (upgrading) return;
    upgrading = true;
    try {
      const invoice = await api.payment.createInvoiceForPlan(plan.id, 'monthly');
      goto(`/pay/${invoice.id}`);
    } catch (e) {
      loadError = friendlyPlanError(extractApiErrorMessage(e));
      upgrading = false;
    }
  }

  const statusTone: StatusTone = $derived(
    subscription?.status === 'active' ? 'positive' : subscription?.status === 'trial' ? 'info' : 'warning',
  );

  const storagePct = $derived(
    usagePercent(subscription?.storage_usage ?? 0, subscription?.storage_limit),
  );
  const memberPct = $derived(
    usagePercent(subscription?.member_usage ?? 0, subscription?.member_limit),
  );

  const columns: Column[] = [
    { key: 'invoice_number', label: 'Invoice' },
    { key: 'description', label: 'Uraian' },
    { key: 'amount', label: 'Nominal', align: 'right' },
    { key: 'status', label: 'Status', width: '120px' },
    { key: 'due_date', label: 'Jatuh tempo', width: '140px' },
    { key: 'actions', label: '', width: '110px', align: 'right' },
  ];

  function invoiceTone(status: string): StatusTone {
    if (status === 'paid') return 'positive';
    if (status === 'pending' || status === 'unpaid') return 'warning';
    if (status === 'overdue') return 'negative';
    return 'neutral';
  }

  const tabs = [
    { id: 'overview', label: 'Ringkasan' },
    { id: 'plans', label: 'Paket' },
    { id: 'history', label: 'Riwayat Pembayaran' },
  ] as const;
</script>

<AppShell title="Langganan Platform">
  <PageHeader title="Langganan Platform" desc="Paket platform Tri Digital, pemakaian, dan riwayat penagihan tenant ini.">
    {#snippet actions()}
      <Button variant="ghost" icon="receipt" onclick={() => goto(billingNav.billingPath)}>
        Tagihan Pelanggan
      </Button>
      <Button variant="ghost" onclick={() => goto(billingNav.billingPlanSettingsPath)}>
        Setelan Paket
      </Button>
    {/snippet}
  </PageHeader>

  {#if loadError}
    <div class="mt-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700">
      {loadError}
      <button type="button" class="ml-2 underline" onclick={() => (loadError = null)}>Tutup</button>
    </div>
  {/if}

  <div class="mt-4 flex gap-1 border-b border-ink-200">
    {#each tabs as t (t.id)}
      <button
        type="button"
        class="focus-ring -mb-px rounded-t-lg border-b-2 px-4 py-2 text-sm font-medium {activeTab === t.id
          ? 'border-ink-900 text-ink-900'
          : 'border-transparent text-ink-500 hover:text-ink-900'}"
        aria-current={activeTab === t.id ? 'page' : undefined}
        onclick={() => (activeTab = t.id)}
      >
        {t.label}
      </button>
    {/each}
  </div>

  {#if loading}
    <div class="mt-10 flex items-center justify-center gap-3 text-ink-500">
      <span class="inline-block size-4 animate-spin rounded-full border-2 border-ink-300 border-t-ink-900"></span>
      Memuat langganan…
    </div>
  {:else if subscription}
    {#if activeTab === 'overview'}
      <section class="mt-5 rounded-xl border border-ink-200 bg-white p-5">
        <div class="flex flex-wrap items-start justify-between gap-4">
          <div>
            <div class="flex items-center gap-2">
              <h2 class="text-lg font-semibold text-ink-900">{subscription.plan_name}</h2>
              <Badge tone={statusTone} label={subscription.status === 'active' ? 'Aktif' : subscription.status} />
            </div>
            <p class="mt-1 text-sm text-ink-500">
              {currentPlanInfo?.description || 'Paket platform aktif untuk tenant ini.'}
            </p>
          </div>
          <div class="text-right">
            {#if currentPlanInfo && currentPlanInfo.price_monthly > 0}
              <div class="text-2xl font-semibold text-ink-900">
                {formatPlanPrice(currentPlanInfo.price_monthly)}
                <span class="text-sm font-normal text-ink-500">/bulan</span>
              </div>
              {#if tenantCurrencyCode !== baseCurrencyCode}
                <div class="mt-1 text-xs text-ink-500">
                  Dasar {formatBasePrice(currentPlanInfo.price_monthly)}
                  {#if fxLoading}
                    <span class="ml-1 rounded bg-ink-100 px-1.5 py-0.5">kurs memuat…</span>
                  {:else if fxSource}
                    <span class="ml-1 rounded bg-ink-100 px-1.5 py-0.5">kurs {fxSource}</span>
                  {:else if fxError}
                    <span class="ml-1 rounded bg-amber-100 px-1.5 py-0.5 text-amber-800">kurs tidak tersedia</span>
                  {/if}
                </div>
              {/if}
            {:else}
              <div class="text-2xl font-semibold text-ink-900">Gratis</div>
            {/if}
            <div class="mt-1 text-sm text-ink-500">
              {#if subscription.current_period_end}
                Berlaku s.d. {formatDate(subscription.current_period_end, { timeZone: $appSettings.app_timezone })}
              {:else}
                Seumur hidup
              {/if}
            </div>
          </div>
        </div>

        <div class="mt-5 grid grid-cols-1 gap-3 md:grid-cols-2">
          <div class="rounded-lg border border-ink-200 p-4">
            <div class="flex items-baseline justify-between">
              <span class="text-sm font-medium text-ink-900">Penyimpanan</span>
              <span class="text-sm text-ink-500">
                {formatBytesIEC(subscription.storage_usage)} /
                {subscription.storage_limit ? formatBytesIEC(subscription.storage_limit) : 'Tanpa batas'}
              </span>
            </div>
            <div class="mt-2 h-2 overflow-hidden rounded-full bg-ink-100">
              <div
                class="h-full rounded-full {storageToneClass(storagePct)}"
                style="width: {Math.max(2, storagePct)}%"
              ></div>
            </div>
          </div>
          <div class="rounded-lg border border-ink-200 p-4">
            <div class="flex items-baseline justify-between">
              <span class="text-sm font-medium text-ink-900">Anggota tim</span>
              <span class="text-sm text-ink-500">
                {subscription.member_usage} /
                {subscription.member_limit ?? 'Tanpa batas'}
              </span>
            </div>
            <div class="mt-2 h-2 overflow-hidden rounded-full bg-ink-100">
              <div
                class="h-full rounded-full {storageToneClass(memberPct)}"
                style="width: {Math.max(2, memberPct)}%"
              ></div>
            </div>
          </div>
        </div>

        <h3 class="mt-6 text-sm font-semibold uppercase tracking-wide text-ink-500">
          Fitur paket (entitlement nyata)
        </h3>
        {#if featureGroups.length === 0}
          <p class="mt-2 text-sm text-ink-500">Belum ada fitur terdefinisi untuk paket ini.</p>
        {:else}
          <div class="mt-3 grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
            {#each featureGroups as g (g.category)}
              <div class="rounded-lg border border-ink-200 p-4">
                <div class="text-sm font-semibold text-ink-900">{g.category}</div>
                <ul class="mt-2 space-y-1.5">
                  {#each g.items as f (f.code)}
                    <li class="flex items-center justify-between gap-2 text-sm">
                      <span class="text-ink-700">{f.name}</span>
                      <span class="shrink-0 font-medium {featureIsOn(f) ? 'text-ink-900' : 'text-ink-400'}">
                        {featureValueLabel(f)}
                      </span>
                    </li>
                  {/each}
                </ul>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {:else if activeTab === 'plans'}
      <div class="mt-5 grid grid-cols-1 gap-4 md:grid-cols-3">
        {#each availablePlans as plan (plan.id)}
          {@const isCurrent = plan.slug === subscription.plan_slug}
          <div
            class="flex flex-col rounded-xl border bg-white p-5 {isCurrent
              ? 'border-ink-900 ring-1 ring-ink-900'
              : 'border-ink-200'}"
          >
            <div class="flex items-baseline justify-between">
              <h3 class="text-base font-semibold text-ink-900">{plan.name}</h3>
              {#if isCurrent}
                <Badge tone="neutral" label="Paket saat ini" />
              {/if}
            </div>
            <div class="mt-2">
              {#if plan.price_monthly > 0}
                <span class="text-2xl font-semibold text-ink-900">{formatPlanPrice(plan.price_monthly)}</span>
                <span class="text-sm text-ink-500">/bulan</span>
                {#if plan.price_yearly > 0}
                  <div class="mt-1 text-xs text-ink-500">
                    Tahunan {formatPlanPrice(plan.price_yearly)}
                  </div>
                {/if}
              {:else}
                <span class="text-2xl font-semibold text-ink-900">Gratis</span>
              {/if}
            </div>
            {#if plan.description}
              <p class="mt-2 text-sm text-ink-500">{plan.description}</p>
            {/if}
            <div class="mt-auto pt-4">
              {#if isCurrent}
                <Button variant="ghost" class="w-full" disabled>Paket aktif</Button>
              {:else}
                <Button
                  variant={plan.price_monthly > 0 ? 'primary' : 'ghost'}
                  class="w-full"
                  disabled={upgrading || plan.price_monthly <= 0}
                  onclick={() => void handleUpgrade(plan)}
                >
                  {upgrading ? 'Memproses…' : subscription.plan_slug === 'free' ? 'Langganan' : 'Upgrade'}
                </Button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="mt-5">
        <DataTable
          {columns}
          rows={invoices}
          {loading}
          footNote={`${invoices.length} invoice langganan platform.`}
        >
          {#snippet cell(row: Invoice, col: Column)}
            {#if col.key === 'amount'}
              <span class="text-sm tabular-nums">{formatMoney(row.amount, { currency: row.currency_code })}</span>
            {:else if col.key === 'status'}
              <Badge tone={invoiceTone(row.status)} label={row.status} />
            {:else if col.key === 'due_date'}
              <span class="text-sm">{row.due_date ? formatDate(row.due_date, { timeZone: $appSettings.app_timezone }) : '—'}</span>
            {:else if col.key === 'actions'}
              <Button
                variant={row.status === 'pending' ? 'primary' : 'ghost'}
                size="sm"
                onclick={() => goto(`/pay/${row.id}`)}
              >
                {row.status === 'pending' ? 'Bayar' : 'Detail'}
              </Button>
            {:else}
              <span class="text-sm">{String((row as unknown as Record<string, unknown>)[col.key] ?? '—')}</span>
            {/if}
          {/snippet}
        </DataTable>
      </div>
    {/if}
  {:else}
    <div class="mt-10 rounded-lg border border-ink-200 bg-white p-6 text-center text-sm text-ink-500">
      Data langganan tidak tersedia.
    </div>
  {/if}
</AppShell>
