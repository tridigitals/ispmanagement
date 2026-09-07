<script lang="ts">
  /*
    Pengumuman detail portal v2 — gelombang 25a.
    Reuse AnnouncementDetailView (colocated legacy) — body artikel + cover.
    Perubahan: dibungkus PortalShell + backHref ke /v2/announcements.
  */
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { api, type Announcement } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import AnnouncementDetailView from '$lib/components/announcements/AnnouncementDetailView.svelte';

  let loading = $state(true);
  let ann = $state<Announcement | null>(null);

  const id = $derived($page.params.id || '');

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

<PortalShell title="Pengumuman">
  <AnnouncementDetailView
    announcement={ann}
    {loading}
    backHref="/v2/announcements"
    backLabel="Kembali ke pengumuman"
  />
</PortalShell>
