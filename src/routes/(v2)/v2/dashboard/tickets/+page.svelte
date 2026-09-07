<script lang="ts">
  /*
    Tiket portal v2 — gelombang 25b.
    Versi lama: (app)/dashboard/tickets/+page.svelte (565 baris).
    Perilaku identik: KPI + tabel tiket (klik → detail /v2/support/[id]).
    Pola DS: PortalShell + PageHeader + StatTile + DataTable + Badge.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api, type SupportTicketListItem } from '$lib/api/client';
  import { formatDate, timeAgo } from '$lib/utils/date';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';
  import type { Column } from '$lib/components/ds/table-types';
  import type { StatusTone } from '$lib/components/ds/tokens';

  let tickets = $state<SupportTicketListItem[]>([]);
  let loading = $state(true);
  let error = $state('');

  function normStatus(status: string) {
    const s = String(status || '').toLowerCase();
    if (s === 'resolved' || s === 'done' || s === 'completed') return 'closed';
    if (s === 'in_progress' || s === 'waiting') return 'pending';
    return s;
  }

  const stats = $derived.by(() => {
    const n = tickets.map((x) => normStatus(x.status));
    return {
      total: tickets.length,
      open: n.filter((s) => s === 'open').length,
      pending: n.filter((s) => s === 'pending').length,
      closed: n.filter((s) => s === 'closed').length,
    };
  });

  function statusTone(status: string): StatusTone {
    const s = normStatus(status);
    if (s === 'open') return 'info';
    if (s === 'pending') return 'warning';
    if (s === 'closed') return 'positive';
    return 'neutral';
  }
  function statusLabel(status: string) {
    const s = normStatus(status);
    if (s === 'open') return 'Terbuka';
    if (s === 'pending') return 'Menunggu';
    if (s === 'closed') return 'Selesai';
    return status || '—';
  }

  onMount(() => {
    loadTickets();
  });

  async function loadTickets() {
    loading = true;
    error = '';
    try {
      const res = await api.support.list({ perPage: 50 });
      tickets = res.data ?? [];
    } catch (e: any) {
      error = String(e?.message || e || 'Gagal memuat tiket');
    } finally {
      loading = false;
    }
  }
</script>

<PortalShell title="Tiket">
  <PageHeader
    title="Tiket Dukungan"
    desc={loading
      ? 'Memuat tiket…'
      : `${stats.total} tiket` +
        (stats.open > 0 ? ` · ${stats.open} terbuka` : '') +
        (stats.pending > 0 ? ` · ${stats.pending} menunggu` : '')}
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={loadTickets}>Segarkan</Button>
      <Button icon="plus" onclick={() => goto('/v2/support')}>Buat tiket</Button>
    {/snippet}
  </PageHeader>

  {#if !loading && !error}
    <div class="mb-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
      <StatTile label="Total" value={String(stats.total)} hint="semua tiket" />
      <StatTile label="Terbuka" value={String(stats.open)} hint="butuh respon" tone={stats.open > 0 ? 'positive' : 'neutral'} />
      <StatTile label="Menunggu" value={String(stats.pending)} hint="diproses" tone={stats.pending > 0 ? 'warning' : 'neutral'} />
      <StatTile label="Selesai" value={String(stats.closed)} hint="tuntas" />
    </div>
  {/if}

  {#if loading}
    <Card title="Memuat…"><p class="text-sm text-ink-500">Mengambil tiket…</p></Card>
  {:else if error}
    <Card title="Gagal memuat">
      <p class="mb-4 text-sm text-ink-500">{error}</p>
      <Button variant="secondary" onclick={loadTickets}>Coba lagi</Button>
    </Card>
  {:else if tickets.length === 0}
    <Card title="Belum ada tiket">
      <p class="mb-4 text-sm text-ink-500">Buat tiket support jika butuh bantuan teknis.</p>
      <Button icon="plus" onclick={() => goto('/v2/support')}>Buat tiket</Button>
    </Card>
  {:else}
    <Card title="Daftar tiket" padded={false}>
      <DataTable
        rows={tickets}
        columns={[
          { key: 'id', label: 'ID' },
          { key: 'subject', label: 'Judul' },
          { key: 'status', label: 'Status' },
          { key: 'date', label: 'Dibuat' },
          { key: 'actions', label: '', align: 'right' },
        ]}
        emptyTitle="Belum ada tiket"
      >
        {#snippet cell(row: SupportTicketListItem, col: Column)}
          {#if col.key === 'id'}
            <span class="font-mono text-xs">{`#${row.id.slice(0, 8)}`}</span>
          {:else if col.key === 'subject'}
            <span class="block">
              <span class="block truncate font-medium">{row.subject}</span>
              <span class="text-xs text-ink-400">{row.message_count ?? 0} pesan</span>
            </span>
          {:else if col.key === 'status'}
            <Badge tone={statusTone(row.status)} label={statusLabel(row.status)} />
          {:else if col.key === 'date'}
            <span title={formatDate(row.created_at)}>{timeAgo(row.created_at)}</span>
          {:else if col.key === 'actions'}
            <Button
              variant="ghost"
              size="sm"
              icon="chevronRight"
              onclick={() => goto(`/v2/support/${row.id}`)}
            >
              Lihat
            </Button>
          {/if}
        {/snippet}
      </DataTable>
    </Card>
  {/if}
</PortalShell>
