<script lang="ts">
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = [
    'Multi-site endpoint mapping',
    'SLA class and committed bandwidth',
    'Redundant path and failover policy',
    'Enterprise onboarding checklist',
  ];
</script>

<ServiceComingSoonPage
  title="Dedicated Link Service"
  description="Dedicated enterprise link order flow is reserved and will be enabled with SLA provisioning support."
  statusLabel="Coming Soon"
  icon="router"
  {roadmap}
  nextTarget="Service will open after hotspot baseline and enterprise provisioning module are ready."
/>
