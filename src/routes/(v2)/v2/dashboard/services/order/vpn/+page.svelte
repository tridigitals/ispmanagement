<script lang="ts">
  /*
    vpn order portal v2 — gelombang 25c (stub roadmap).
    Versi lama: (app)/dashboard/services/order/vpn/+page.svelte.
    Pola DS: PortalShell + reuse ServiceComingSoonPage legacy.
  */
  import { t } from 'svelte-i18n';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import ServiceComingSoonPage from '$lib/components/services/ServiceComingSoonPage.svelte';

  const roadmap = $derived([
    $t('dashboard.vpn_order.rm1'),
    $t('dashboard.vpn_order.rm2'),
    $t('dashboard.vpn_order.rm3'),
    $t('dashboard.vpn_order.rm4'),
  ]);
</script>

<PortalShell title={ $t('dashboard.vpn_order.title') }>
  <ServiceComingSoonPage
    title={ $t('dashboard.vpn_order.title') }
    description={ $t('dashboard.vpn_order.desc') }
    statusLabel={ $t('dashboard.hotspot_order.soon') }
    icon="shield"
    {roadmap}
    nextTarget={ $t('dashboard.vpn_order.next') }
  />
</PortalShell>
