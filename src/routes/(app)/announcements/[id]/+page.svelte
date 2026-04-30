<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { api, type Announcement } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { user, tenant } from '$lib/stores/auth';
  import AnnouncementDetailView from '$lib/components/announcements/AnnouncementDetailView.svelte';

  let loading = $state(true);
  let ann = $state<Announcement | null>(null);

  const id = $derived($page.params.id || '');

  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);

  async function load() {
    loading = true;
    try {
      if (!id) return;
      ann = await api.announcements.get(id);
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<AnnouncementDetailView announcement={ann} {loading} backHref={`${tenantPrefix}/announcements`} />
