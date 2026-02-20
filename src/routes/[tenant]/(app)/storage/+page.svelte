<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { user, tenant } from '$lib/stores/auth';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

  onMount(() => {
    const ctx = resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    });
    goto(`${ctx.tenantPrefix}/dashboard`);
  });
</script>
