<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can, user, tenant } from '$lib/stores/auth';
  import MiniSelect from '$lib/components/ui/MiniSelect.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { api } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

  type LayoutPreset = '2x2' | '3x2' | '3x3' | '4x3';
  type RotateMode = 'manual' | 'auto';
  type StatusFilter = 'all' | 'online' | 'offline';

  const SETTINGS_LAYOUT_KEY = 'mikrotik_wallboard_layout';
  const ROTATE_MODE_KEY = 'mikrotik_wallboard_rotate_mode';
  const ROTATE_MS_KEY = 'mikrotik_wallboard_rotate_ms';
  const FOCUS_MODE_KEY = 'mikrotik_wallboard_focus_mode';
  const STATUS_FILTER_KEY = 'mikrotik_wallboard_status_filter';
  const POLL_MS_KEY = 'mikrotik_wallboard_poll_ms';
  const KEEP_AWAKE_KEY = 'mikrotik_wallboard_keep_awake';

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

  function loadLocal() {
    try {
      const l = localStorage.getItem(SETTINGS_LAYOUT_KEY);
      if (l === '2x2' || l === '3x2' || l === '3x3' || l === '4x3') layout = l;
      const rm = localStorage.getItem(ROTATE_MODE_KEY);
      if (rm === 'manual' || rm === 'auto') rotateMode = rm;
      const rms = Number(localStorage.getItem(ROTATE_MS_KEY) || 10000);
      if ([5000, 10000, 15000, 30000, 60000].includes(rms)) rotateMs = rms;
      const sf = localStorage.getItem(STATUS_FILTER_KEY);
      if (sf === 'all' || sf === 'online' || sf === 'offline') statusFilter = sf;
      const pm = Number(localStorage.getItem(POLL_MS_KEY) || 1000);
      if ([1000, 2000, 5000].includes(pm)) pollMs = pm;
      const ka = localStorage.getItem(KEEP_AWAKE_KEY);
      if (ka != null) keepAwake = ka === '1' || ka === 'true';
      const fm = localStorage.getItem(FOCUS_MODE_KEY);
      if (fm != null) focusMode = fm === '1' || fm === 'true';
    } catch {
      // ignore
    }
  }

  async function loadRemoteLayout() {
    try {
      const rl = await api.settings.getValue(SETTINGS_LAYOUT_KEY);
      if (rl === '2x2' || rl === '3x2' || rl === '3x3' || rl === '4x3') layout = rl;
    } catch {
      // ignore
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

  async function navigateWithTransition(path: string) {
    try {
      const start = (document as any).startViewTransition;
      if (typeof start === 'function') {
        await start(() => goto(path)).finished;
        return;
      }
    } catch {
      // fallback
    }
    await goto(path);
  }

  async function saveAndExit() {
    saving = true;
    try {
      saveLocal();
      try {
        await api.settings.upsert(SETTINGS_LAYOUT_KEY, layout, 'Wallboard layout preset (tenant scoped)');
      } catch {
        // remote save best effort
      }
      toast.success($t('common.saved') || 'Saved');
      await navigateWithTransition(`${tenantPrefix}/admin/network/noc/wallboard`);
    } catch (e: any) {
      toast.error(e?.message || e || 'Failed to save settings');
    } finally {
      saving = false;
    }
  }

  function exitWithoutSave() {
    void navigateWithTransition(`${tenantPrefix}/admin/network/noc/wallboard`);
  }

  onMount(() => {
    if (!$can('read', 'network_routers') && !$can('manage', 'network_routers')) {
      goto('/unauthorized');
      return;
    }
    document.body.classList.add('wallboard-settings');
    loadLocal();
    void loadRemoteLayout();
  });

  onDestroy(() => {
    document.body.classList.remove('wallboard-settings');
  });
</script>

<div class="wall-settings-wrap">
  <div class="wall-settings-head">
    <div>
      <div class="kicker">{$t('admin.network.wallboard.controls.title') || 'Wallboard Settings'}</div>
      <h1>{$t('admin.network.wallboard.title') || 'Network Wallboard'}</h1>
      <p>{$t('admin.network.wallboard.subtitle') || 'Live NOC view optimized for 24/7 display.'}</p>
    </div>
    <div class="actions">
      <button class="btn ghost" type="button" onclick={exitWithoutSave}>
        <Icon name="arrow-left" size={16} />
        {$t('common.cancel') || 'Exit'}
      </button>
      <button class="btn primary" type="button" onclick={saveAndExit} disabled={saving}>
        <Icon name="save" size={16} />
        {saving ? ($t('common.saving') || 'Saving...') : ($t('common.save') || 'Save & Exit')}
      </button>
    </div>
  </div>

  <div class="grid">
    <div class="field">
      <MiniSelect
        bind:value={layout}
        label={$t('admin.network.wallboard.controls.layout') || 'Layout'}
        ariaLabel={$t('admin.network.wallboard.controls.layout') || 'Layout'}
        options={[
          { value: '2x2', label: '2x2' },
          { value: '3x2', label: '3x2' },
          { value: '3x3', label: '3x3' },
          { value: '4x3', label: '4x3' },
        ]}
      />
    </div>

    <div class="field">
      <MiniSelect
        bind:value={statusFilter}
        label={$t('admin.network.wallboard.controls.filter') || 'Filter'}
        ariaLabel={$t('admin.network.wallboard.controls.filter') || 'Filter'}
        options={[
          { value: 'all', label: $t('admin.network.wallboard.filters.all') || 'All' },
          { value: 'online', label: $t('admin.network.wallboard.filters.online') || 'Online' },
          { value: 'offline', label: $t('admin.network.wallboard.filters.offline') || 'Offline' },
        ]}
      />
    </div>

    <div class="field">
      <MiniSelect
        bind:value={rotateMode}
        label={$t('admin.network.wallboard.controls.pager') || 'Pager'}
        ariaLabel={$t('admin.network.wallboard.controls.pager') || 'Pager'}
        options={[
          { value: 'manual', label: $t('admin.network.wallboard.controls.manual') || 'Manual' },
          { value: 'auto', label: $t('admin.network.wallboard.controls.auto_rotate') || 'Auto rotate' },
        ]}
      />
    </div>

    <div class="field">
      <MiniSelect
        bind:value={rotateMs}
        label={$t('admin.network.wallboard.controls.rotate_every') || 'Rotate'}
        ariaLabel={$t('admin.network.wallboard.controls.rotate_every') || 'Rotate'}
        options={[
          { value: 5000, label: '5s' },
          { value: 10000, label: '10s' },
          { value: 15000, label: '15s' },
          { value: 30000, label: '30s' },
          { value: 60000, label: '60s' },
        ]}
      />
    </div>

    <div class="field">
      <MiniSelect
        bind:value={pollMs}
        label={$t('admin.network.wallboard.poll') || 'Poll'}
        ariaLabel={$t('admin.network.wallboard.poll') || 'Poll'}
        options={[
          { value: 1000, label: '1s' },
          { value: 2000, label: '2s' },
          { value: 5000, label: '5s' },
        ]}
      />
    </div>

    <label class="toggle">
      <input type="checkbox" bind:checked={keepAwake} />
      <span>{$t('admin.network.wallboard.keep_awake') || 'Keep awake'}</span>
    </label>

    <label class="toggle">
      <input type="checkbox" bind:checked={focusMode} />
      <span>{$t('admin.network.wallboard.focus_mode') || 'Focus mode'}</span>
    </label>
  </div>
</div>

<style>
  :global(body.wallboard-settings header.topbar),
  :global(body.wallboard-settings .sidebar),
  :global(body.wallboard-settings .wrap[role='region']),
  :global(body.wallboard-settings .wrap.loading) {
    display: none;
  }

  :global(body.wallboard-settings .main-viewport) {
    padding: 0;
  }

  .wall-settings-wrap {
    min-height: 100dvh;
    padding: clamp(16px, 2vw, 24px);
    background: var(--bg-base);
    animation: wall-settings-in 180ms ease-out;
  }

  .wall-settings-head {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
    margin-bottom: 20px;
  }

  .kicker {
    font-size: 12px;
    letter-spacing: 0.1em;
    font-weight: 800;
    color: var(--text-muted);
    text-transform: uppercase;
    margin-bottom: 8px;
  }

  h1 {
    margin: 0;
    font-size: 28px;
  }

  p {
    margin: 6px 0 0;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    gap: 10px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    padding: 10px 14px;
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
    color: var(--text-primary);
    font-weight: 700;
    cursor: pointer;
  }

  .btn.primary {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 18%, var(--bg-surface));
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 14px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }

  .field {
    min-width: 0;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    color: var(--text-primary);
    font-weight: 650;
  }

  .toggle input {
    accent-color: var(--accent);
  }

  @media (max-width: 900px) {
    .wall-settings-head {
      flex-direction: column;
    }

    .grid {
      grid-template-columns: 1fr;
    }
  }

  @keyframes wall-settings-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
