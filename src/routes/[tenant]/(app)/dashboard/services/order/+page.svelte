<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { api } from '$lib/api/client';

  let loading = $state(true);
  let loadError = $state('');
  let locationsCount = $state(0);
  let internetPackageCount = $state(0);
  let hotspotPackageCount = $state(0);
  let vpnPackageCount = $state(0);
  let dedicatedLinkEnabled = $state(false);

  onMount(() => {
    void loadSummary();
  });

  async function loadSummary() {
    loading = true;
    loadError = '';
    try {
      const [myLocations, myPackages] = await Promise.all([
        api.customers.portal.myLocations(),
        api.customers.portal.myPackages(),
      ]);
      const activePackages = (myPackages || []).filter((pkg) => pkg.is_active);
      locationsCount = myLocations?.length || 0;
      internetPackageCount = activePackages.filter((pkg) => pkg.service_type === 'internet_pppoe').length;
      hotspotPackageCount = activePackages.filter((pkg) => pkg.service_type === 'hotspot').length;
      vpnPackageCount = activePackages.filter((pkg) => pkg.service_type === 'vpn').length;
      // Backend doesn't expose dedicated-link package type yet.
      dedicatedLinkEnabled = false;
    } catch (e: any) {
      loadError = e?.message || String(e);
    } finally {
      loading = false;
    }
  }

</script>

<div class="service-order-page fade-in">
  <section class="hero card">
    <div>
      <h1>Order Service</h1>
      <p>Choose service type first. Each type has its own dedicated order flow.</p>
      <div class="hero-meta">
        <span>{locationsCount} location</span>
        <span>{internetPackageCount + hotspotPackageCount + vpnPackageCount} active package</span>
      </div>
    </div>
    <div class="hero-actions">
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/services')}>
        <Icon name="arrow-left" size={15} />
        Back to Services
      </button>
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/invoices')}>
        <Icon name="file-text" size={15} />
        Billing & Invoices
      </button>
      <button class="btn btn-secondary" type="button" onclick={loadSummary} disabled={loading}>
        <Icon name="refresh-cw" size={15} />
        Refresh
      </button>
    </div>
  </section>

  {#if loadError}
    <section class="alert alert-error">{loadError}</section>
  {/if}

  <section class="service-grid">
    <article class="service-card live">
      <div class="service-head">
        <span class="service-icon"><Icon name="wifi" size={18} /></span>
        <span class="service-status live">Ready</span>
      </div>
      <h3>Internet Access (PPPoE)</h3>
      <p>Fiber/cable internet installation flow with address and package selection.</p>
      <div class="service-meta">
        <span>{internetPackageCount} package</span>
        <span>{locationsCount} location</span>
      </div>
      <button
        class="btn btn-primary"
        type="button"
        onclick={() => goto('/dashboard/services/order/internet')}
        disabled={loading || internetPackageCount === 0}
      >
        <Icon name="arrow-right" size={15} />
        Start Internet Order
      </button>
    </article>

    <article class="service-card">
      <div class="service-head">
        <span class="service-icon"><Icon name="radio" size={18} /></span>
        <span class={`service-status ${hotspotPackageCount > 0 ? 'live' : 'soon'}`}>
          {hotspotPackageCount > 0 ? 'Ready' : 'Soon'}
        </span>
      </div>
      <h3>Hotspot Service</h3>
      <p>Voucher-based hotspot and captive portal deployment.</p>
      <div class="service-meta">
        <span>{hotspotPackageCount} package</span>
      </div>
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/services/order/hotspot')}>
        {hotspotPackageCount > 0 ? 'Open Order Flow' : 'Open Service Page'}
      </button>
    </article>

    <article class="service-card">
      <div class="service-head">
        <span class="service-icon"><Icon name="router" size={18} /></span>
        <span class={`service-status ${dedicatedLinkEnabled ? 'live' : 'soon'}`}>
          {dedicatedLinkEnabled ? 'Ready' : 'Soon'}
        </span>
      </div>
      <h3>Dedicated Link</h3>
      <p>Point-to-point business connectivity with SLA profile.</p>
      <div class="service-meta">
        <span>Not enabled yet</span>
      </div>
      <button
        class="btn btn-secondary"
        type="button"
        onclick={() => goto('/dashboard/services/order/dedicated-link')}
      >
        {dedicatedLinkEnabled ? 'Open Order Flow' : 'Open Service Page'}
      </button>
    </article>

    <article class="service-card">
      <div class="service-head">
        <span class="service-icon"><Icon name="shield" size={18} /></span>
        <span class={`service-status ${vpnPackageCount > 0 ? 'live' : 'soon'}`}>
          {vpnPackageCount > 0 ? 'Ready' : 'Soon'}
        </span>
      </div>
      <h3>Managed VPN</h3>
      <p>Private encrypted access for branch and remote users.</p>
      <div class="service-meta">
        <span>{vpnPackageCount} package</span>
      </div>
      <button class="btn btn-secondary" type="button" onclick={() => goto('/dashboard/services/order/vpn')}>
        {vpnPackageCount > 0 ? 'Open Order Flow' : 'Open Service Page'}
      </button>
    </article>
  </section>
</div>

<style>
  .service-order-page {
    max-width: 1240px;
    margin: 0 auto;
    padding: clamp(1rem, 2.2vw, 1.8rem);
    display: grid;
    gap: 0.9rem;
  }

  .card {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: var(--bg-surface);
    padding: 1rem;
  }

  .hero {
    display: flex;
    justify-content: space-between;
    gap: 0.9rem;
    flex-wrap: wrap;
  }

  .hero h1 {
    margin: 0;
    font-size: clamp(1.3rem, 2.3vw, 1.7rem);
  }

  .hero p {
    margin: 0.3rem 0 0;
    color: var(--text-secondary);
  }

  .hero-meta {
    margin-top: 0.65rem;
    display: inline-flex;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .hero-meta span {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.73rem;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-secondary) 78%, transparent);
  }

  .hero-actions {
    display: inline-flex;
    gap: 0.45rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .service-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 0.75rem;
  }

  .service-card {
    border: 1px solid var(--border-color);
    border-radius: 13px;
    background: color-mix(in srgb, var(--bg-secondary) 80%, transparent);
    padding: 0.9rem;
    display: grid;
    gap: 0.6rem;
    align-content: start;
  }

  .service-card.live {
    border-color: color-mix(in srgb, var(--accent-primary) 46%, var(--border-color));
    background:
      linear-gradient(
        150deg,
        color-mix(in srgb, var(--accent-primary) 12%, transparent) 0%,
        transparent 62%
      ),
      color-mix(in srgb, var(--bg-secondary) 78%, transparent);
  }

  .service-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
  }

  .service-icon {
    width: 32px;
    height: 32px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .service-status {
    border-radius: 999px;
    border: 1px solid var(--border-color);
    padding: 0.16rem 0.5rem;
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .service-status.live {
    border-color: color-mix(in srgb, #22c55e 52%, var(--border-color));
    color: #22c55e;
    background: color-mix(in srgb, #22c55e 14%, transparent);
  }

  .service-status.soon {
    border-color: color-mix(in srgb, #f59e0b 48%, var(--border-color));
    color: #fbbf24;
    background: color-mix(in srgb, #f59e0b 14%, transparent);
  }

  .service-card h3 {
    margin: 0;
    font-size: 1rem;
  }

  .service-card p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.83rem;
    line-height: 1.45;
  }

  .service-meta {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.1rem;
  }

  .service-meta span {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.15rem 0.48rem;
    font-size: 0.7rem;
    color: var(--text-secondary);
  }

  .alert {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 0.65rem 0.75rem;
    font-size: 0.85rem;
  }

  .alert-error {
    color: #fca5a5;
    border-color: color-mix(in srgb, #ef4444 40%, var(--border-color));
    background: color-mix(in srgb, #ef4444 8%, transparent);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    border-radius: 10px;
    border: 1px solid transparent;
    padding: 0.58rem 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-secondary);
    border-color: var(--border-color);
    color: var(--text-primary);
  }

  .btn-primary {
    background: var(--accent-primary);
    color: var(--text-on-primary, #fff);
  }
</style>
