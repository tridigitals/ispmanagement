<script lang="ts">
  /*
    Rekonsiliasi lifecycle v2 — gelombang 24b.

    Versi lama: (app)/admin/customers/lifecycle-reconciliation/+page.svelte
    (679 baris). Perilaku identik: laporan isu lifecycle + perbaikan
    massal 2 tipe (konfirmasi), filter isu, search, pager server-side.
    Label isu/aksi/periode kini dari helper murni
    lifecycleReconciliation (4 tes).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    api,
    type CustomerServiceLifecycleIssue,
    type CustomerServiceLifecycleRepairResult,
    type CustomerServiceLifecycleReport,
  } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import {
    lifecycleActionLabel,
    lifecycleIssueLabel,
    lifecyclePeriod,
    lifecycleServiceLabel,
  } from '$lib/utils/lifecycleReconciliation';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    PageHeader,
    StatTile,
  } from '$lib/components/ds';
  import type { Column } from '$lib/components/ds/table-types';

  const columns: Column[] = [
    { key: 'customer', label: 'Pelanggan' },
    { key: 'service', label: 'Layanan' },
    { key: 'issue', label: 'Masalah' },
    { key: 'action', label: 'Aksi anjuran' },
  ];

  let report = $state<CustomerServiceLifecycleReport>({
    generated_at: '',
    total_issues: 0,
    missing_bootstrap_invoice: 0,
    invalid_active_lifecycle: 0,
    page: 1,
    per_page: 25,
    data: [],
  });
  let repairResult = $state<CustomerServiceLifecycleRepairResult | null>(null);
  let repairConfirmOpen = $state(false);
  let pendingRepairType = $state<'missing_bootstrap_invoice' | 'invalid_active_lifecycle' | null>(null);
  let loading = $state(true);
  let repairing = $state(false);
  let error = $state('');
  let q = $state('');
  let issueFilter = $state<'all' | 'missing_bootstrap_invoice' | 'invalid_active_lifecycle'>('all');
  let page = $state(0);
  let perPage = $state(25);

  const canRead = $derived($can('read', 'customers') || $can('create', 'orders'));
  const canRepair = $derived($can('manage', 'billing'));

  const issueOptions = [
    { value: 'all', label: 'Semua masalah' },
    { value: 'missing_bootstrap_invoice', label: 'Belum ada invoice awal' },
    { value: 'invalid_active_lifecycle', label: 'Lifecycle aktif tidak valid' },
  ];
  const perPageOptions = [
    { value: '25', label: '25' },
    { value: '50', label: '50' },
    { value: '100', label: '100' },
  ];

  function openCustomer(issue: CustomerServiceLifecycleIssue) {
    goto(`/v2/admin/customers/${issue.customer_id}?tab=subscriptions`);
  }

  onMount(async () => {
    if (!canRead) {
      goto('/unauthorized');
      return;
    }
    await loadReport();
  });

  async function loadReport() {
    loading = true;
    error = '';
    try {
      report = await api.customers.reconciliation.report({
        q: q.trim() || undefined,
        issueType: issueFilter,
        page: page + 1,
        perPage,
      });
      if (!repairing) repairResult = null;
    } catch (e) {
      error = extractApiErrorMessage(e);
      toast.error(error);
    } finally {
      loading = false;
    }
  }

  function requestRepair(issueType: 'missing_bootstrap_invoice' | 'invalid_active_lifecycle') {
    if (repairing) return;
    pendingRepairType = issueType;
    repairConfirmOpen = true;
  }

  async function repairIssues(issueType: 'missing_bootstrap_invoice' | 'invalid_active_lifecycle') {
    if (repairing) return;
    repairing = true;
    try {
      const result = await api.customers.reconciliation.repair(issueType);
      repairResult = result;
      toast.success(`Cocok ${result.matched_count}, diperbaiki ${result.repaired_count}, dilewati ${result.skipped_count}, gagal ${result.failed_count}.`);
      await loadReport();
    } catch (e) {
      toast.error(extractApiErrorMessage(e));
    } finally {
      repairing = false;
    }
  }

  async function setIssueFilter(next: 'all' | 'missing_bootstrap_invoice' | 'invalid_active_lifecycle') {
    if (issueFilter === next) return;
    issueFilter = next;
    page = 0;
    await loadReport();
  }

  function repairConfirmMessage(): string {
    return pendingRepairType === 'invalid_active_lifecycle'
      ? 'Suspend semua layanan dengan lifecycle aktif yang tidak valid? Tindakan ini berdampak ke pelanggan.'
      : 'Buat invoice awal untuk semua layanan yang belum punya? Tindakan ini berdampak ke tagihan.';
  }
</script>
<AppShell title="Rekonsiliasi lifecycle">
  <PageHeader
    title="Rekonsiliasi lifecycle"
    eyebrow="Pelanggan"
    desc="Layanan yang lifecycle-nya tidak sinkron dengan tagihan — periksa lalu perbaiki massal."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void loadReport()} disabled={loading || repairing}>
        Segarkan
      </Button>
      {#if canRepair}
        <Button
          variant="primary"
          onclick={() => requestRepair('missing_bootstrap_invoice')}
          disabled={repairing || loading || report.missing_bootstrap_invoice === 0}
        >
          Buat invoice awal ({report.missing_bootstrap_invoice})
        </Button>
        <Button
          variant="danger"
          onclick={() => requestRepair('invalid_active_lifecycle')}
          disabled={repairing || loading || report.invalid_active_lifecycle === 0}
        >
          Suspend invalid ({report.invalid_active_lifecycle})
        </Button>
      {/if}
    {/snippet}
  </PageHeader>

  <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
    <StatTile label="Total masalah" value={String(report.total_issues)} hint="semua isu terbuka" tone="warning" />
    <StatTile label="Belum ada invoice awal" value={String(report.missing_bootstrap_invoice)} hint="bisa diperbaiki massal" tone="warning" />
    <StatTile label="Lifecycle aktif invalid" value={String(report.invalid_active_lifecycle)} hint="perlu suspend" tone="negative" />
    <StatTile label="Baris tampil" value={String(report.data.length)} hint={`dari ${report.total_issues} masalah`} />
  </div>

  <Card title="Filter">
    <div class="grid gap-3 sm:grid-cols-3">
      <Field id="rc-q" label="Cari" type="text" stacked value={q} onchange={(v) => { q = v; page = 0; void loadReport(); }} placeholder="Cari nama pelanggan…" />
      <Field id="rc-issue" label="Jenis masalah" type="select" stacked value={issueFilter} options={issueOptions} onchange={(v) => void setIssueFilter(v as typeof issueFilter)} />
      <Field id="rc-perpage" label="Per halaman" type="select" stacked value={String(perPage)} options={perPageOptions} onchange={(v) => { perPage = Number(v); page = 0; void loadReport(); }} />
    </div>
  </Card>

  {#if error}
    <Card><p class="text-sm text-red-700">{error}</p></Card>
  {/if}

  {#if repairResult}
    <Card title="Hasil perbaikan terakhir">
      <div class="grid gap-3 sm:grid-cols-4">
        <StatTile label="Cocok" value={String(repairResult.matched_count)} hint="kandidat ditemukan" />
        <StatTile label="Diperbaiki" value={String(repairResult.repaired_count)} hint="berhasil" tone="positive" />
        <StatTile label="Dilewati" value={String(repairResult.skipped_count)} hint="tidak perlu aksi" />
        <StatTile label="Gagal" value={String(repairResult.failed_count)} hint="perlu tinjau manual" tone="negative" />
      </div>
      {#if repairResult.errors.length > 0}
        <ul class="mt-3 list-disc space-y-1 pl-5 text-sm text-red-700">
          {#each repairResult.errors as repairError}
            <li>{repairError}</li>
          {/each}
        </ul>
      {/if}
    </Card>
  {/if}

  <Card title={`Daftar masalah (${report.total_issues})`} padded={false}>
    <DataTable
      {columns}
      rows={report.data}
      {loading}
      emptyTitle="Tidak ada masalah lifecycle"
      emptyHint="Semua layanan sinkron dengan tagihan."
      footNote={report.generated_at ? `Dibuat ${report.generated_at.replace('T', ' ').slice(0, 16)}` : undefined}
    >
      {#snippet cell(item, column)}
        {#if column.key === 'customer'}
          <div>
            <button type="button" class="text-sm font-semibold text-ink-900 hover:underline" onclick={() => openCustomer(item)}>
              {item.customer_name}
            </button>
            <div class="font-mono text-xs text-ink-400">{item.subscription_id}</div>
          </div>
        {:else if column.key === 'service'}
          <div>
            <div class="text-sm text-ink-800">{lifecycleServiceLabel(item.package_name, item.location_label)}</div>
            <div class="text-xs text-ink-400">{lifecyclePeriod(item.starts_at, item.ends_at)}</div>
          </div>
        {:else if column.key === 'issue'}
          <div>
            <Badge tone="warning" label={lifecycleIssueLabel(item.issue_type)} />
            <div class="mt-0.5 text-xs text-ink-400">{item.subscription_status}</div>
          </div>
        {:else if column.key === 'action'}
          <div class="flex items-center gap-2">
            <span class="text-sm text-ink-700">{lifecycleActionLabel(item.recommended_action)}</span>
            <Button variant="ghost" onclick={() => openCustomer(item)}>Buka layanan</Button>
          </div>
        {/if}
      {/snippet}
    </DataTable>
    <div class="flex items-center justify-between border-t border-ink-200 px-4 py-3">
      <span class="text-xs text-ink-500">Halaman {page + 1}</span>
      <div class="flex gap-2">
        <Button variant="ghost" onclick={() => { if (page > 0) { page -= 1; void loadReport(); } }} disabled={page === 0 || loading}>
          Sebelumnya
        </Button>
        <Button variant="ghost" onclick={() => { page += 1; void loadReport(); }} disabled={loading || report.data.length < perPage}>
          Berikutnya
        </Button>
      </div>
    </div>
  </Card>
</AppShell>

<ConfirmDialog
  bind:show={repairConfirmOpen}
  title="Jalankan perbaikan massal?"
  message={repairConfirmMessage()}
  confirmText="Jalankan"
  cancelText="Batal"
  type={pendingRepairType === 'invalid_active_lifecycle' ? 'danger' : 'warning'}
  loading={repairing}
  onconfirm={() => {
    repairConfirmOpen = false;
    if (pendingRepairType) void repairIssues(pendingRepairType);
  }}
  oncancel={() => (pendingRepairType = null)}
/>
