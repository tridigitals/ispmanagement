<script lang="ts">
  /*
    hotspot order portal v2 — gelombang 25c (stub roadmap).
    Versi lama: (app)/dashboard/services/order/hotspot/+page.svelte.
    Pola DS: PortalShell + reuse ServiceComingSoonPage legacy.
  */
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = ['Profil voucher dan harga', 'Branding captive portal dan kebijakan login', 'Kebijakan batas perangkat/sesi', 'Integrasi NAS dan profil radius'];
</script>

<PortalShell title="Hotspot">
  <ServiceComingSoonPage
    title="Hotspot"
    description="Alur pesanan ini sudah disiapkan. Aktivasi layanan masih pada tahap roadmap."
    statusLabel="Segera Hadir"
    icon="radio"
    {roadmap}
    nextTarget="Setelah alur pesanan internet ISP stabil, pemesanan dan provisioning hotspot akan diaktifkan."
  />
</PortalShell>
