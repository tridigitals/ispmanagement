<script lang="ts">
  /*
    Impor PPPoE v2 — gelombang 24c.

    Versi lama: (app)/admin/network/pppoe/import/+page.svelte (743 baris).
    Wizard 3 langkah identik: pilih router + mapping opsional → pindai →
    preview centang (default baru+perbarui) → impor → hasil. Label/tone/
    validasi/seleksi kini helper murni pppoeImportInsights (3 tes).
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { can } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import {
    pppoeActionLabel,
    pppoeActionTone,
    pppoeDefaultSelection,
    pppoeMappingError,
    pppoeSummary,
    type PppoeCandidate,
  } from '$lib/utils/pppoeImportInsights';
  import type { Column } from '$lib/components/ds/table-types';
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

  type RouterRow = { id: string; name: string };
  type CustomerRow = { id: string; name: string };
  type LocationRow = { id: string; label: string };

  let step = $state<1 | 2 | 3>(1);
  let loading = $state(false);

  let routers = $state<RouterRow[]>([]);
  let routerId = $state('');
  let includeDisabled = $state(false);

  let customers = $state<CustomerRow[]>([]);
  let customerId = $state('');
  let locations = $state<LocationRow[]>([]);
  let locationId = $state('');

  let candidates = $state<PppoeCandidate[]>([]);
  let selected = $state<Set<string>>(new Set());
  let result = $state<any | null>(null);

  const summary = $derived(pppoeSummary(candidates));

  const columns: Column[] = [
    { key: 'pick', label: '' },
    { key: 'username', label: 'Username' },
    { key: 'profile', label: 'Profil' },
    { key: 'remote', label: 'Remote' },
    { key: 'disabled', label: 'Status' },
    { key: 'action', label: 'Aksi' },
    { key: 'pw', label: 'Password' },
  ];

  onMount(async () => {
    if (!$can('manage', 'pppoe') && !$can('read', 'pppoe')) {
      goto('/unauthorized');
      return;
    }
    await loadRouters();
    await loadCustomers();
  });

  async function loadRouters() {
    try {
      routers = (await api.mikrotik.routers.list()) as any;
      if (!routerId && routers.length) routerId = routers[0].id;
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal memuat router.');
    }
  }

  async function loadCustomers() {
    try {
      const res = await api.customers.list({ page: 1, perPage: 1000 });
      customers = (res.data || []).map((c) => ({ id: c.id, name: c.name }));
    } catch {
      customers = [];
    }
  }

  async function loadLocationsForCustomer(cid: string) {
    locations = [];
    locationId = '';
    if (!cid) return;
    try {
      locations = (await api.customers.locations.list(cid)) as any;
      if (locations.length) locationId = locations[0].id;
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal memuat lokasi.');
    }
  }

  function resetPreview() {
    candidates = [];
    selected = new Set();
    result = null;
    step = 1;
  }

  function toggleAll(kind: 'new_update' | 'all' | 'none') {
    if (kind === 'none') {
      selected = new Set();
      return;
    }
    if (kind === 'all') {
      selected = new Set(candidates.map((c) => c.username));
      return;
    }
    selected = new Set(pppoeDefaultSelection(candidates));
  }

  async function scan() {
    if (!routerId) {
      toast.error('Pilih router dulu.');
      return;
    }
    loading = true;
    try {
      const rows = (await api.pppoe.import.preview(routerId, { include_disabled: includeDisabled })) as any[];
      candidates = (rows || []) as any;
      selected = new Set(pppoeDefaultSelection(candidates));
      step = 2;
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal pindai router.');
    } finally {
      loading = false;
    }
  }

  async function runImport() {
    if (!$can('manage', 'pppoe')) {
      toast.error('Akses ditolak.');
      return;
    }
    if (!routerId) return;
    const usernames = Array.from(selected);
    if (usernames.length === 0) {
      toast.error('Pilih minimal satu akun.');
      return;
    }
    const mappingErr = pppoeMappingError(customerId, locationId);
    if (mappingErr) {
      toast.error(mappingErr);
      return;
    }
    loading = true;
    try {
      result = await api.pppoe.import.run(routerId, {
        usernames,
        customer_id: customerId || undefined,
        location_id: locationId || undefined,
      });
      step = 3;
    } catch (e) {
      toast.error(extractApiErrorMessage(e) || 'Gagal impor.');
    } finally {
      loading = false;
    }
  }

  function toggleOne(id: string, on: boolean) {
    const next = new Set(selected);
    if (on) next.add(id);
    else next.delete(id);
    selected = next;
  }
</script>
<AppShell title="Impor PPPoE">
  <PageHeader
    title="Impor PPPoE"
    eyebrow="Jaringan"
    desc="Tarik akun PPPoE dari router MikroTik ke database."
  >
    {#snippet actions()}
      <Button variant="ghost" href="/v2/admin/network/pppoe">Kembali ke PPPoE</Button>
    {/snippet}
  </PageHeader>

  <ol class="mb-3 flex items-center gap-2 text-sm">
    <li class="flex items-center gap-1.5 {step === 1 ? 'font-semibold text-ink-900' : 'text-ink-500'}"><span class="flex h-6 w-6 items-center justify-center rounded-full text-xs {step === 1 ? 'bg-ink-900 text-white' : 'bg-ink-100'}">1</span> Pilih</li>
    <li class="text-ink-300">→</li>
    <li class="flex items-center gap-1.5 {step === 2 ? 'font-semibold text-ink-900' : 'text-ink-500'}"><span class="flex h-6 w-6 items-center justify-center rounded-full text-xs {step === 2 ? 'bg-ink-900 text-white' : 'bg-ink-100'}">2</span> Preview</li>
    <li class="text-ink-300">→</li>
    <li class="flex items-center gap-1.5 {step === 3 ? 'font-semibold text-ink-900' : 'text-ink-500'}"><span class="flex h-6 w-6 items-center justify-center rounded-full text-xs {step === 3 ? 'bg-ink-900 text-white' : 'bg-ink-100'}">3</span> Impor</li>
  </ol>

  {#if step === 1}
    <Card title="Sumber & mapping">
      <div class="grid gap-2 sm:grid-cols-2">
        <Field stacked id="pi-router" label="Router" type="select" value={routerId} options={[{ value: '', label: 'Pilih…' }, ...routers.map((r) => ({ value: r.id, label: r.name }))]} onchange={(v) => (routerId = v)} />
        <Field stacked id="pi-disabled" label="Sertakan akun nonaktif?" type="toggle" value={includeDisabled ? '1' : ''} onchange={(v) => (includeDisabled = v === '1')} />
        <Field stacked id="pi-cust" label="Pelanggan (opsional)" type="select" value={customerId} options={[{ value: '', label: 'Tanpa mapping' }, ...customers.map((c) => ({ value: c.id, label: c.name }))]} onchange={(v) => { customerId = v; void loadLocationsForCustomer(v); }} />
        <Field stacked id="pi-loc" label="Lokasi (opsional)" type="select" value={locationId} options={[{ value: '', label: 'Pilih…' }, ...locations.map((l) => ({ value: l.id, label: l.label }))]} onchange={(v) => (locationId = v)} help={customerId ? 'Wajib isi bila pelanggan dipilih.' : ''} />
      </div>
      <div class="mt-3 flex justify-end gap-2">
        <Button variant="ghost" onclick={resetPreview} disabled={loading}>Bersihkan</Button>
        <Button variant="primary" icon="search" loading={loading} onclick={() => void scan()} disabled={loading || !routerId}>Pindai router</Button>
      </div>
    </Card>
  {:else if step === 2}
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
      <StatTile label="Item" value={String(summary.total)} hint="Kandidat pindaian" />
      <StatTile label="Baru" value={String(summary.fresh)} hint="Belum ada di DB" tone="positive" />
      <StatTile label="Perbarui" value={String(summary.updates)} hint="Sudah ada, beda data" tone="warning" />
      <StatTile label="Sama" value={String(summary.same)} hint="Tidak berubah" />
    </div>
    <Card title={`Preview — ${selected.size} dipilih`}>
      <div class="mb-2 flex flex-wrap gap-2">
        <Button variant="ghost" onclick={() => toggleAll('new_update')} disabled={loading}>Pilih baru+perbarui</Button>
        <Button variant="ghost" onclick={() => toggleAll('all')} disabled={loading}>Pilih semua</Button>
        <Button variant="ghost" onclick={() => toggleAll('none')} disabled={loading}>Kosongkan</Button>
        <Button variant="ghost" icon="refresh" onclick={() => void scan()} disabled={loading}>Pindai ulang</Button>
        <Button variant="primary" icon="download" loading={loading} onclick={() => void runImport()} disabled={loading || selected.size === 0}>Impor ({selected.size})</Button>
      </div>
      <DataTable
        {columns}
        rows={candidates.map((c) => ({
          id: c.username,
          username: c.username,
          profile: c.profile_name || '—',
          remote: c.remote_address || '—',
          disabled: c.disabled,
          action: c.action,
          pw: c.password_available,
        }))}
        emptyTitle="Tidak ada kandidat"
      >
        {#snippet cell(row, col)}
          {@const cellVal = (row as unknown as Record<string, unknown>)[col.key] as string}
          {#if col.key === 'pick'}
            <input type="checkbox" checked={selected.has(row.id)} onchange={(e) => toggleOne(row.id, (e.currentTarget as HTMLInputElement).checked)} aria-label={`Pilih ${row.id}`} />
          {:else if col.key === 'disabled'}
            <Badge tone={row.disabled ? 'negative' : 'positive'} label={row.disabled ? 'Nonaktif' : 'Aktif'} />
          {:else if col.key === 'action'}
            <Badge tone={pppoeActionTone(row.action)} label={pppoeActionLabel(row.action)} />
          {:else if col.key === 'pw'}
            <Badge tone={row.pw ? 'positive' : 'warning'} label={row.pw ? 'Ada' : 'Hilang'} />
          {:else if col.key === 'profile' || col.key === 'remote'}
            <span class="font-mono text-xs">{cellVal}</span>
          {:else}
            <span class="text-sm">{cellVal}</span>
          {/if}
        {/snippet}
      </DataTable>
      <div class="mt-3">
        <Button variant="ghost" onclick={() => (step = 1)} disabled={loading}>Kembali</Button>
      </div>
    </Card>
  {:else}
    <Card title="Hasil impor">
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <StatTile label="Dibuat" value={String(result?.created ?? 0)} hint="Akun baru" tone="positive" />
        <StatTile label="Diperbarui" value={String(result?.updated ?? 0)} hint="Akun lama" />
        <StatTile label="Dilewati" value={String(result?.skipped ?? 0)} hint="Tidak berubah" />
        <StatTile label="Password hilang" value={String(result?.missing_password ?? 0)} hint="Perlu set manual" tone="warning" />
      </div>
      {#if result?.errors?.length}
        <ul class="mt-3 grid gap-1 text-sm">
          {#each result.errors as e}
            <li class="rounded-lg bg-red-50 px-3 py-2 text-red-800"><span class="font-mono">{e.username}</span>: {extractApiErrorMessage(e, '')}</li>
          {/each}
        </ul>
      {/if}
      <div class="mt-3 flex gap-2">
        <Button variant="ghost" icon="refresh" onclick={resetPreview}>Impor lagi</Button>
        <Button variant="primary" href="/v2/admin/network/pppoe">Ke daftar PPPoE</Button>
      </div>
    </Card>
  {/if}
</AppShell>
