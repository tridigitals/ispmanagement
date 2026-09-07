<script lang="ts">
  /*
    Katalog pesanan layanan portal v2 — gelombang 25c.
    Versi lama: (app)/dashboard/services/order/+page.svelte (338 baris).
    Perilaku identik: hero + kartu layanan (internet, hotspot, dedicated, vpn).
    Pola DS: PortalShell + PageHeader + Card + Badge + Button.
  */
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';

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
      dedicatedLinkEnabled = false;
    } catch (e: any) {
      loadError = e?.message || String(e);
    } finally {
      loading = false;
    }
  }
</script>

<PortalShell title="Pesan Layanan">
  <PageHeader
    title="Pesan Layanan Baru"
    desc="Pilih jenis layanan untuk memulai pesanan instalasi."
  >
    {#snippet actions()}
      <Button variant="ghost" icon="chevronLeft" onclick={() => goto('/v2/dashboard/services')}>
        Ke Layanan
      </Button>
      <Button variant="ghost" icon="receipt" onclick={() => goto('/v2/dashboard/invoices')}>
        Tagihan
      </Button>
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={loadSummary}>Segarkan</Button>
    {/snippet}
  </PageHeader>

  {#if loadError}
    <div class="banner-bad">
      <span>{loadError}</span>
      <Button variant="ghost" size="sm" onclick={loadSummary}>Coba lagi</Button>
    </div>
  {/if}

  <div class="grid gap-4 md:grid-cols-2">
    <Card title="Internet Fiber / PPPoE" class="service-card">
      {#snippet aside()}
        <Badge tone={internetPackageCount > 0 ? 'positive' : 'neutral'} label={internetPackageCount > 0 ? 'Tersedia' : 'Segera'} />
      {/snippet}
      <p class="mb-4 text-sm text-ink-500">
        Alur instalasi internet fiber/kabel dengan pemilihan alamat dan paket.
      </p>
      <div class="mb-4 flex flex-wrap gap-2 text-xs text-ink-500">
        <span class="chip-meta">{internetPackageCount} paket</span>
        <span class="chip-meta">{locationsCount} lokasi</span>
      </div>
      <Button
        icon="chevronRight"
        onclick={() => goto('/v2/dashboard/services/order/internet')}
        disabled={loading || internetPackageCount === 0}
      >
        Mulai Pesanan Internet
      </Button>
    </Card>

    <Card title="Hotspot" class="service-card">
      {#snippet aside()}
        <Badge tone={hotspotPackageCount > 0 ? 'positive' : 'neutral'} label={hotspotPackageCount > 0 ? 'Tersedia' : 'Segera'} />
      {/snippet}
      <p class="mb-4 text-sm text-ink-500">Layanan hotspot / akses publik berbasis voucher.</p>
      <div class="mb-4 flex flex-wrap gap-2 text-xs text-ink-500">
        <span class="chip-meta">{hotspotPackageCount} paket</span>
      </div>
      <Button variant="secondary" onclick={() => goto('/v2/dashboard/services/order/hotspot')}>
        {hotspotPackageCount > 0 ? 'Buka Alur Pesanan' : 'Buka Halaman Layanan'}
      </Button>
    </Card>

    <Card title="Dedicated Link" class="service-card">
      {#snippet aside()}
        <Badge tone={dedicatedLinkEnabled ? 'positive' : 'neutral'} label={dedicatedLinkEnabled ? 'Tersedia' : 'Segera'} />
      {/snippet}
      <p class="mb-4 text-sm text-ink-500">Sambungan khusus berkecepatan tinggi untuk bisnis.</p>
      <div class="mb-4 flex flex-wrap gap-2 text-xs text-ink-500">
        <span class="chip-meta">Belum diaktifkan</span>
      </div>
      <Button variant="secondary" onclick={() => goto('/v2/dashboard/services/order/dedicated-link')}>
        {dedicatedLinkEnabled ? 'Buka Alur Pesanan' : 'Buka Halaman Layanan'}
      </Button>
    </Card>

    <Card title="Managed VPN" class="service-card">
      {#snippet aside()}
        <Badge tone={vpnPackageCount > 0 ? 'positive' : 'neutral'} label={vpnPackageCount > 0 ? 'Tersedia' : 'Segera'} />
      {/snippet}
      <p class="mb-4 text-sm text-ink-500">Akses aman antar-kantor melalui jaringan terenkripsi.</p>
      <div class="mb-4 flex flex-wrap gap-2 text-xs text-ink-500">
        <span class="chip-meta">{vpnPackageCount} paket</span>
      </div>
      <Button variant="secondary" onclick={() => goto('/v2/dashboard/services/order/vpn')}>
        {vpnPackageCount > 0 ? 'Buka Alur Pesanan' : 'Buka Halaman Layanan'}
      </Button>
    </Card>
  </div>
</PortalShell>

<style>
  .banner-bad {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    border-radius: 0.75rem;
    border: 1px solid #fecaca;
    background: #fef2f2;
    color: #991b1b;
    padding: 0.7rem 1rem;
    margin-bottom: 1rem;
    font-size: 0.875rem;
  }
  .chip-meta {
    border-radius: 9999px;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fafaf9;
    padding: 0.15rem 0.6rem;
    font-size: 0.7rem;
  }
</style>
