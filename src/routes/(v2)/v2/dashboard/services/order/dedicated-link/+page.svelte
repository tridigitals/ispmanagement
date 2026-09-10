<script lang="ts">
  /*
    dedicated-link order portal v2 — gelombang 25c (stub roadmap).
    Versi lama: (app)/dashboard/services/order/dedicated-link/+page.svelte.
    Pola DS: PortalShell + reuse ServiceComingSoonPage legacy.
  */
  import { t } from 'svelte-i18n';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = $derived([
    $t('dashboard.dedicated_order.rm1'),
    $t('dashboard.dedicated_order.rm2'),
    $t('dashboard.dedicated_order.rm3'),
    $t('dashboard.dedicated_order.rm4'),
  ]);
</script>

<PortalShell title={ $t('dashboard.dedicated_order.title') }>
  <ServiceComingSoonPage
    title={ $t('dashboard.dedicated_order.title') }
    description={ $t('dashboard.dedicated_order.desc') }
    statusLabel={ $t('dashboard.hotspot_order.soon') }
    icon="router"
    {roadmap}
    nextTarget={ $t('dashboard.dedicated_order.next') }
  />
</PortalShell>
