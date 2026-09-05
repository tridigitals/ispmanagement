<script lang="ts">
  /*
    Detail pengumuman v2 — memuat satu pengumuman lewat api.announcements.get
    dan menampilkannya dengan AnnouncementDetailView yang sama dengan versi lama
    (`(app)/admin/announcements/[id]/+page.svelte`, 57 baris). Tautan kembali
    menunjuk ke rute v2.
  */
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api, type Announcement } from '$lib/api/client';
  import { can, user, tenant } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { hasInternalAppAccess } from '$lib/utils/appLanding';
  import AnnouncementDetailView from '$lib/components/announcements/AnnouncementDetailView.svelte';
  import { AppShell } from '$lib/components/ds';

  let loading = $state(true);
  let ann = $state<Announcement | null>(null);

  const id = $derived($page.params.id || '');

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const canManageAnnouncements = $derived($can('manage', 'announcements'));
  const canViewInAdminShell = $derived(hasInternalAppAccess($user));

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
    if (!canViewInAdminShell) {
      goto('/unauthorized');
      return;
    }
    void load();
  });
</script>

<AppShell title={ann?.title || 'Pengumuman'}>
  <AnnouncementDetailView
    announcement={ann}
    {loading}
    backHref={canManageAnnouncements
      ? `${tenantPrefix}/v2/admin/announcements`
      : `${tenantPrefix}/v2/admin`}
  />
</AppShell>
