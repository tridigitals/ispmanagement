<script lang="ts">
  /*
    Pengaturan wallboard NOC v2.

    Versi lama (`(app)/admin/network/noc/wallboard/settings/+page.svelte`,
    615 baris) memakai tema kaca gelap sendiri yang menyembunyikan chrome
    aplikasi. v2 memakai komponen DS yang sama (Field select/toggle + SaveBar
    implisit lewat tombol Simpan) dengan API yang sama: localStorage dulu,
    lalu api.settings.upsert best-effort kalau punya izin settings.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { can, user, tenant } from '$lib/stores/auth';
  import { api } from '$lib/api/client';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import { t } from 'svelte-i18n';
  import {
    FOCUS_MODE_KEY,
    KEEP_AWAKE_KEY,
    POLL_MS_KEY,
    ROTATE_MODE_KEY,
    ROTATE_MS_KEY,
    SETTINGS_LAYOUT_KEY,
    STATUS_FILTER_KEY,
    WALLBOARD_LAYOUT_PRESETS,
    WALLBOARD_POLL_MS_OPTIONS,
    WALLBOARD_ROTATE_MODES,
    WALLBOARD_ROTATE_MS_OPTIONS,
    WALLBOARD_STATUS_FILTERS,
    isLayoutPreset,
    isRotateMode,
    isStatusFilter,
    type LayoutPreset,
    type RotateMode,
    type StatusFilter,
  } from '$lib/constants/wallboard';
  import {
    AppShell,
    Button,
    Card,
    Field,
    PageHeader,
  } from '$lib/components/ds';

  let layout = $state<LayoutPreset>('3x3');
  let rotateMode = $state<RotateMode>('manual');
  let rotateMs = $state(10000);
  let statusFilter = $state<StatusFilter>('all');
  let pollMs = $state(1000);
  let keepAwake = $state(false);
  let focusMode = $state(false);
  let saving = $state(false);

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $pageStore.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $pageStore.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const canUseTenantSettings = $derived($can('read', 'settings') || $can('update', 'settings'));

  const layoutOptions = WALLBOARD_LAYOUT_PRESETS.map((preset) => ({ value: preset, label: preset }));
  const statusOptions = $derived(WALLBOARD_STATUS_FILTERS.map((value) => ({
    value,
    label: value === 'all' ? $t('common.all') : value === 'online' ? 'Online' : 'Offline',
  })));
  const rotateModeOptions = $derived(WALLBOARD_ROTATE_MODES.map((value) => ({
    value,
    label: value === 'manual' ? 'Manual' : $t('network.wallboard_set.auto'),
  })));
  const rotateMsOptions = $derived(WALLBOARD_ROTATE_MS_OPTIONS.map((value) => ({
    value: String(value),
    label: $t('common.time.sec', { values: { n: Math.floor(value / 1000) } }),
  })));
  const pollMsOptions = $derived(WALLBOARD_POLL_MS_OPTIONS.map((value) => ({
    value: String(value),
    label: $t('common.time.sec', { values: { n: Math.floor(value / 1000) } }),
  })));

  function loadLocal() {
    try {
      const l = localStorage.getItem(SETTINGS_LAYOUT_KEY);
      if (isLayoutPreset(l)) layout = l;
      const rm = localStorage.getItem(ROTATE_MODE_KEY);
      if (isRotateMode(rm)) rotateMode = rm;
      const rms = Number(localStorage.getItem(ROTATE_MS_KEY) || 10000);
      if ((WALLBOARD_ROTATE_MS_OPTIONS as readonly number[]).includes(rms)) rotateMs = rms;
      const sf = localStorage.getItem(STATUS_FILTER_KEY);
      if (isStatusFilter(sf)) statusFilter = sf;
      const pm = Number(localStorage.getItem(POLL_MS_KEY) || 1000);
      if ((WALLBOARD_POLL_MS_OPTIONS as readonly number[]).includes(pm)) pollMs = pm;
      const ka = localStorage.getItem(KEEP_AWAKE_KEY);
      if (ka != null) keepAwake = ka === '1' || ka === 'true';
      const fm = localStorage.getItem(FOCUS_MODE_KEY);
      if (fm != null) focusMode = fm === '1' || fm === 'true';
    } catch {
      // abaikan: penyimpanan lokal tidak tersedia
    }
  }

  async function loadRemoteAll() {
    if (!canUseTenantSettings) return;
    try {
      const [rl, rm, rs, sf, pm] = await Promise.all([
        api.settings.getValue(SETTINGS_LAYOUT_KEY),
        api.settings.getValue(ROTATE_MODE_KEY),
        api.settings.getValue(ROTATE_MS_KEY),
        api.settings.getValue(STATUS_FILTER_KEY),
        api.settings.getValue(POLL_MS_KEY),
      ]);
      if (isLayoutPreset(rl)) layout = rl;
      if (isRotateMode(rm)) rotateMode = rm;
      const rms = Number(rs || 10000);
      if ((WALLBOARD_ROTATE_MS_OPTIONS as readonly number[]).includes(rms)) rotateMs = rms;
      if (isStatusFilter(sf)) statusFilter = sf;
      const pms = Number(pm || 1000);
      if ((WALLBOARD_POLL_MS_OPTIONS as readonly number[]).includes(pms)) pollMs = pms;
    } catch (e) {
      // abaikan: best-effort
      console.error('loadRemoteAll gagal:', e);
    }
  }

  function saveLocal() {
    localStorage.setItem(SETTINGS_LAYOUT_KEY, layout);
    localStorage.setItem(ROTATE_MODE_KEY, rotateMode);
    localStorage.setItem(ROTATE_MS_KEY, String(rotateMs));
    localStorage.setItem(STATUS_FILTER_KEY, statusFilter);
    localStorage.setItem(POLL_MS_KEY, String(pollMs));
    localStorage.setItem(KEEP_AWAKE_KEY, keepAwake ? '1' : '0');
    localStorage.setItem(FOCUS_MODE_KEY, focusMode ? '1' : '0');
  }

  async function saveAndExit() {
    saving = true;
    try {
      saveLocal();
      if (canUseTenantSettings) {
        try {
          await Promise.all([
            api.settings.upsert(SETTINGS_LAYOUT_KEY, layout, 'Wallboard layout preset (tenant scoped)'),
            api.settings.upsert(ROTATE_MODE_KEY, rotateMode, 'Wallboard rotate mode'),
            api.settings.upsert(ROTATE_MS_KEY, String(rotateMs), 'Wallboard rotate interval (ms)'),
            api.settings.upsert(STATUS_FILTER_KEY, statusFilter, 'Wallboard status filter'),
            api.settings.upsert(POLL_MS_KEY, String(pollMs), 'Wallboard poll interval (ms)'),
            api.settings.upsert(KEEP_AWAKE_KEY, keepAwake ? 'true' : 'false', 'Wallboard keep awake'),
            api.settings.upsert(FOCUS_MODE_KEY, focusMode ? 'true' : 'false', 'Wallboard focus mode'),
          ]);
        } catch (e) {
          // simpan jarak jauh best-effort
          console.error('upsert pengaturan jarak jauh gagal:', e);
        }
      }
      toast.success($t('network.wallboard_set.saved'))
      await goto(`${tenantPrefix}/v2/admin/network/noc/wallboard`);
    } catch (e: unknown) {
      toast.error(extractApiErrorMessage(e, $t('network.wallboard_set.save_fail')));
    } finally {
      saving = false;
    }
  }

  function exitWithoutSave() {
    void goto(`${tenantPrefix}/v2/admin/network/noc/wallboard`);
  }

  onMount(() => {
    if (!$can('read', 'network_noc') && !$can('manage', 'network_noc')) {
      goto('/unauthorized');
      return;
    }
    loadLocal();
    void loadRemoteAll();
  });
</script>

<AppShell title={ $t('network.wallboard_set.title') }>
  <PageHeader
    title={ $t('network.wallboard_set.title') }
    eyebrow="Jaringan · NOC"
    desc={ $t('network.wallboard_set.desc') }
  >
    {#snippet actions()}
      <Button variant="ghost" onclick={exitWithoutSave}>{ $t('network.wallboard_set.discard') }</Button>
      <Button variant="primary" loading={saving} onclick={() => void saveAndExit()}>
        Simpan & keluar
      </Button>
    {/snippet}
  </PageHeader>

  <div class="mt-4 grid gap-4">
    <Card title={ $t('network.wallboard_set.sec_view') }>
      <Field
        id="wb-layout"
        label={ $t('network.wallboard_set.layout') }
        type="select"
        value={layout}
        options={layoutOptions}
        help="Jumlah ubin router yang tampil di layar wallboard."
        onchange={(v) => (layout = v as LayoutPreset)}
      />
      <Field
        id="wb-filter"
        label={ $t('network.wallboard_set.status_filter') }
        type="select"
        value={statusFilter}
        options={statusOptions}
        help="Batasi ubin yang tampil berdasarkan status router."
        onchange={(v) => (statusFilter = v as StatusFilter)}
      />
      <Field
        id="wb-focus"
        label={ $t('network.wallboard_set.focus_mode') }
        type="toggle"
        value={focusMode ? 'true' : 'false'}
        help="Sembunyikan router sehat, tampilkan yang bermasalah saja."
        onchange={(v) => (focusMode = v === 'true')}
      />
    </Card>

    <Card title={ $t('network.wallboard_set.sec_rot') }>
      <Field
        id="wb-rotate-mode"
        label={ $t('network.wallboard_set.rot_mode') }
        type="select"
        value={rotateMode}
        options={rotateModeOptions}
        help={ $t('network.wallboard_set.auto_help') }
        onchange={(v) => (rotateMode = v as RotateMode)}
      />
      <Field
        id="wb-rotate-ms"
        label={ $t('network.wallboard_set.rot_interval') }
        type="select"
        value={String(rotateMs)}
        options={rotateMsOptions}
        onchange={(v) => (rotateMs = Number(v))}
      />
      <Field
        id="wb-poll"
        label={ $t('network.wallboard_set.poll_interval') }
        type="select"
        value={String(pollMs)}
        options={pollMsOptions}
        help="Seberapa sering wallboard mengambil status terbaru."
        onchange={(v) => (pollMs = Number(v))}
      />
      <Field
        id="wb-awake"
        label={ $t('network.wallboard_set.keep_awake') }
        type="toggle"
        value={keepAwake ? 'true' : 'false'}
        help="Mencegah layar wallboard mati otomatis."
        onchange={(v) => (keepAwake = v === 'true')}
      />
    </Card>
  </div>
</AppShell>
