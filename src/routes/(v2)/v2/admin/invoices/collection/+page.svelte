<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    api,
    type BillingCollectionLogView,
    type BillingCollectionRunResult,
    type InvoiceReminderLogView,
  } from '$lib/api/client';
  import { appSettings } from '$lib/stores/settings';
  import { can } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { extractApiErrorMessage } from '$lib/api/core';
  import type { Column } from '$lib/components/ds/table-types';
  import {
    collectionActionHint,
    collectionActionLabel,
    collectionActionTone,
    collectionReminderLabel,
    collectionResultLabel,
    collectionResultTone,
    toIsoUtc,
  } from '$lib/utils/collectionLogInsights';
  import { loadCollectionExportModule } from '../../../../../(app)/admin/invoices/collection/collectionPageModules';
  import {
    AppShell,
    Badge,
    Button,
    Card,
    DataTable,
    Field,
    PageHeader,
    StatTile,
    Tabs,
  } from '$lib/components/ds';

  type ActiveTab = 'collection' | 'reminders';

  let activeTab = $state<ActiveTab>('collection');
  let loadingCollection = $state(true);
  let loadingReminders = $state(true);
  let runningNow = $state(false);
  let ready = $state(false);

  let collectionRows = $state<BillingCollectionLogView[]>([]);
  let reminderRows = $state<InvoiceReminderLogView[]>([]);
  let lastRunResult = $state<BillingCollectionRunResult | null>(null);
  let collectionLoadSequence = 0;
  let reminderLoadSequence = 0;
  const canManageBilling = $derived($can('manage', 'billing'));

  let collectionAction = $state('all');
  let collectionResult = $state('all');
  let collectionSearch = $state('');
  let collectionFrom = $state('');
  let collectionTo = $state('');
  let collectionLimit = $state(200);

  let reminderCode = $state('all');
  let reminderStatus = $state('all');
  let reminderSearch = $state('');
  let reminderFrom = $state('');
  let reminderTo = $state('');
  let reminderLimit = $state(200);

  const LIMITS = [100, 200, 500, 1000];

  const collectionActionOptions = $derived.by(() => {
    const fromData = collectionRows.map((row) => row.action).filter(Boolean);
    return Array.from(
      new Set([
        'reminder',
        'suspend',
        'grace_expire_suspend',
        'resume',
        'installation',
        'assignment',
        'payment_callback',
        ...fromData,
      ]),
    );
  });
  const collectionResultOptions = $derived.by(() => {
    const fromData = collectionRows.map((row) => row.result).filter(Boolean);
    return Array.from(new Set(['success', 'skipped', 'failed', ...fromData]));
  });
  const reminderCodeOptions = $derived.by(() => {
    const fromData = reminderRows.map((row) => row.reminder_code).filter(Boolean);
    return Array.from(new Set(['d7', 'd3', 'd0', ...fromData]));
  });
  const reminderStatusOptions = $derived.by(() => {
    const fromData = reminderRows.map((row) => row.status).filter(Boolean);
    return Array.from(new Set(['sent', 'queued', 'failed', ...fromData]));
  });

  const currentLoading = $derived(
    activeTab === 'collection' ? loadingCollection : loadingReminders,
  );

  onMount(() => {
    void (async () => {
      if (!$can('read', 'billing') && !$can('manage', 'billing')) {
        goto('/unauthorized');
        return;
      }
      await Promise.all([loadCollection(), loadReminders()]);
      ready = true;
    })();
  });

  $effect(() => {
    if (!ready || activeTab !== 'collection') return;
    void collectionAction;
    void collectionResult;
    void collectionSearch;
    void collectionFrom;
    void collectionTo;
    void collectionLimit;
    const timer = setTimeout(() => void loadCollection(), 280);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (!ready || activeTab !== 'reminders') return;
    void reminderCode;
    void reminderStatus;
    void reminderSearch;
    void reminderFrom;
    void reminderTo;
    void reminderLimit;
    const timer = setTimeout(() => void loadReminders(), 280);
    return () => clearTimeout(timer);
  });

  async function loadCollection() {
    const seq = ++collectionLoadSequence;
    loadingCollection = true;
    try {
      const rows = await api.payment.listBillingCollectionLogs({
        action: collectionAction === 'all' ? undefined : collectionAction,
        result: collectionResult === 'all' ? undefined : collectionResult,
        search: collectionSearch.trim() || undefined,
        from: toIsoUtc(collectionFrom),
        to: toIsoUtc(collectionTo),
        limit: collectionLimit,
      });
      if (seq === collectionLoadSequence) collectionRows = rows;
    } catch (e) {
      if (seq === collectionLoadSequence) toast.error(extractApiErrorMessage(e) || 'Gagal memuat log penagihan.');
    } finally {
      if (seq === collectionLoadSequence) loadingCollection = false;
    }
  }

  async function loadReminders() {
    const seq = ++reminderLoadSequence;
    loadingReminders = true;
    try {
      const rows = await api.payment.listInvoiceReminderLogs({
        reminderCode: reminderCode === 'all' ? undefined : reminderCode,
        status: reminderStatus === 'all' ? undefined : reminderStatus,
        search: reminderSearch.trim() || undefined,
        from: toIsoUtc(reminderFrom),
        to: toIsoUtc(reminderTo),
        limit: reminderLimit,
      });
      if (seq === reminderLoadSequence) reminderRows = rows;
    } catch (e) {
      if (seq === reminderLoadSequence) toast.error(extractApiErrorMessage(e) || 'Gagal memuat log pengingat.');
    } finally {
      if (seq === reminderLoadSequence) loadingReminders = false;
    }
  }

  async function refreshCurrent() {
    if (runningNow) return;
    if (activeTab === 'collection') await loadCollection();
    else await loadReminders();
  }

  async function runCollectionNow() {
    if (!canManageBilling || runningNow || loadingCollection || loadingReminders) return;
    runningNow = true;
    try {
      lastRunResult = await api.payment.runBillingCollectionNow();
      toast.success('Penagihan manual selesai.');
      await Promise.all([loadCollection(), loadReminders()]);
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal jalankan penagihan.');
    } finally {
      runningNow = false;
    }
  }

  function switchTab(next: string) {
    if (next === activeTab) return;
    activeTab = next as ActiveTab;
  }

  function clearFilters() {
    if (activeTab === 'collection') {
      collectionAction = 'all';
      collectionResult = 'all';
      collectionSearch = '';
      collectionFrom = '';
      collectionTo = '';
      collectionLimit = 200;
      void loadCollection();
      return;
    }
    reminderCode = 'all';
    reminderStatus = 'all';
    reminderSearch = '';
    reminderFrom = '';
    reminderTo = '';
    reminderLimit = 200;
    void loadReminders();
  }

  async function exportCsv() {
    const rows =
      activeTab === 'collection' ? buildCollectionExportRows() : buildReminderExportRows();
    const { exportCsvRows } = await loadCollectionExportModule();
    const ok = exportCsvRows(
      rows,
      activeTab === 'collection' ? 'billing-collection-logs' : 'invoice-reminder-logs',
    );
    if (!ok) {
      toast.error('Tidak ada data untuk diekspor.');
      return;
    }
    toast.success('Ekspor selesai.');
  }

  async function exportExcel() {
    const rows =
      activeTab === 'collection' ? buildCollectionExportRows() : buildReminderExportRows();
    const { exportExcelRows } = await loadCollectionExportModule();
    const ok = exportExcelRows(
      rows,
      activeTab === 'collection' ? 'billing-collection-logs' : 'invoice-reminder-logs',
    );
    if (!ok) {
      toast.error('Tidak ada data untuk diekspor.');
      return;
    }
    toast.success('Ekspor selesai.');
  }

  function buildCollectionExportRows() {
    return collectionRows.map((row) => ({
      time: formatDateTime(row.created_at, { timeZone: $appSettings.app_timezone }),
      invoice: row.invoice_number || row.invoice_id,
      customer: row.customer_name || '—',
      action: row.action,
      result: row.result,
      invoice_status: row.invoice_status || '—',
      subscription_status: row.subscription_status || '—',
      reason: row.reason || '',
    }));
  }

  function buildReminderExportRows() {
    return reminderRows.map((row) => ({
      time: formatDateTime(row.created_at, { timeZone: $appSettings.app_timezone }),
      invoice: row.invoice_number || row.invoice_id,
      reminder_code: row.reminder_code,
      channel: row.channel || '—',
      recipient: row.recipient || '—',
      status: row.status,
      detail: row.detail || '',
    }));
  }
</script>
<AppShell title="Log penagihan">
  <PageHeader
    title="Log penagihan"
    eyebrow="Invoice"
    desc="Jejak scheduler billing: pengingat, suspend, aktif-lagi, dan callback."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" onclick={() => void refreshCurrent()} disabled={currentLoading || runningNow}>
        Segarkan
      </Button>
      {#if canManageBilling}
        <Button variant="primary" icon="zap" loading={runningNow} onclick={() => void runCollectionNow()} disabled={runningNow || currentLoading}>
          {runningNow ? 'Menjalankan…' : 'Jalankan sekarang'}
        </Button>
      {/if}
      <Button variant="ghost" onclick={() => void exportCsv()}>Ekspor CSV</Button>
      <Button variant="ghost" onclick={() => void exportExcel()}>Ekspor Excel</Button>
    {/snippet}
  </PageHeader>

  {#if lastRunResult}
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
      <StatTile label="Dievaluasi" value={String(lastRunResult.evaluated_count)} hint="Invoice diproses run ini" />
      <StatTile label="Pengingat terkirim" value={String(lastRunResult.reminder_sent_count)} hint="WA/email terkirim" />
      <StatTile label="Disuspend" value={String(lastRunResult.suspended_count)} hint="Suspend baru" tone="negative" />
      <StatTile label="Aktif lagi" value={String(lastRunResult.resumed_count)} hint="Resume setelah bayar" tone="positive" />
      <StatTile label="Gagal" value={String(lastRunResult.failed_count)} hint="Perlu tindak lanjut" tone="negative" />
    </div>
  {/if}

  <Card>
    <Tabs
      items={[
        { id: 'collection', label: 'Log penagihan', count: collectionRows.length },
        { id: 'reminders', label: 'Log pengingat', count: reminderRows.length },
      ]}
      active={activeTab}
      onselect={switchTab}
    />

    {#if activeTab === 'collection'}
      <div class="mt-3 grid gap-2 sm:grid-cols-3 lg:grid-cols-6">
        <Field stacked type="select" id="col-action" label="Aksi" value={collectionAction} options={[{ value: 'all', label: 'Semua aksi' }, ...collectionActionOptions.map((o) => ({ value: o, label: collectionActionLabel(o) }))]} onchange={(v) => (collectionAction = v)} />
        <Field stacked type="select" id="col-result" label="Hasil" value={collectionResult} options={[{ value: 'all', label: 'Semua hasil' }, ...collectionResultOptions.map((o) => ({ value: o, label: collectionResultLabel(o) }))]} onchange={(v) => (collectionResult = v)} />
        <Field id="col-search" label="Cari" placeholder="Invoice / pelanggan…" value={collectionSearch} onchange={(v) => (collectionSearch = v)} />
        <Field id="col-from" label="Dari" type="text" placeholder="2026-09-01 10:00" value={collectionFrom} onchange={(v) => (collectionFrom = v)} />
        <Field id="col-to" label="Sampai" type="text" placeholder="2026-09-06 10:00" value={collectionTo} onchange={(v) => (collectionTo = v)} />
        <Field stacked type="select" id="col-limit" label="Batas" value={String(collectionLimit)} options={LIMITS.map((l) => ({ value: String(l), label: String(l) }))} onchange={(v) => (collectionLimit = Number(v))} />
      </div>
      <div class="mt-2">
        <Button variant="ghost" onclick={clearFilters}>Bersihkan filter</Button>
      </div>
      <div class="mt-3">
        {#if loadingCollection}
          <p class="py-8 text-center text-sm text-ink-500">Memuat log…</p>
        {:else if collectionRows.length === 0}
          <p class="py-8 text-center text-sm text-ink-500">Belum ada log penagihan. Jalankan manual atau tunggu scheduler.</p>
        {:else}
          <DataTable
            columns={[
              { key: 'time', label: 'Waktu' },
              { key: 'invoice', label: 'Invoice' },
              { key: 'customer', label: 'Pelanggan' },
              { key: 'action', label: 'Aksi' },
              { key: 'result', label: 'Hasil' },
              { key: 'reason', label: 'Alasan' },
            ]}
            rows={collectionRows}
          >
            {#snippet cell(row: BillingCollectionLogView, col: Column)}
              {#if col.key === 'time'}
                <div><div class="font-medium">{formatDateTime(row.created_at, { timeZone: $appSettings.app_timezone })}</div><div class="text-xs text-ink-500">{timeAgo(row.created_at)}</div></div>
              {:else if col.key === 'invoice'}
                <div><div class="font-medium">{row.invoice_number || row.invoice_id}</div><div class="text-xs text-ink-500">{row.invoice_status || '—'}</div></div>
              {:else if col.key === 'customer'}
                <div><div class="font-medium">{row.customer_name || '—'}</div><div class="text-xs text-ink-500">{row.subscription_status || '—'}</div></div>
              {:else if col.key === 'action'}
                <div><Badge tone={collectionActionTone(row.action)} label={collectionActionLabel(row.action)} /><div class="mt-1 text-xs text-ink-500">{collectionActionHint(row.action)}</div></div>
              {:else if col.key === 'result'}
                <Badge tone={collectionResultTone(row.result)} label={collectionResultLabel(row.result)} />
              {:else}
                <span class="text-xs">{row.reason || '—'}</span>
              {/if}
            {/snippet}
          </DataTable>
        {/if}
      </div>
    {:else}
      <div class="mt-3 grid gap-2 sm:grid-cols-3 lg:grid-cols-6">
        <Field stacked type="select" id="rem-code" label="Pengingat" value={reminderCode} options={[{ value: 'all', label: 'Semua' }, ...reminderCodeOptions.map((o) => ({ value: o, label: collectionReminderLabel(o) }))]} onchange={(v) => (reminderCode = v)} />
        <Field stacked type="select" id="rem-status" label="Status" value={reminderStatus} options={[{ value: 'all', label: 'Semua status' }, ...reminderStatusOptions.map((o) => ({ value: o, label: collectionResultLabel(o) }))]} onchange={(v) => (reminderStatus = v)} />
        <Field id="rem-search" label="Cari" placeholder="Invoice / penerima…" value={reminderSearch} onchange={(v) => (reminderSearch = v)} />
        <Field id="rem-from" label="Dari" type="text" placeholder="2026-09-01 10:00" value={reminderFrom} onchange={(v) => (reminderFrom = v)} />
        <Field id="rem-to" label="Sampai" type="text" placeholder="2026-09-06 10:00" value={reminderTo} onchange={(v) => (reminderTo = v)} />
        <Field stacked type="select" id="rem-limit" label="Batas" value={String(reminderLimit)} options={LIMITS.map((l) => ({ value: String(l), label: String(l) }))} onchange={(v) => (reminderLimit = Number(v))} />
      </div>
      <div class="mt-2">
        <Button variant="ghost" onclick={clearFilters}>Bersihkan filter</Button>
      </div>
      <div class="mt-3">
        {#if loadingReminders}
          <p class="py-8 text-center text-sm text-ink-500">Memuat log…</p>
        {:else if reminderRows.length === 0}
          <p class="py-8 text-center text-sm text-ink-500">Belum ada log pengingat.</p>
        {:else}
          <DataTable
            columns={[
              { key: 'time', label: 'Waktu' },
              { key: 'invoice', label: 'Invoice' },
              { key: 'reminder', label: 'Pengingat' },
              { key: 'channel', label: 'Kanal' },
              { key: 'status', label: 'Status' },
              { key: 'detail', label: 'Detail' },
            ]}
            rows={reminderRows}
          >
            {#snippet cell(row: InvoiceReminderLogView, col: Column)}
              {#if col.key === 'time'}
                <div><div class="font-medium">{formatDateTime(row.created_at, { timeZone: $appSettings.app_timezone })}</div><div class="text-xs text-ink-500">{timeAgo(row.created_at)}</div></div>
              {:else if col.key === 'invoice'}
                <div class="font-medium">{row.invoice_number || row.invoice_id}</div>
              {:else if col.key === 'reminder'}
                <Badge label={collectionReminderLabel(row.reminder_code)} />
              {:else if col.key === 'channel'}
                <span class="text-xs">{row.channel || '—'}</span>
              {:else if col.key === 'status'}
                <Badge tone={collectionResultTone(row.status)} label={collectionResultLabel(row.status)} />
              {:else}
                <span class="text-xs">{row.detail || row.recipient || '—'}</span>
              {/if}
            {/snippet}
          </DataTable>
        {/if}
      </div>
    {/if}
  </Card>
</AppShell>
