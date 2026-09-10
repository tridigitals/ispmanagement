<script lang="ts">
  /*
    Pengumuman portal v2 — gelombang 25a.
    Versi lama: (app)/announcements/+page.svelte (515 baris).
    Perilaku identik: feed + cari + filter severity/mode + muat lagi.
    Pola DS: PortalShell + PageHeader + Field + Card + Badge + Button.
  */
  import { onMount } from 'svelte';
  import { api, type Announcement, type PaginatedResponse } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import { goto } from '$app/navigation';
  import { stripHtmlToText } from '$lib/utils/sanitizeHtml';
  import { getApiBaseUrl } from '$lib/utils/apiUrl';
  import { formatDateTime } from '$lib/utils/date';
  import { appSettings } from '$lib/stores/settings';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import type { StatusTone } from '$lib/components/ds/tokens';
  import { t } from 'svelte-i18n';

  const API_BASE = getApiBaseUrl();

  let rows = $state<Announcement[]>([]);
  let total = $state(0);
  let pageNum = $state(1);
  const perPage = 20;
  let loading = $state(true);
  let loadingMore = $state(false);
  let q = $state('');
  let sev = $state('all');
  let mode = $state('all');

  let hasMore = $derived(rows.length < total);

  function snippet(body: string) {
    const s = stripHtmlToText(body || '');
    return s.length <= 220 ? s : s.slice(0, 220) + '…';
  }
  function tone(sev: string): StatusTone {
    if (sev === 'success') return 'positive';
    if (sev === 'warning') return 'warning';
    if (sev === 'error') return 'negative';
    return 'info';
  }
  function label(sev: string) {
    return { success: $t('announcements.v2.f_success'), warning: $t('announcements.v2.f_warning'), error: $t('announcements.v2.f_error') }[sev] ?? 'Info';
  }
  function clearFilters() {
    q = '';
    sev = 'all';
    mode = 'all';
  }
  function openDetail(id: string) {
    goto(`/v2/announcements/${id}`);
  }
  async function load(reset: boolean) {
    loading = true;
    if (reset) {
      pageNum = 1;
      rows = [];
      total = 0;
    }
    try {
      const res = await api.announcements.listRecent({
        page: pageNum,
        per_page: perPage,
        search: q.trim() || undefined,
        severity: sev === 'all' ? undefined : (sev as 'info' | 'success' | 'warning' | 'error'),
        mode: mode === 'all' ? undefined : (mode as 'post' | 'banner'),
      });
      total = res.total || 0;
      rows = reset ? res.data : [...rows, ...res.data];
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loading = false;
    }
  }
  onMount(() => {
    void load(true);
    const onChange = () => void load(true);
    window.addEventListener('announcements_changed', onChange);
    return () => window.removeEventListener('announcements_changed', onChange);
  });
  $effect(() => {
    const _q = q, _s = sev, _m = mode;
    const timer = setTimeout(() => void load(true), 250);
    return () => clearTimeout(timer);
  });
  async function loadMore() {
    if (loadingMore || loading || !hasMore) return;
    loadingMore = true;
    try {
      pageNum += 1;
      const res = await api.announcements.listRecent({
        page: pageNum,
        per_page: perPage,
        search: q.trim() || undefined,
        severity: sev === 'all' ? undefined : (sev as 'info' | 'success' | 'warning' | 'error'),
        mode: mode === 'all' ? undefined : (mode as 'post' | 'banner'),
      });
      total = res.total || total;
      rows = [...rows, ...res.data];
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      loadingMore = false;
    }
  }
</script>

<PortalShell title={ $t('announcements.v2.title') }>
  <PageHeader
    title={ $t('announcements.v2.title') }
    desc={ $t('announcements.v2.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={() => load(true)}>
        { $t('common.refresh') }
      </Button>
    {/snippet}
  </PageHeader>

  <div class="mb-4 grid gap-3 sm:grid-cols-[minmax(0,1fr)_170px_150px]">
    <Field
      id="ann-search"
      label={ $t('common.search') }
      type="text"
      value={q}
      placeholder={ $t('announcements.v2.search_ph') }
      onchange={(v) => (q = v)}
    />
    <Field
      id="ann-sev"
      label={ $t('announcements.v2.f_level') }
      type="select"
      value={sev}
      options={[
        { value: 'all', label: $t('announcements.v2.all') },
        { value: 'info', label: 'Info' },
        { value: 'success', label: $t('announcements.v2.f_success') },
        { value: 'warning', label: $t('announcements.v2.f_warning') },
        { value: 'error', label: $t('announcements.v2.f_error') },
      ]}
      onchange={(v) => (sev = v)}
    />
    <Field
      id="ann-mode"
      label={ $t('announcements.v2.f_kind') }
      type="select"
      value={mode}
      options={[
        { value: 'all', label: $t('announcements.v2.all') },
        { value: 'post', label: $t('announcements.v2.f_post') },
        { value: 'banner', label: $t('announcements.v2.f_banner') },
      ]}
      onchange={(v) => (mode = v)}
    />
  </div>
  {#if q.trim() || sev !== 'all' || mode !== 'all'}
    <div class="mb-4">
      <Button variant="ghost" size="sm" icon="close" onclick={clearFilters}>
        { $t('announcements.v2.clear_filters') }
      </Button>
    </div>
  {/if}

  {#if loading && rows.length === 0}
    <Card title={ $t('common.loading') }><p class="text-sm text-ink-500">{ $t('announcements.v2.fetching') }</p></Card>
  {:else if rows.length === 0}
    <Card title={ $t('announcements.v2.empty') }
      ><p class="text-sm text-ink-500">{ $t('announcements.v2.empty_hint') }</p></Card
    >
  {:else}
    <p class="mb-3 text-sm text-ink-500">
      { $t('announcements.v2.count_updated', { values: { n: rows.length } }) }
      {formatDateTime(new Date().toISOString(), { timeZone: $appSettings.app_timezone })}
    </p>
    <div class="grid gap-4 md:grid-cols-2">
      {#each rows as a (a.id)}
        <Card title={a.title}>
          {#snippet aside()}
            <Badge tone={tone(a.severity)} label={label(a.severity)} />
          {/snippet}
          {#if a.cover_file_id}
            <img
              src={`${API_BASE}/storage/files/${a.cover_file_id}/content`}
              alt=""
              loading="lazy"
              class="mb-3 h-36 w-full rounded-lg object-cover"
            />
          {/if}
          <p class="mb-1 text-xs text-ink-400">
            {formatDateTime(a.starts_at, { timeZone: $appSettings.app_timezone })}
            {#if a.mode === 'banner'} · Banner{/if}
          </p>
          <p class="mb-3 text-sm text-ink-600">{snippet(a.body)}</p>
          <Button variant="ghost" size="sm" icon="chevronRight" onclick={() => openDetail(a.id)}>
            Baca
          </Button>
        </Card>
      {/each}
    </div>
    {#if hasMore}
      <div class="mt-4 flex justify-center">
        <Button variant="secondary" icon="chevronDown" disabled={loadingMore} onclick={loadMore}>
          {loadingMore ? $t('common.loading') : $t('common.load_more')}
        </Button>
      </div>
    {/if}
  {/if}
</PortalShell>
