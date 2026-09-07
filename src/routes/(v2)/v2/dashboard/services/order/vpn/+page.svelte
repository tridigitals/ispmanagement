<script lang="ts">
  /*
    vpn order portal v2 — gelombang 25c (stub roadmap).
    Versi lama: (app)/dashboard/services/order/vpn/+page.svelte.
    Pola DS: PortalShell + reuse ServiceComingSoonPage legacy.
  */
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = ['Mode site-to-site dan remote access', 'Kebijakan user/group dan profil rute', 'Manajemen siklus hidup sertifikat dan kunci', 'Monitoring dan analitik sesi'];
</script>

<PortalShell title="Managed VPN">
  <ServiceComingSoonPage
    title="Managed VPN"
    description="Alur pemesanan dan provisioning VPN akan diaktifkan setelah penguatan modul kebijakan dan keamanan."
    statusLabel="Segera Hadir"
    icon="shield"
    {roadmap}
    nextTarget="Layanan VPN akan dibuka dengan template keamanan dan dukungan logging kepatuhan."
  />
</PortalShell>
