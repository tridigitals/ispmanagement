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
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
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
  const statusOptions = WALLBOARD_STATUS_FILTERS.map((value) => ({
    value,
    label: value === 'all' ? 'Semua' : value === 'online' ? 'Online' : 'Offline',
  }));
  const rotateModeOptions = WALLBOARD_ROTATE_MODES.map((value) => ({
    value,
    label: value === 'manual' ? 'Manual' : 'Otomatis',
  }));
  const rotateMsOptions = WALLBOARD_ROTATE_MS_OPTIONS.map((value) => ({
    value: String(value),
    label: `${Math.floor(value / 1000)} detik`,
  }));
  const pollMsOptions = WALLBOARD_POLL_MS_OPTIONS.map((value) => ({
    value: String(value),
    label: `${Math.floor(value / 1000)} detik`,
  }));

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
    } catch {
      // abaikan: best-effort
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
        } catch {
          // simpan jarak jauh best-effort
        }
      }
      toast.success('Pengaturan wallboard disimpan');
      await goto(`${tenantPrefix}/v2/admin/network/noc/wallboard`);
    } catch (e: any) {
      toast.error(e?.message || e || 'Gagal menyimpan pengaturan');
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

<AppShell title="Pengaturan wallboard">
  <PageHeader
    title="Pengaturan wallboard"
    eyebrow="Jaringan · NOC"
    desc="Tata letak, rotasi halaman, dan filter status untuk layar wallboard NOC."
  >
    {#snippet actions()}
      <Button variant="ghost" onclick={exitWithoutSave}>Keluar tanpa menyimpan</Button>
      <Button variant="primary" loading={saving} onclick={() => void saveAndExit()}>
        Simpan & keluar
      </Button>
    {/snippet}
  </PageHeader>

  <div class="mt-4 grid gap-4">
    <Card title="Tampilan">
      <Field
        id="wb-layout"
        label="Tata letak"
        type="select"
        value={layout}
        options={layoutOptions}
        help="Jumlah ubin router yang tampil di layar wallboard."
        onchange={(v) => (layout = v as LayoutPreset)}
      />
      <Field
        id="wb-filter"
        label="Filter status"
        type="select"
        value={statusFilter}
        options={statusOptions}
        help="Batasi ubin yang tampil berdasarkan status router."
        onchange={(v) => (statusFilter = v as StatusFilter)}
      />
      <Field
        id="wb-focus"
        label="Mode fokus"
        type="toggle"
        value={focusMode ? 'true' : 'false'}
        help="Sembunyikan router sehat, tampilkan yang bermasalah saja."
        onchange={(v) => (focusMode = v === 'true')}
      />
    </Card>

    <Card title="Rotasi & polling">
      <Field
        id="wb-rotate-mode"
        label="Mode rotasi"
        type="select"
        value={rotateMode}
        options={rotateModeOptions}
        help="Otomatis mengganti halaman ubin sesuai interval."
        onchange={(v) => (rotateMode = v as RotateMode)}
      />
      <Field
        id="wb-rotate-ms"
        label="Interval rotasi"
        type="select"
        value={String(rotateMs)}
        options={rotateMsOptions}
        onchange={(v) => (rotateMs = Number(v))}
      />
      <Field
        id="wb-poll"
        label="Interval polling"
        type="select"
        value={String(pollMs)}
        options={pollMsOptions}
        help="Seberapa sering wallboard mengambil status terbaru."
        onchange={(v) => (pollMs = Number(v))}
      />
      <Field
        id="wb-awake"
        label="Jaga layar tetap menyala"
        type="toggle"
        value={keepAwake ? 'true' : 'false'}
        help="Mencegah layar wallboard mati otomatis."
        onchange={(v) => (keepAwake = v === 'true')}
      />
    </Card>
  </div>
</AppShell>
