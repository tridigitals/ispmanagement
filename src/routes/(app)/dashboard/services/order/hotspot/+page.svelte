<script lang="ts">
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = [
    'Voucher profile and pricing',
    'Captive portal branding and login policy',
    'Device/session limit policy',
    'NAS integration and radius profile',
  ];
</script>

<ServiceComingSoonPage
  title="Hotspot Service"
  description="This dedicated order flow is prepared. Service activation is still in roadmap stage."
  statusLabel="Coming Soon"
  icon="radio"
  {roadmap}
  nextTarget="After ISP internet order flow is stabilized, hotspot ordering and provisioning will be activated."
/>
