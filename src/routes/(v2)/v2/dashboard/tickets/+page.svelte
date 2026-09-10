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

  import { t } from 'svelte-i18n';
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
    if (s === 'open') return $t('dashboard.tickets_v2.st_open');
    if (s === 'pending') return $t('dashboard.tickets_v2.st_pending');
    if (s === 'closed') return $t('dashboard.tickets_v2.st_resolved');
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

<PortalShell title={ $t('dashboard.tickets_v2.col_ticket') }>
  <PageHeader
    title={ $t('dashboard.tickets_v2.title') }
    desc={loading
      ? $t('dashboard.tickets_v2.loading')
      : `${stats.total} tiket` +
        (stats.open > 0 ? ` · ${stats.open} terbuka` : '') +
        (stats.pending > 0 ? ` · ${stats.pending} menunggu` : '')}
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={loadTickets}>{ $t('common.refresh') }</Button>
      <Button icon="plus" onclick={() => goto('/v2/support')}>{ $t('support.v2.new_btn') }</Button>
    {/snippet}
  </PageHeader>

  {#if !loading && !error}
    <div class="mb-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
      <StatTile label={ $t('dashboard.tickets_v2.st_total') } value={String(stats.total)} hint={ $t('dashboard.tickets_v2.h_all') } />
      <StatTile label={ $t('dashboard.tickets_v2.st_open') } value={String(stats.open)} hint={ $t('dashboard.tickets_v2.h_need_resp') } tone={stats.open > 0 ? 'positive' : 'neutral'} />
      <StatTile label={ $t('dashboard.tickets_v2.st_pending') } value={String(stats.pending)} hint={ $t('dashboard.tickets_v2.h_processing') } tone={stats.pending > 0 ? 'warning' : 'neutral'} />
      <StatTile label={ $t('dashboard.tickets_v2.st_resolved') } value={String(stats.closed)} hint={ $t('dashboard.tickets_v2.h_done') } />
    </div>
  {/if}

  {#if loading}
    <Card title={ $t('common.loading') }><p class="text-sm text-ink-500">{ $t('dashboard.tickets_v2.fetching') }</p></Card>
  {:else if error}
    <Card title={ $t('dashboard.tickets_v2.load_fail') }>
      <p class="mb-4 text-sm text-ink-500">{error}</p>
      <Button variant="secondary" onclick={loadTickets}>{ $t('common.retry') }</Button>
    </Card>
  {:else if tickets.length === 0}
    <Card title={ $t('dashboard.tickets_v2.empty') }>
      <p class="mb-4 text-sm text-ink-500">{ $t('dashboard.tickets_v2.empty_hint') }</p>
      <Button icon="plus" onclick={() => goto('/v2/support')}>{ $t('support.v2.new_btn') }</Button>
    </Card>
  {:else}
    <Card title={ $t('dashboard.tickets_v2.list') } padded={false}>
      <DataTable
        rows={tickets}
        pageSize={25}
        columns={[
          { key: 'id', label: 'ID' },
          { key: 'subject', label: $t('dashboard.tickets_v2.col_title') },
          { key: 'status', label: $t('dashboard.tickets_v2.col_status') },
          { key: 'date', label: $t('dashboard.tickets_v2.col_created') },
          { key: 'actions', label: '', align: 'right' },
        ]}
        emptyTitle={ $t('dashboard.tickets_v2.empty') }
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
