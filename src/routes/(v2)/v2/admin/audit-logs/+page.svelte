<script lang="ts">
  /*
    Audit log v2.

    Versi lama: `(app)/admin/audit-logs` (278 baris) + komponen superadmin
    (1.247 baris). Temuan yang dikunci gelombang ini (probe data produksi
    2026-09-04, 21.706 baris):

    1. Search tidak mencakup kolom action — kata "collection_run" menghasilkan
       NOL baris padahal 13.594 baris ber-action itu. Diperbaiki di service
       (audit_service.rs) + wildcard di-escape (search "%" dulu mencocokkan
       21.667 baris).
    2. Tanggal rusak dibuang DIEM-DIEM oleh server (`.ok()`); sekarang 400
       eksplisit. Klien tetap memvalidasi sebelum kirim (validateDateRange).
    3. date_to tanggal-only berarti `<= 00:00` sehingga hari TERAKHIR selalu
       kosong; toIsoRange menutup jadi 23:59:59.999 lokal.
    4. 14.331 baris tanpa user (aksi sistem) dan 56 join-miss (user terhapus)
       tampil "—" polos di UI lama — tidak bisa dibedakan. describeActor
       memisahkan Sistem vs User terhapus vs Anonim.
    5. details campuran JSON (20.667) dan teks bebas (1.000); tabel lama
       menampilkan mentah. summarizeDetails meringkas satu baris, modal
       menampilkan penuh.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { extractApiErrorMessage } from '$lib/api/core';
  import type { AuditLog } from '$lib/api/types';
  import {
    actionTone,
    describeActor,
    resourceLabel,
    summarizeDetails,
    toIsoRange,
    validateDateRange,
  } from '$lib/utils/auditInsights';
  import {
    AppShell,
    Badge,
    Button,
    DataTable,
    Field,
    PageHeader,
    RowActions,
    StatTile,
    type Column,
    type FieldOption,
  } from '$lib/components/ds';
  import Modal from '$lib/components/ui/Modal.svelte';
  import { formatDate } from '$lib/components/ds/format';

  let logs = $state<AuditLog[]>([]);
  let loading = $state(true);
  let error = $state('');
  let total = $state(0);
  let pageNum = $state(1);
  const perPage = 25;

  let q = $state('');
  let actionQ = $state('');
  let resourceQ = $state('');
  let dateFrom = $state('');
  let dateTo = $state('');
  let dateError = $state('');

  let detailLog = $state<AuditLog | null>(null);
  let detailOpen = $state(false);

  let debounce: ReturnType<typeof setTimeout> | undefined;

  const resourceOptions: FieldOption[] = [
    { value: '', label: 'Semua modul' },
    ...[
      'billing',
      'settings',
      'auth',
      'mikrotik_alert',
      'mikrotik_router',
      'support_ticket',
      'customer_subscriptions',
      'installation_work_orders',
      'file_records',
      'pppoe',
      'customers',
      'announcements',
      'invoice',
      'ftth_assets',
      'customer_users',
    ].map((v) => ({ value: v, label: resourceLabel(v) })),
  ];

  const filtered = $derived(
    Boolean(q || actionQ || resourceQ || dateFrom || dateTo),
  );

  const stats = $derived.by(() => {
    // Agregat lokal dari HALAMAN ini saja — jujur soal itu di sublabel.
    let sistem = 0;
    let gagal = 0;
    let manusia = 0;
    for (const l of logs) {
      const a = describeActor(l);
      if (a.kind === 'system') sistem += 1;
      else if (a.kind === 'user') manusia += 1;
      if (actionTone(l.action) === 'negative') gagal += 1;
    }
    return { sistem, gagal, manusia };
  });

  async function load() {
    loading = true;
    error = '';
    try {
      const res = await api.audit.listTenant(pageNum, perPage, {
        ...(q ? { search: q } : {}),
        ...(actionQ ? { action: actionQ } : {}),
        ...(resourceQ ? { resource: resourceQ } : {}),
        ...toIsoRange(dateFrom, dateTo),
      });
      logs = res.data;
      total = res.total;
    } catch (e: unknown) {
      error = extractApiErrorMessage(e, 'Gagal memuat log');
      logs = [];
      total = 0;
    } finally {
      loading = false;
    }
  }

  function applyFilters() {
    const v = validateDateRange(dateFrom, dateTo);
    dateError = v || '';
    if (v) return;
    pageNum = 1;
    void load();
  }

  function onSearchInput() {
    clearTimeout(debounce);
    debounce = setTimeout(applyFilters, 500);
  }

  function resetFilters() {
    q = '';
    actionQ = '';
    resourceQ = '';
    dateFrom = '';
    dateTo = '';
    dateError = '';
    pageNum = 1;
    void load();
  }

  function openDetail(l: AuditLog) {
    detailLog = l;
    detailOpen = true;
  }

  const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));

  const columns: Column[] = [
    { key: 'created_at', label: 'Waktu', width: '160px' },
    { key: 'actor', label: 'Aktor' },
    { key: 'action', label: 'Aksi', width: '180px' },
    { key: 'resource', label: 'Modul', width: '150px', hideSm: true },
    { key: 'target', label: 'Target', hideSm: true },
    { key: 'details', label: 'Detail' },
    { key: 'ip', label: 'IP', width: '120px', hideSm: true },
    { key: 'actions', label: '', width: '110px', align: 'right' },
  ];

  onMount(() => {
    if (!$can('read', 'audit_logs')) {
      goto('/unauthorized');
      return;
    }
    void load();
  });
</script>

<AppShell title="Audit log">
  <PageHeader
    title="Audit log"
    desc="Jejak tindakan di tenant ini — {total.toLocaleString('id-ID')} entri tersimpan"
  />

  <div class="grid stats">
    <StatTile label="Dari halaman ini" value={logs.length.toString()} hint="baris tampil · {total.toLocaleString('id-ID')} total" />
    <StatTile label="Oleh manusia" value={stats.manusia.toString()} hint="user aktif pada halaman ini" />
    <StatTile label="Oleh sistem" value={stats.sistem.toString()} hint="pekerja latar, bukan manusia" />
    <StatTile label="Perlu perhatian" value={stats.gagal.toString()} tone={stats.gagal > 0 ? 'negative' : 'neutral'} hint="gagal / terkunci / dihapus" />
  </div>

  <div class="panel">
    <div class="filters">
      <div class="f-search">
        <label class="lbl" for="aq">Cari</label>
        <input
          id="aq"
          class="inp"
          type="search"
          placeholder="aksi, modul, target, detail, nama user…"
          bind:value={q}
          oninput={onSearchInput}
        />
      </div>
      <Field
        id="af-resource"
        label="Modul"
        value={resourceQ}
        type="select"
        options={resourceOptions}
        onchange={(v) => {
          resourceQ = v;
          applyFilters();
        }}
      />
      <div class="f-text">
        <label class="lbl" for="af-action">Aksi persis</label>
        <input
          id="af-action"
          class="inp"
          type="text"
          placeholder="billing.collection_run"
          bind:value={actionQ}
          oninput={onSearchInput}
        />
      </div>
      <div class="f-date">
        <label class="lbl" for="af-from">Dari</label>
        <input id="af-from" class="inp" type="date" bind:value={dateFrom} onchange={applyFilters} />
      </div>
      <div class="f-date">
        <label class="lbl" for="af-to">Sampai</label>
        <input id="af-to" class="inp" type="date" bind:value={dateTo} onchange={applyFilters} />
      </div>
      <div class="f-btn">
        <Button variant="ghost" size="sm" onclick={resetFilters}>Bersihkan</Button>
      </div>
    </div>
    {#if dateError}
      <p class="date-err" role="alert">{dateError}</p>
    {/if}
    <p class="hint">
      Kata kunci diperlakukan persis — karakter % dan _ tidak lagi menjadi
      wildcard. Filter tanggal memakai hari lokal penuh (sampai 23:59:59).
    </p>
  </div>

  {#if error}
    <div class="err" role="alert">{error}</div>
  {/if}

  <DataTable
    {columns}
    rows={logs}
    {loading}
    emptyTitle={filtered ? 'Tidak ada yang cocok' : 'Belum ada aktivitas tercatat'}
    emptyHint={filtered
      ? 'Longgarkan filter atau bersihkan kata kunci.'
      : 'Entri muncul saat ada tindakan admin, sistem, atau autentikasi.'}
    footNote="{logs.length} dari {total.toLocaleString('id-ID')} entri · halaman {pageNum}/{totalPages}"
  >
    {#snippet cell(l, c)}
      {#if c.key === 'created_at'}
        <span class="mono">{formatDate(l.created_at, true)}</span>
      {:else if c.key === 'actor'}
        {@const a = describeActor(l)}
        <span title={a.detail} class={a.kind === 'system' ? 'muted' : ''}>{a.label}</span>
      {:else if c.key === 'action'}
        <Badge tone={actionTone(l.action)} label={l.action} />
      {:else if c.key === 'resource'}
        {resourceLabel(l.resource)}
      {:else if c.key === 'target'}
        <span class="mono">{l.resource_name || l.resource_id || '—'}</span>
      {:else if c.key === 'details'}
        <span class="det" title={l.details || ''}>{summarizeDetails(l.details).summary}</span>
      {:else if c.key === 'ip'}
        <span class="mono">{l.ip_address || '—'}</span>
      {:else if c.key === 'actions'}
        <RowActions
          primary={{ label: 'Detail', icon: 'search', onclick: () => openDetail(l) }}
        />
      {:else}
        {String((l as unknown as Record<string, unknown>)[c.key] ?? '—')}
      {/if}
    {/snippet}
  </DataTable>

  <div class="pager">
    <div class="pbtns">
      <Button
        variant="ghost"
        size="sm"
        disabled={pageNum <= 1}
        onclick={() => {
          pageNum -= 1;
          void load();
        }}>Sebelumnya</Button
      >
      <Button
        variant="ghost"
        size="sm"
        disabled={pageNum >= totalPages}
        onclick={() => {
          pageNum += 1;
          void load();
        }}>Berikutnya</Button
      >
    </div>
  </div>
</AppShell>

<Modal bind:show={detailOpen} title="Detail entri audit" width="640px">
  {#if detailLog}
    {@const d = summarizeDetails(detailLog.details)}
    {@const actor = describeActor(detailLog)}
    <dl class="detail">
      <div><dt>Waktu</dt><dd>{formatDate(detailLog.created_at, true)}</dd></div>
      <div><dt>Aktor</dt><dd title={actor.detail}>{actor.label}</dd></div>
      <div><dt>Aksi</dt><dd><Badge tone={actionTone(detailLog.action)} label={detailLog.action} /></dd></div>
      <div><dt>Modul</dt><dd>{resourceLabel(detailLog.resource)}</dd></div>
      <div><dt>Target</dt><dd class="mono">{detailLog.resource_name || detailLog.resource_id || '—'}</dd></div>
      <div><dt>IP</dt><dd class="mono">{detailLog.ip_address || '—'}</dd></div>
    </dl>
    <div class="det-block">
      <p class="det-title">Detail{d.kind === 'json' ? ' (terurai)' : ''}</p>
      {#if d.kind === 'json' && d.fields.length}
        <dl class="detail">
          {#each d.fields as f}
            <div><dt>{f.key}</dt><dd class="mono">{f.value}</dd></div>
          {/each}
        </dl>
      {:else if d.kind === 'empty'}
        <p class="muted">Tidak ada detail tersimpan untuk entri ini.</p>
      {:else}
        <p class="mono det-raw">{d.summary}</p>
      {/if}
    </div>
  {/if}
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (detailOpen = false)}>Tutup</Button>
  {/snippet}
</Modal>

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }
  .panel {
    background: var(--ds-surface, #fff);
    border: 1px solid var(--ds-border, #e4e4e7);
    border-radius: 12px;
    padding: 14px 16px 10px;
    margin-bottom: 14px;
  }
  .filters {
    display: grid;
    grid-template-columns: minmax(240px, 2fr) repeat(2, minmax(150px, 1fr)) repeat(2, 150px) auto;
    gap: 12px;
    align-items: end;
  }
  @media (max-width: 1100px) {
    .filters {
      grid-template-columns: 1fr 1fr;
    }
  }
  .lbl {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: var(--ds-ink-500, #52525b);
    margin-bottom: 4px;
  }
  .inp {
    width: 100%;
    height: 36px;
    border-radius: 8px;
    border: 1px solid var(--ds-border, #e4e4e7);
    background: #fff;
    padding: 0 10px;
    font-size: 13px;
    color: var(--ds-ink-900, #18181b);
  }
  .inp:focus-visible {
    outline: 2px solid var(--ds-brand-600, #4f46e5);
    outline-offset: 1px;
  }
  .f-btn {
    padding-bottom: 2px;
  }
  .hint {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--ds-ink-500, #52525b);
  }
  .date-err {
    margin: 8px 0 0;
    font-size: 12px;
    color: var(--ds-negative, #b91c1c);
  }
  .err {
    margin-bottom: 12px;
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.25);
    color: var(--ds-negative, #b91c1c);
    font-size: 13px;
  }
  .det {
    display: block;
    max-width: 420px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
    color: var(--ds-ink-500, #52525b);
  }
  .muted {
    color: var(--ds-ink-500, #52525b);
  }
  .mono {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    word-break: break-all;
  }
  .pager {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 12px;
    gap: 12px;
    flex-wrap: wrap;
  }
  .pbtns {
    display: flex;
    gap: 8px;
  }
  .detail {
    display: grid;
    gap: 8px;
    margin: 0;
  }
  .detail > div {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 10px;
    align-items: baseline;
  }
  .detail dt {
    font-size: 12px;
    font-weight: 600;
    color: var(--ds-ink-500, #52525b);
  }
  .detail dd {
    margin: 0;
    font-size: 13px;
    color: var(--ds-ink-900, #18181b);
  }
  .det-block {
    margin-top: 14px;
    border-top: 1px solid var(--ds-border, #e4e4e7);
    padding-top: 12px;
  }
  .det-title {
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 700;
    color: var(--ds-ink-500, #52525b);
  }
  .det-raw {
    margin: 0;
    white-space: pre-wrap;
  }
</style>
