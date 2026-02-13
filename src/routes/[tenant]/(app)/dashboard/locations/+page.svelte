<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { api, type CustomerLocation } from '$lib/api/client';

  import Icon from '$lib/components/ui/Icon.svelte';

  let loading = $state(true);
  let locations = $state<CustomerLocation[]>([]);
  let error = $state('');

  onMount(async () => {
    if (!$can('read_own', 'customers')) {
      goto('/unauthorized');
      return;
    }
    await load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      locations = await api.customers.portal.myLocations();
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to load locations');
      toast.error(get(t)('dashboard.locations.toasts.load_failed') || 'Failed to load locations');
    } finally {
      loading = false;
    }
  }

  function formatAddress(loc: CustomerLocation) {
    const line1 = loc.address_line1 || '';
    const parts = [loc.city, loc.state, loc.postal_code, loc.country].filter(Boolean).join(', ');
    return [line1, parts].filter(Boolean).join(' • ');
  }
</script>

<div class="page-content fade-in">
  <div class="page-header">
    <div>
      <div class="kicker">
        <span class="dot"></span>
        {$t('dashboard.locations.kicker') || 'Customer portal'}
      </div>
      <h1>{$t('dashboard.locations.title') || 'My Locations'}</h1>
      <p class="subtitle">
        {$t('dashboard.locations.subtitle') ||
          'Your service locations. If something looks wrong, contact support.'}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn-secondary" onclick={load} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="error-banner">
      <Icon name="alert-triangle" size={18} />
      <span>{error}</span>
    </div>
  {/if}

  {#if loading}
    <div class="loading-card card">
      <div class="spinner"></div>
      <p>{$t('common.loading') || 'Loading...'}</p>
    </div>
  {:else if locations.length === 0}
    <div class="empty card">
      <Icon name="map-pin" size={28} />
      <div class="empty-text">
        <div class="title">{$t('dashboard.locations.empty') || 'No locations yet.'}</div>
        <div class="sub">
          {$t('dashboard.locations.empty_hint') ||
            'Your admin has not linked your account to a customer location.'}
        </div>
      </div>
    </div>
  {:else}
    <div class="grid">
      {#each locations as loc (loc.id)}
        <div class="location card">
          <div class="top">
            <div class="badge">
              <Icon name="map-pin" size={16} />
              <span>{$t('dashboard.locations.location') || 'Location'}</span>
            </div>
          </div>
          <div class="name">{loc.label}</div>
          <div class="addr">{formatAddress(loc) || '—'}</div>
          {#if loc.notes}
            <div class="notes">{loc.notes}</div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.35rem;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: rgba(99, 102, 241, 0.9);
    box-shadow: 0 0 0 6px rgba(99, 102, 241, 0.12);
  }

  .subtitle {
    color: var(--text-secondary);
    margin-top: 0.35rem;
  }

  .header-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .error-banner {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.75rem 0.9rem;
    border-radius: 12px;
    border: 1px solid rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.08);
    color: var(--text-primary);
    margin-bottom: 0.75rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
  }

  .location {
    padding: 1.15rem;
    position: relative;
    overflow: hidden;
  }

  .location::before {
    content: '';
    position: absolute;
    inset: -1px;
    background:
      radial-gradient(800px 240px at 0% 0%, rgba(99, 102, 241, 0.18), transparent 55%),
      radial-gradient(900px 260px at 100% 0%, rgba(34, 197, 94, 0.12), transparent 58%);
    pointer-events: none;
  }

  .top {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.9rem;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem;
    border-radius: 999px;
    border: 1px solid rgba(148, 163, 184, 0.35);
    background: rgba(148, 163, 184, 0.08);
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 650;
  }

  .name {
    position: relative;
    font-size: 1.1rem;
    font-weight: 750;
    margin-bottom: 0.35rem;
  }

  .addr {
    position: relative;
    color: var(--text-secondary);
    line-height: 1.4;
    font-size: 0.95rem;
  }

  .notes {
    position: relative;
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
    color: var(--text-secondary);
    font-size: 0.9rem;
    white-space: pre-wrap;
  }

  .empty {
    padding: 1.2rem;
    display: flex;
    gap: 0.9rem;
    align-items: flex-start;
  }

  .empty-text .title {
    font-weight: 750;
    margin-bottom: 0.25rem;
  }

  .empty-text .sub {
    color: var(--text-secondary);
  }

  .loading-card {
    padding: 1.25rem;
    display: grid;
    place-items: center;
    gap: 0.5rem;
  }

  .spinner {
    width: 26px;
    height: 26px;
    border-radius: 999px;
    border: 3px solid rgba(148, 163, 184, 0.3);
    border-top-color: rgba(99, 102, 241, 0.9);
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 980px) {
    .grid {
      grid-template-columns: 1fr;
    }
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: stretch;
    }
  }
</style>
