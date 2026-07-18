<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { api, type SupportTicketListItem } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { formatDate, timeAgo } from '$lib/utils/date';

  let tickets = $state<SupportTicketListItem[]>([]);
  let loading = $state(true);
  let error = $state('');

  // svelte-i18n returns the key itself when missing — falsy-or fallback never fires
  function tt(key: string, fallback: string) {
    const v = get(t)(key);
    return !v || v === key ? fallback : v;
  }

  function normStatus(status: string) {
    const s = String(status || '').toLowerCase();
    if (s === 'resolved' || s === 'done' || s === 'completed') return 'closed';
    if (s === 'in_progress' || s === 'waiting') return 'pending';
    return s;
  }

  let stats = $derived.by(() => {
    const tix = tickets;
    const n = tix.map((x) => normStatus(x.status));
    return {
      total: tix.length,
      open: n.filter((s) => s === 'open').length,
      pending: n.filter((s) => s === 'pending').length,
      closed: n.filter((s) => s === 'closed').length,
    };
  });

  function statusClass(status: string) {
    switch (normStatus(status)) {
      case 'open':
        return 'pill-open';
      case 'pending':
        return 'pill-pending';
      case 'closed':
        return 'pill-closed';
      default:
        return 'pill-neutral';
    }
  }

  function statusLabel(status: string) {
    const s = normStatus(status);
    if (s === 'open') return 'Open';
    if (s === 'pending') return 'Pending';
    if (s === 'closed') return 'Resolved';
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
      error = String(e?.message || e || 'Failed to load tickets');
    } finally {
      loading = false;
    }
  }
</script>

<div class="page fade-in">
  <div class="page-head">
    <div class="page-head-text">
      <h1>{tt('dashboard.tickets.title', 'Support Tickets')}</h1>
      <p class="page-sub">
        {#if loading}
          Memuat tiket...
        {:else}
          {stats.total} tiket
          {#if stats.open > 0}
            · {stats.open} open
          {/if}
          {#if stats.pending > 0}
            · {stats.pending} pending
          {/if}
        {/if}
      </p>
    </div>
    <div class="head-actions">
      <button class="btn btn-ghost" type="button" onclick={loadTickets} disabled={loading}>
        <Icon name="refresh-cw" size={14} />
        <span>Refresh</span>
      </button>
      <button class="btn btn-primary" type="button" onclick={() => goto('/support')}>
        <Icon name="plus" size={16} />
        <span>{tt('dashboard.tickets.new_ticket', 'Buat tiket')}</span>
      </button>
    </div>
  </div>

  {#if !loading && !error}
    <div class="kpis">
      <div class="kpi">
        <div class="kpi-label">Total</div>
        <div class="kpi-val">{stats.total}</div>
        <div class="kpi-sub">semua tiket</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Open</div>
        <div class="kpi-val {stats.open > 0 ? 'ok' : ''}">{stats.open}</div>
        <div class="kpi-sub">butuh respon</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Pending</div>
        <div class="kpi-val {stats.pending > 0 ? 'warn' : ''}">{stats.pending}</div>
        <div class="kpi-sub">menunggu</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">Resolved</div>
        <div class="kpi-val">{stats.closed}</div>
        <div class="kpi-sub">selesai</div>
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="panel">
      <div class="state">
        <div class="spinner"></div>
        <p>{tt('common.loading', 'Loading...')}</p>
      </div>
    </div>
  {:else if error}
    <div class="panel">
      <div class="state">
        <Icon name="alert-triangle" size={36} />
        <h3>{tt('common.error', 'Error')}</h3>
        <p>{error}</p>
        <button class="btn btn-secondary" type="button" onclick={loadTickets}>
          {tt('common.retry', 'Coba lagi')}
        </button>
      </div>
    </div>
  {:else if tickets.length === 0}
    <div class="panel">
      <div class="state">
        <Icon name="inbox" size={40} />
        <h3>{tt('dashboard.tickets.empty_title', 'Belum ada tiket')}</h3>
        <p>
          {tt(
            'dashboard.tickets.empty_subtitle',
            'Buat tiket support jika butuh bantuan teknis'
          )}
        </p>
        <button class="btn btn-primary" type="button" onclick={() => goto('/support')}>
          <Icon name="plus" size={16} />
          <span>{tt('dashboard.tickets.create_first', 'Buat tiket')}</span>
        </button>
      </div>
    </div>
  {:else}
    <div class="panel">
      <div class="list-desktop">
        <table class="data-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Subject</th>
              <th>Status</th>
              <th>Date</th>
              <th class="actions-col"></th>
            </tr>
          </thead>
          <tbody>
            {#each tickets as ticket (ticket.id)}
              <tr class="row-click" onclick={() => goto(`/support/${ticket.id}`)}>
                <td class="mono">#{ticket.id.slice(0, 8)}</td>
                <td>
                  <span class="subject">{ticket.subject}</span>
                  <span class="meta">{ticket.message_count ?? 0} pesan</span>
                </td>
                <td>
                  <span class="pill {statusClass(ticket.status)}">
                    {statusLabel(ticket.status)}
                  </span>
                </td>
                <td>
                  <span title={formatDate(ticket.created_at)}>{timeAgo(ticket.created_at)}</span>
                </td>
                <td class="actions-col">
                  <button
                    class="icon-btn"
                    type="button"
                    title="Lihat"
                    aria-label="Lihat tiket"
                    onclick={(e) => {
                      e.stopPropagation();
                      goto(`/support/${ticket.id}`);
                    }}
                  >
                    <Icon name="eye" size={16} />
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <div class="list-mobile">
        {#each tickets as ticket (ticket.id)}
          <button
            class="ticket-card"
            type="button"
            onclick={() => goto(`/support/${ticket.id}`)}
          >
            <div class="ticket-card-top">
              <span class="pill {statusClass(ticket.status)}">
                {statusLabel(ticket.status)}
              </span>
              <span class="time" title={formatDate(ticket.created_at)}
                >{timeAgo(ticket.created_at)}</span
              >
            </div>
            <div class="ticket-card-subject">{ticket.subject}</div>
            <div class="ticket-card-meta">
              <span class="mono">#{ticket.id.slice(0, 8)}</span>
              <span>·</span>
              <span>{ticket.message_count ?? 0} pesan</span>
            </div>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .page {
    padding: clamp(1rem, 2.2vw, 1.75rem);
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .page-head h1 {
    font-size: clamp(1.25rem, 2.2vw, 1.45rem);
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
    color: var(--text-primary);
  }
  .page-sub {
    color: var(--text-secondary);
    font-size: 0.88rem;
    margin: 0.25rem 0 0;
  }
  .head-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .kpis {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.7rem;
  }
  .kpi {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    padding: 0.9rem 1rem;
  }
  .kpi-label {
    font-size: 0.7rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    margin-bottom: 0.35rem;
  }
  .kpi-val {
    font-size: 1.25rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }
  .kpi-val.ok {
    color: var(--color-success);
  }
  .kpi-val.warn {
    color: var(--color-warning);
  }
  .kpi-sub {
    font-size: 0.74rem;
    color: var(--text-secondary);
    margin-top: 0.2rem;
  }

  .panel {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: var(--radius-lg, 12px);
    overflow: hidden;
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    min-height: 260px;
    gap: 0.5rem;
    padding: 2rem 1.25rem;
    color: var(--text-secondary);
  }
  .state h3 {
    margin: 0.5rem 0 0;
    color: var(--text-primary);
    font-size: 1.05rem;
  }
  .state p {
    margin: 0 0 0.75rem;
    font-size: 0.88rem;
    max-width: 320px;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.08);
    border-top-color: var(--color-primary, #8b9cff);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
  }
  .data-table th {
    text-align: left;
    font-size: 0.72rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    padding: 0.85rem 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.02);
  }
  .data-table td {
    padding: 0.9rem 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
    font-size: 0.9rem;
    vertical-align: middle;
  }
  .row-click {
    cursor: pointer;
  }
  .row-click:hover td {
    background: rgba(255, 255, 255, 0.02);
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }
  .subject {
    display: block;
    font-weight: 600;
    color: var(--text-primary);
  }
  .meta {
    display: block;
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-top: 0.15rem;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .pill-open {
    background: color-mix(in srgb, var(--color-success) 16%, transparent);
    color: var(--color-success);
  }
  .pill-pending {
    background: color-mix(in srgb, var(--color-warning) 16%, transparent);
    color: var(--color-warning);
  }
  .pill-closed,
  .pill-neutral {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
  }

  .actions-col {
    text-align: right;
    width: 64px;
  }
  .icon-btn {
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 8px;
  }
  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
  }

  .list-mobile {
    display: none;
    flex-direction: column;
  }
  .ticket-card {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    width: 100%;
    text-align: left;
    padding: 0.95rem 1rem;
    border: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    background: transparent;
    color: inherit;
    cursor: pointer;
    min-height: 72px;
  }
  .ticket-card:hover {
    background: rgba(255, 255, 255, 0.02);
  }
  .ticket-card-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }
  .ticket-card-subject {
    font-weight: 650;
    color: var(--text-primary);
    font-size: 0.95rem;
    line-height: 1.35;
  }
  .ticket-card-meta {
    display: flex;
    gap: 0.4rem;
    font-size: 0.78rem;
    color: var(--text-secondary);
  }
  .time {
    font-size: 0.78rem;
    color: var(--text-secondary);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.55rem 0.95rem;
    border-radius: 8px;
    font-weight: 650;
    font-size: 0.88rem;
    cursor: pointer;
    border: none;
    min-height: 40px;
  }
  .btn-primary {
    background: var(--color-primary);
    color: #fff;
  }
  .btn-secondary {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .btn-ghost:hover:not(:disabled) {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  @media (max-width: 900px) {
    .kpis {
      grid-template-columns: 1fr 1fr;
    }
  }
  @media (max-width: 640px) {
    .list-desktop {
      display: none;
    }
    .list-mobile {
      display: flex;
    }
  }
  @media (max-width: 560px) {
    .page-head {
      align-items: stretch;
    }
    .head-actions {
      width: 100%;
    }
    .head-actions .btn {
      flex: 1;
      min-height: 44px;
    }
    .kpis {
      gap: 0.55rem;
    }
    .kpi {
      padding: 0.75rem 0.85rem;
    }
    .kpi-val {
      font-size: 1.1rem;
    }
  }
</style>
