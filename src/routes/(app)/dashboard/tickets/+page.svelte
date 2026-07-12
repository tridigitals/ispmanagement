<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { api, type SupportTicketListItem } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { formatDate, timeAgo } from '$lib/utils/date';

  let tickets = $state<SupportTicketListItem[]>([]);
  let loading = $state(true);
  let error = $state('');

  const statusPill = (status: string) => {
    switch (status) {
      case 'open': return 'pill-success';
      case 'pending': return 'pill-warning';
      case 'closed': return 'pill-neutral';
      default: return 'pill-info';
    }
  };

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

<div class="page-container fade-in">
  <div class="page-header">
    <div class="header-content">
      <div class="kicker">
        <span class="kicker-dot"></span>
        {$t('dashboard.tickets.kicker') || 'Support'}
      </div>
      <h1>{$t('dashboard.tickets.title') || 'Support Tickets'}</h1>
      <p>{$t('dashboard.tickets.subtitle') || 'View and manage your support tickets'}</p>
    </div>
    <button class="btn btn-primary btn-sm" onclick={() => goto('/support')}>
      <Icon name="plus" size={16} />
      <span>{$t('dashboard.tickets.new_ticket') || 'New Ticket'}</span>
    </button>
  </div>

  {#if loading}
    <div class="card">
      <div class="loading-state">
        <div class="spinner"></div>
        <p class="muted">{$t('common.loading') || 'Loading...'}</p>
      </div>
    </div>
  {:else if error}
    <div class="card">
      <div class="empty-state">
        <Icon name="alert-triangle" size={40} />
        <h3>{$t('common.error') || 'Error'}</h3>
        <p>{error}</p>
        <button class="btn btn-secondary" onclick={loadTickets}>
          {$t('common.retry') || 'Retry'}
        </button>
      </div>
    </div>
  {:else if tickets.length === 0}
    <div class="card">
      <div class="empty-state">
        <Icon name="inbox" size={40} />
        <h3>{$t('dashboard.tickets.empty_title') || 'No tickets yet'}</h3>
        <p>{$t('dashboard.tickets.empty_subtitle') || 'Create your first support ticket to get help'}</p>
        <button class="btn btn-primary" onclick={() => goto('/support')}>
          <Icon name="plus" size={16} />
          <span>{$t('dashboard.tickets.create_first') || 'Create Ticket'}</span>
        </button>
      </div>
    </div>
  {:else}
    <div class="card">
      <table class="data-table">
        <thead>
          <tr>
            <th>{$t('dashboard.tickets.id') || 'ID'}</th>
            <th>{$t('dashboard.tickets.subject') || 'Subject'}</th>
            <th>{$t('dashboard.tickets.status') || 'Status'}</th>
            <th>{$t('dashboard.tickets.date') || 'Date'}</th>
            <th class="actions-col">{$t('dashboard.tickets.actions') || 'Actions'}</th>
          </tr>
        </thead>
        <tbody>
          {#each tickets as ticket (ticket.id)}
            <tr>
              <td class="font-mono">#{ticket.id.slice(0, 8)}</td>
              <td>
                <a href="/support/{ticket.id}" class="ticket-link">{ticket.subject}</a>
                <span class="ticket-meta">{ticket.message_count ?? 0} {$t('dashboard.tickets.messages') || 'messages'}</span>
              </td>
              <td>
                <span class="pill {statusPill(ticket.status)}">{$t('support.status.' + ticket.status) || ticket.status}</span>
              </td>
              <td>
                <span title={formatDate(ticket.created_at)}>{timeAgo(ticket.created_at)}</span>
              </td>
              <td class="table-actions">
                <button
                  class="table-action"
                  title={$t('dashboard.tickets.view') || 'View'}
                  aria-label={$t('dashboard.tickets.view') || 'View ticket'}
                  onclick={() => goto(`/support/${ticket.id}`)}
                >
                  <Icon name="eye" size={16} />
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .page-container {
    padding: clamp(1rem, 3vw, 2rem);
    max-width: 1100px;
    margin: 0 auto;
  }

  .header-content h1 {
    font-size: 1.8rem;
    font-weight: 700;
    margin: 0 0 0.25rem;
  }

  .header-content p {
    color: var(--text-secondary);
    margin: 0;
  }

  /* ponytail: global .page-header covers most layout; minimal overrides here */
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1.5rem;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .card {
    background: var(--bg-surface, #11141c);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
  }

  /* ---- scoped state components (not in global.css) ---- */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 300px;
    gap: 0.75rem;
    padding: 2rem;
  }

  .spinner {
    width: 36px;
    height: 36px;
    border: 3px solid var(--border-color);
    border-top-color: var(--color-primary, #8b9cff);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .muted {
    color: var(--text-secondary);
    margin: 0;
  }

  .empty-state {
    text-align: center;
    padding: 3rem 2rem;
    max-width: 360px;
    margin: 0 auto;
    color: var(--text-secondary);
  }

  .empty-state h3 {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 1rem 0 0.5rem;
    color: var(--text-primary);
  }

  .empty-state p {
    margin: 0 0 1.25rem;
    font-size: 0.9rem;
    line-height: 1.5;
  }

  /* ---- ticket-specific ---- */
  .ticket-link {
    color: var(--color-primary, #8b9cff);
    text-decoration: none;
    font-weight: 500;
  }

  .ticket-link:hover {
    text-decoration: underline;
  }

  .ticket-meta {
    display: block;
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-top: 0.15rem;
  }

  .actions-col {
    text-align: right;
    width: 80px;
  }

  @media (max-width: 768px) {
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }

    .header-content h1 {
      font-size: 1.35rem;
    }

    .card {
      border-radius: var(--radius-lg, 12px);
    }
  }
</style>
