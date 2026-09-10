<script lang="ts">
  /*
    Analitik penagihan v2.

    Versi lama: `(app)/admin/billing/+page.svelte` — 396 baris. Layar itu
    memajang "MRR Rp 0" dan "0 langganan aktif" persis di sebelah piutang
    Rp 57,6 juta, jadi setiap angkanya saling membantah. Lima masalah
    dibuktikan di data produksi tenant ISP Management (2026-09-04), empat di
    antaranya bukan soal tampilan:

    1. UANG MASUK DICAMPUR UANG KELUAR.
       Tabel `invoices` menampung dua jenis tagihan: milik pelanggan ISP
       (`external_id LIKE 'pkgsub:%'`) dan tagihan tenant ke platform
       (`plan:%`). `revenue_trend` menjumlahkan keduanya:

         2026-05  layar lama: Rp 1.280.000
                  dari pelanggan: Rp 0
                  tagihan platform: Rp 1.280.000

       Jadi batang Mei seluruhnya adalah uang yang tenant BAYARKAN, bukan
       terima. Sisa kode sudah lama memisahkan keduanya lewat prefix
       (`list_invoices`, `list_customer_package_invoices`); hanya analytics
       yang tidak. Diperbaiki di `payment_service/analytics.rs`.

    2. LAPORAN UMUR PIUTANG TIDAK UTUH.
       Kueri lama `status IN ('pending','overdue')` melewatkan
       `verification_pending` — invoice yang buktinya sudah diunggah dan
       menunggu verifikasi, tetap piutang (Rp 165.000 di tenant ini). Filter
       `due_date < NOW()` juga membuang seluruh invoice yang belum jatuh
       tempo; uang itu tidak punya tempat di layar sama sekali. Sekarang ada
       bucket `not_due` dan totalnya dikirim server sebagai `aging_total`.

    3. GRAFIK "6 BULAN" MENAMPILKAN 4 BATANG.
       `GROUP BY DATE_TRUNC` hanya menghasilkan baris untuk bulan yang punya
       invoice lunas, jadi Juli–September 2026 hilang — padahal tiga bulan
       tanpa pemasukan justru informasi terpentingnya.

    4. PERSENTASE TANPA PENYEBUT.
       `collection_rate` 0% di tenant ini dihitung dari 2 invoice, dan
       `avg_days_to_pay` 0 hari berasal dari himpunan kosong. Tanpa basis
       sampel, keduanya tak bisa dibedakan dari "semua pelanggan gagal bayar".

    5. MRR NOL TANPA PENJELASAN.
       Tidak ada satu pun `customer_subscriptions` berstatus `active`: 542
       `suspended`, 5 `pending_installation`, 2 `cancelled`. Itu akibat
       `mixradius_import_mapper` memetakan UNPAID+kedaluwarsa ke `suspended`.
       Halaman kini menyebut rinciannya, jadi MRR nol terbaca sebagai kondisi
       bisnis, bukan bug.

    Turunan tampilan pindah ke `$lib/utils/billingAnalytics` (27 tes unit),
    termasuk rekonsiliasi total aging — dulu klien menjumlahkan empat bucket
    sendiri, sehingga bucket kelima di server akan hilang tanpa gejala.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import type { BillingAnalytics } from '$lib/api/types';
  import {
    AppShell,
    AttentionPanel,
    Badge,
    Button,
    Card,
    EmptyState,
    Icon,
    PageHeader,
    StatTile,
    formatCompactRupiah,
    formatRupiah,
  } from '$lib/components/ds';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
  import {
    agingReconciliation,
    avgDaysCaption,
    buildAgingRows,
    buildTrendBars,
    collectionCaption,
    hasPlatformDues,
    mrrExplanation,
    subscriptionStatusLabel,
    subscriptionSummary,
    trendIsEmpty,
  } from '$lib/utils/billingAnalytics';
  import { t } from 'svelte-i18n';

  let analytics = $state<BillingAnalytics | null>(null);
  let loading = $state(true);
  let errorMessage = $state<string | null>(null);
  let loadSequence = 0;

  onMount(() => {
    if (!$can('read', 'billing') && !$can('manage', 'billing')) {
      void goto('/unauthorized');
      return;
    }
    void load();
  });

  async function load() {
    const sequence = ++loadSequence;
    loading = true;
    errorMessage = null;
    try {
      const result = await api.payment.getBillingAnalytics();
      if (sequence === loadSequence) analytics = result;
    } catch (err) {
      if (sequence === loadSequence) {
        errorMessage = extractApiErrorMessage(err, 'Gagal memuat analitik penagihan');
        toast.error(errorMessage);
      }
    } finally {
      if (sequence === loadSequence) loading = false;
    }
  }

  const tr = (k: string, v?: Record<string, unknown>) => $t(k, v ? { values: v as Record<string, string | number> } : undefined);
  const trendBars = $derived(analytics ? buildTrendBars(analytics.revenue_trend) : []);
  const trendKosong = $derived(trendIsEmpty(trendBars));
  const agingRows = $derived(analytics ? buildAgingRows(analytics, tr) : []);
  const rekonsiliasi = $derived(analytics ? agingReconciliation(analytics) : null);
  const penjelasanMrr = $derived(analytics ? mrrExplanation(analytics, tr) : null);

  const barTone: Record<string, string> = {
    neutral: 'bg-slate-400',
    info: 'bg-sky-500',
    warning: 'bg-amber-500',
    danger: 'bg-orange-500',
    critical: 'bg-red-500',
  };

  /* Hal yang perlu ditindaklanjuti, bukan sekadar angka. */
  const peringatan = $derived.by((): AttentionItem[] => {
    if (!analytics) return [];
    const items: AttentionItem[] = [];

    if (analytics.aging.over_90 > 0) {
      items.push({
        icon: 'alert',
        title: $t('admin.billing.analytics.v2.at1_title'),
        detail: `${formatRupiah(analytics.aging.over_90)} ${tr('admin.billing.analytics.v2.at1_detail')}`,
        action: $t('admin.billing.analytics.v2.at1_act'),
        href: '/v2/admin/invoices',
        severity: 'high',
      });
    }

    if (penjelasanMrr) {
      items.push({
        icon: 'users',
        title: $t('admin.billing.analytics.v2.at2_title'),
        detail: penjelasanMrr,
        action: $t('admin.billing.analytics.v2.at2_act'),
        href: '/v2/admin/customers',
        severity: 'high',
      });
    }

    if (analytics.collection_sample.invoices_considered === 0) {
      items.push({
        icon: 'activity',
        title: $t('admin.billing.analytics.v2.at3_title'),
        detail: tr('admin.billing.analytics.v2.at3_detail', { d: analytics.collection_sample.window_days }),
        action: $t('admin.billing.analytics.v2.at3_act'),
        href: '/v2/admin/invoices',
        severity: 'medium',
      });
    }

    if (analytics && hasPlatformDues(analytics)) {
      items.push({
        icon: 'card',
        title: $t('admin.billing.analytics.v2.at4_title'),
        detail: `${analytics.platform_dues.outstanding_count} tagihan senilai ${formatRupiah(analytics.platform_dues.outstanding_amount)} — ini biaya tenant, bukan piutang pelanggan`,
        action: $t('admin.billing.analytics.v2.at4_act'),
        href: '/v2/admin/settings',
        severity: 'medium',
      });
    }

    if (rekonsiliasi && !rekonsiliasi.consistent) {
      items.push({
        icon: 'alert',
        title: $t('admin.billing.analytics.v2.at5_title'),
        detail: tr('admin.billing.analytics.v2.at5_detail', { sv: formatRupiah(rekonsiliasi.serverTotal), b: formatRupiah(rekonsiliasi.bucketSum), d: formatRupiah(Math.abs(rekonsiliasi.drift)) }),
        action: $t('admin.billing.analytics.v2.at5_act'),
        severity: 'high',
      });
    }

    return items;
  });
</script>

<AppShell title={ $t('admin.billing.analytics.v2.title') }>
  <PageHeader
    title={ $t('admin.billing.analytics.v2.title') }
    eyebrow={ $t('admin.eyebrows.finance') }
    desc={ $t('admin.billing.analytics.v2.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void load()} disabled={loading}>
        { $t('admin.network.packages.v2.reload') }
      </Button>
      <Button variant="secondary" icon="receipt" onclick={() => void goto('/v2/admin/invoices')}>
        { $t('admin.billing.analytics.v2.invoices_btn') }
      </Button>
    {/snippet}
  </PageHeader>

  {#if loading}
    <Card>
      <div class="flex items-center gap-3 py-10 text-sm text-ink-500">
        <Icon name="refresh" size={16} class="animate-spin" />
        { $t('admin.billing.analytics.v2.loading') }
      </div>
    </Card>
  {:else if errorMessage}
    <Card>
      <div class="py-10 text-center">
        <div class="mb-2 text-sm font-medium text-red-700">{errorMessage}</div>
        <Button variant="secondary" icon="refresh" onclick={() => void load()}>{ $t('common.retry') }</Button>
      </div>
    </Card>
  {:else if analytics}
    <Card>
      <div class="grid grid-cols-2 gap-6 lg:grid-cols-4">
        <StatTile
          label="MRR"
          value={formatCompactRupiah(analytics.mrr)}
          hint={penjelasanMrr ?? tr('admin.billing.analytics.v2.mrr_from', { n: analytics.active_subscriptions })}
          tone={analytics.mrr > 0 ? 'neutral' : 'warning'}
        />
        <StatTile
          label="ARR"
          value={formatCompactRupiah(analytics.arr)}
          hint={ $t('admin.billing.analytics.v2.arr_hint') }
          tone={analytics.arr > 0 ? 'neutral' : 'warning'}
        />
        <StatTile
          label={ $t('admin.billing.analytics.v2.rev_month') }
          value={formatCompactRupiah(analytics.total_revenue)}
          hint={ $t('admin.billing.analytics.v2.rev_hint') }
          tone={analytics.total_revenue > 0 ? 'positive' : 'warning'}
        />
        <StatTile
          label={ $t('admin.billing.analytics.v2.receivable') }
          value={formatCompactRupiah(analytics.aging_total)}
          hint={`${subscriptionSummary(analytics, tr)} · ${analytics.total_customers} ${$t('admin.billing.analytics.customers')}`}
          tone={analytics.aging_total > 0 ? 'negative' : 'positive'}
        />
      </div>
    </Card>

    {#if peringatan.length}
      <div class="mt-4">
        <AttentionPanel items={peringatan} title={ $t('admin.billing.analytics.v2.attention') } />
      </div>
    {/if}

    <div class="mt-4 grid gap-4 lg:grid-cols-2">
      <Card>
        <div class="mb-4 flex items-baseline justify-between gap-3">
          <h2 class="text-sm font-semibold text-ink-900">{ $t('admin.billing.analytics.v2.aging') }</h2>
          <span class="text-xs text-ink-400">
            { $t('admin.billing.analytics.v2.total_prefix') } {formatRupiah(analytics.aging_total)}
          </span>
        </div>

        {#if analytics.aging_total > 0}
          <div class="space-y-3">
            {#each agingRows as row (row.key)}
              <div class="grid grid-cols-[7.5rem_1fr_auto] items-center gap-3">
                <span class="truncate text-xs text-ink-500">{row.label}</span>
                <div class="h-2 overflow-hidden rounded-full bg-ink-100">
                  <div
                    class="h-full rounded-full {barTone[row.severity]}"
                    style="width: {row.sharePct}%"
                  ></div>
                </div>
                <span class="text-right text-xs font-medium tabular-nums text-ink-700">
                  {formatCompactRupiah(row.amount)}
                </span>
              </div>
            {/each}
          </div>
          <p class="mt-4 text-xs text-ink-400">
            Termasuk tagihan yang menunggu verifikasi bukti bayar dan yang belum jatuh tempo.
          </p>
        {:else}
          <div class="py-8 text-center text-sm text-ink-500">
            Tidak ada piutang pelanggan yang tercatat
          </div>
        {/if}
      </Card>

      <Card>
        <div class="mb-4 flex items-baseline justify-between gap-3">
          <h2 class="text-sm font-semibold text-ink-900">{ $t('admin.billing.analytics.v2.trend6') }</h2>
          <span class="text-xs text-ink-400">{ $t('admin.billing.analytics.v2.trend_sub') }</span>
        </div>

        <div class="flex h-40 gap-2">
          {#each trendBars as bar (bar.month)}
            <div class="flex min-w-0 flex-1 flex-col items-center gap-2">
              <span class="text-[10px] tabular-nums text-ink-400">
                {bar.revenue > 0 ? formatCompactRupiah(bar.revenue) : '-'}
              </span>
              <div class="relative w-full flex-1">
                <div
                  class="absolute inset-x-0 bottom-0 rounded-t {bar.empty
                    ? 'bg-ink-100'
                    : 'bg-indigo-500'}"
                  style="height: {bar.empty ? 2 : Math.max(bar.heightPct, 4)}%"
                ></div>
              </div>
              <span class="text-xs text-ink-500">{bar.label}</span>
            </div>
          {/each}
        </div>

        {#if trendKosong}
          <p class="mt-3 text-xs text-amber-700">
            Tidak ada pemasukan dari pelanggan sepanjang enam bulan ini.
          </p>
        {:else}
          <p class="mt-3 text-xs text-ink-400">
            Bulan tanpa pemasukan tetap ditampilkan sebagai batang kosong.
          </p>
        {/if}
      </Card>
    </div>

    <div class="mt-4 grid gap-4 lg:grid-cols-3">
      <Card>
        <h2 class="mb-3 text-sm font-semibold text-ink-900">{ $t('admin.billing.analytics.v2.coll_rate') }</h2>
        <div class="mb-2 flex items-baseline gap-2">
          <span class="text-2xl font-semibold tabular-nums text-ink-900">
            {analytics.collection_rate}%
          </span>
          <span class="text-xs text-ink-400">{ $t('admin.billing.analytics.v2.on_time') }</span>
        </div>
        <div class="mb-3 h-2 overflow-hidden rounded-full bg-ink-100">
          <div
            class="h-full rounded-full {analytics.collection_rate >= 90
              ? 'bg-emerald-500'
              : analytics.collection_rate >= 70
                ? 'bg-amber-500'
                : 'bg-red-500'}"
            style="width: {Math.min(analytics.collection_rate, 100)}%"
          ></div>
        </div>
        <p class="text-xs text-ink-500">{collectionCaption(analytics, tr)}</p>
      </Card>

      <Card>
        <h2 class="mb-3 text-sm font-semibold text-ink-900">{ $t('admin.billing.analytics.v2.avg_days') }</h2>
        <div class="mb-2 flex items-baseline gap-2">
          <span class="text-2xl font-semibold tabular-nums text-ink-900">
            {analytics.avg_days_to_pay}
          </span>
          <span class="text-xs text-ink-400">{ $t('admin.billing.analytics.v2.avg_sub') }</span>
        </div>
        <p class="text-xs text-ink-500">{avgDaysCaption(analytics, tr)}</p>
      </Card>

      <Card>
        <h2 class="mb-3 text-sm font-semibold text-ink-900">{ $t('admin.billing.analytics.v2.sub_status') }</h2>
        {#if analytics.subscription_breakdown.length}
          <div class="space-y-2">
            {#each analytics.subscription_breakdown as row (row.status)}
              <div class="flex items-center justify-between gap-3">
                <Badge
                  tone={row.status === 'active' || row.status === 'grace_active'
                    ? 'positive'
                    : row.status === 'cancelled'
                      ? 'negative'
                      : 'warning'}
                  label={subscriptionStatusLabel(row.status, tr)}
                />
                <span class="text-sm font-medium tabular-nums text-ink-700">{row.count}</span>
              </div>
            {/each}
          </div>
          <p class="mt-3 text-xs text-ink-400">
            Churn bulan ini {analytics.churn_rate}%.
          </p>
        {:else}
          <EmptyState icon="users" title={ $t('admin.billing.analytics.v2.sum_none') } hint={ $t('admin.billing.analytics.v2.churn_hint') } />
        {/if}
      </Card>
    </div>
  {/if}
</AppShell>
