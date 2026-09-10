<script lang="ts">
  /*
    hotspot order portal v2 — gelombang 25c (stub roadmap).
    Versi lama: (app)/dashboard/services/order/hotspot/+page.svelte.
    Pola DS: PortalShell + reuse ServiceComingSoonPage legacy.
  */
  import { t } from 'svelte-i18n';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = $derived([
    $t('dashboard.hotspot_order.rm1'),
    $t('dashboard.hotspot_order.rm2'),
    $t('dashboard.hotspot_order.rm3'),
    $t('dashboard.hotspot_order.rm4'),
  ]);
</script>

<PortalShell title={ $t('dashboard.hotspot_order.title') }>
  <ServiceComingSoonPage
    title={ $t('dashboard.hotspot_order.title') }
    description={ $t('dashboard.hotspot_order.desc') }
    statusLabel={ $t('dashboard.hotspot_order.soon') }
    icon="radio"
    {roadmap}
    nextTarget={ $t('dashboard.hotspot_order.next') }
  />
</PortalShell>
