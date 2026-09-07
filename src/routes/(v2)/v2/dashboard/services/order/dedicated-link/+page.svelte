<script lang="ts">
  /*
    dedicated-link order portal v2 — gelombang 25c (stub roadmap).
    Versi lama: (app)/dashboard/services/order/dedicated-link/+page.svelte.
    Pola DS: PortalShell + reuse ServiceComingSoonPage legacy.
  */
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = ['Pemetaan endpoint multi-site', 'Kelas SLA dan bandwidth terkomitmen', 'Path redundan dan kebijakan failover', 'Checklist onboarding enterprise'];
</script>

<PortalShell title="Dedicated Link">
  <ServiceComingSoonPage
    title="Dedicated Link"
    description="Alur pesanan link enterprise khusus disisihkan dan akan diaktifkan bersama dukungan provisioning SLA."
    statusLabel="Segera Hadir"
    icon="router"
    {roadmap}
    nextTarget="Layanan akan terbuka setelah baseline hotspot dan modul provisioning enterprise siap."
  />
</PortalShell>
