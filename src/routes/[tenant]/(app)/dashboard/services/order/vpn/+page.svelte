<script lang="ts">
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = [
    'Site-to-site and remote-access mode',
    'User/group policy and route profile',
    'Certificate and key lifecycle management',
    'Monitoring and session analytics',
  ];
</script>

<ServiceComingSoonPage
  title="Managed VPN Service"
  description="VPN ordering and provisioning flow will be activated after policy and security module hardening."
  statusLabel="Coming Soon"
  icon="shield"
  {roadmap}
  nextTarget="VPN service will be opened with security templates and compliance logging support."
/>
