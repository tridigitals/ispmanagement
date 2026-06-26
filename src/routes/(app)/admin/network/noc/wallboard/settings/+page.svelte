<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can, user, tenant } from '$lib/stores/auth';
  import MiniSelect from '$lib/components/ui/MiniSelect.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { api } from '$lib/api/client';
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
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

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
      // ignore
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
          // remote save best effort
        }
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
    if (!$can('read', 'network_noc') && !$can('manage', 'network_noc')) {
      goto('/unauthorized');
      return;
    }
    document.body.classList.add('wallboard-settings');
    loadLocal();
    void loadRemoteAll();
  });

  onDestroy(() => {
    document.body.classList.remove('wallboard-settings');
  });
</script>

<div class="wall-settings-wrap">
  <div class="wall-settings-head">
    <div>
      <div class="kicker">{$t('admin.network.wallboard.controls.title') || 'Wallboard'}</div>
      <h1>{$t('admin.network.wallboard.title') || 'Network Wallboard'}</h1>
      <p>{$t('network.noc.wallboard_settings') || 'Tampilan NOC live.'}</p>
    </div>
    <div class="actions">
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
          ...WALLBOARD_LAYOUT_PRESETS.map((preset) => ({
            value: preset,
            label: ($t(`admin.network.wallboard.layouts.${preset}`) as string) || preset,
          })),
        ]}
      />
    </div>

    <div class="field">
      <MiniSelect
        bind:value={statusFilter}
        label={$t('admin.network.wallboard.controls.filter') || 'Filter'}
        ariaLabel={$t('admin.network.wallboard.controls.filter') || 'Filter'}
        options={[
          ...WALLBOARD_STATUS_FILTERS.map((value) => ({
            value,
            label: ($t(`admin.network.wallboard.filters.${value}`) as string) || value,
          })),
        ]}
      />
    </div>

    <div class="field">
      <MiniSelect
        bind:value={rotateMode}
        label={$t('admin.network.wallboard.controls.pager') || 'Pager'}
        ariaLabel={$t('admin.network.wallboard.controls.pager') || 'Pager'}
        options={[
          ...WALLBOARD_ROTATE_MODES.map((value) => ({
            value,
            label:
              value === 'manual'
                ? (($t('admin.network.wallboard.controls.manual') as string) || 'Manual')
                : (($t('admin.network.wallboard.controls.auto_rotate') as string) || 'Auto rotate'),
          })),
        ]}
      />
    </div>

    <div class="field">
      <MiniSelect
        bind:value={rotateMs}
        label={$t('admin.network.wallboard.controls.rotate_every') || 'Rotate'}
        ariaLabel={$t('admin.network.wallboard.controls.rotate_every') || 'Rotate'}
        options={[
          ...WALLBOARD_ROTATE_MS_OPTIONS.map((value) => ({
            value,
            label: `${Math.floor(value / 1000)}s`,
          })),
        ]}
      />
    </div>

    <div class="field">
      <MiniSelect
        bind:value={pollMs}
        label={$t('admin.network.wallboard.poll') || 'Poll'}
        ariaLabel={$t('admin.network.wallboard.poll') || 'Poll'}
        options={[
          ...WALLBOARD_POLL_MS_OPTIONS.map((value) => ({
            value,
            label: `${Math.floor(value / 1000)}s`,
          })),
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
  /* ── Global overrides: hide chrome ── */
  :global(body.wallboard-settings header.topbar),
  :global(body.wallboard-settings .sidebar),
  :global(body.wallboard-settings .wrap[role='region']),
  :global(body.wallboard-settings .wrap.loading) {
    display: none;
  }

  :global(body.wallboard-settings .main-viewport) {
    padding: 0;
  }

  /* ── Page wrapper ── */
  .wall-settings-wrap {
    --glass-bg: rgba(15, 20, 35, 0.65);
    --glass-border: rgba(255, 255, 255, 0.06);
    --glass-blur: 18px;
    --input-bg: rgba(10, 14, 28, 0.7);
    --input-border: rgba(255, 255, 255, 0.08);
    --input-border-focus: var(--accent, #6c8cff);
    --glow-color: color-mix(in srgb, var(--accent, #6c8cff) 50%, transparent);

    min-height: 100dvh;
    padding: clamp(16px, 3vw, 40px);
    background: linear-gradient(
      145deg,
      #080c18 0%,
      #0d1225 40%,
      #0a0f20 100%
    );
    animation: ws-page-in 520ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  /* ── Header ── */
  .wall-settings-head {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-end;
    margin-bottom: 20px;
    padding: 20px 24px;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: 16px;
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
    animation: ws-header-in 600ms cubic-bezier(0.22, 1, 0.36, 1) 80ms both;
  }

  .kicker {
    font-size: 10px;
    letter-spacing: 0.14em;
    font-weight: 800;
    color: var(--accent, #6c8cff);
    text-transform: uppercase;
    margin-bottom: 4px;
    opacity: 0.85;
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.01em;
    color: var(--text-primary);
    line-height: 1.3;
  }

  p {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 13px;
    opacity: 0.7;
  }

  /* ── Action buttons ── */
  .actions {
    display: flex;
    gap: 10px;
    flex-shrink: 0;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: 100px;
    border: 1px solid var(--input-border);
    padding: 9px 18px;
    background: var(--glass-bg);
    backdrop-filter: blur(12px);
    color: var(--text-primary);
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    transition:
      background 200ms ease,
      border-color 200ms ease,
      box-shadow 200ms ease,
      transform 120ms ease;
    letter-spacing: 0.01em;
  }

  .btn:hover {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(255, 255, 255, 0.12);
    transform: translateY(-1px);
  }

  .btn:active {
    transform: translateY(0);
  }

  .btn.primary {
    background: color-mix(in srgb, var(--accent, #6c8cff) 16%, rgba(10, 14, 28, 0.8));
    border-color: color-mix(in srgb, var(--accent, #6c8cff) 40%, transparent);
    color: #fff;
    box-shadow:
      0 0 20px color-mix(in srgb, var(--accent, #6c8cff) 20%, transparent),
      0 0 4px color-mix(in srgb, var(--accent, #6c8cff) 12%, transparent);
  }

  .btn.primary:hover {
    background: color-mix(in srgb, var(--accent, #6c8cff) 24%, rgba(10, 14, 28, 0.8));
    border-color: color-mix(in srgb, var(--accent, #6c8cff) 55%, transparent);
    box-shadow:
      0 0 32px color-mix(in srgb, var(--accent, #6c8cff) 30%, transparent),
      0 0 8px color-mix(in srgb, var(--accent, #6c8cff) 18%, transparent);
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: default;
    transform: none;
    box-shadow: none;
  }

  /* ── Form grid ── */
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
    border: 1px solid var(--glass-border);
    border-radius: 16px;
    padding: 24px;
    background: var(--glass-bg);
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
    animation: ws-grid-in 620ms cubic-bezier(0.22, 1, 0.36, 1) 180ms both;
  }

  .field {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  /* Style MiniSelect internals (labels + native selects it may render) */
  .field :global(label),
  .field :global(.label),
  .field :global(.mini-select-label) {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-bottom: 2px;
  }

  .field :global(select),
  .field :global(input:not([type='checkbox'])) {
    background: var(--input-bg);
    border: 1px solid var(--input-border);
    border-radius: 10px;
    padding: 10px 14px;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 500;
    outline: none;
    transition:
      border-color 200ms ease,
      box-shadow 200ms ease;
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
  }

  .field :global(select:hover),
  .field :global(input:not([type='checkbox']):hover) {
    border-color: rgba(255, 255, 255, 0.14);
  }

  .field :global(select:focus),
  .field :global(input:not([type='checkbox']):focus) {
    border-color: var(--input-border-focus);
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--accent, #6c8cff) 18%, transparent),
      0 0 12px color-mix(in srgb, var(--accent, #6c8cff) 8%, transparent);
  }

  .field :global(select option) {
    background: #0d1225;
    color: var(--text-primary);
  }

  /* ── Toggle switches ── */
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border: 1px solid var(--glass-border);
    border-radius: 12px;
    background: var(--glass-bg);
    backdrop-filter: blur(12px);
    color: var(--text-primary);
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    transition: background 200ms ease;
    user-select: none;
  }

  .toggle:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  /* Custom switch track */
  .toggle input[type='checkbox'] {
    -webkit-appearance: none;
    appearance: none;
    position: relative;
    width: 40px;
    height: 22px;
    flex-shrink: 0;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.1);
    cursor: pointer;
    transition:
      background 250ms ease,
      border-color 250ms ease,
      box-shadow 250ms ease;
    outline: none;
  }

  /* Custom switch knob */
  .toggle input[type='checkbox']::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.4);
    transition:
      transform 250ms cubic-bezier(0.34, 1.56, 0.64, 1),
      background 250ms ease;
  }

  /* Checked state */
  .toggle input[type='checkbox']:checked {
    background: color-mix(in srgb, var(--accent, #6c8cff) 35%, rgba(10, 14, 28, 0.9));
    border-color: color-mix(in srgb, var(--accent, #6c8cff) 50%, transparent);
    box-shadow: 0 0 14px color-mix(in srgb, var(--accent, #6c8cff) 25%, transparent);
  }

  .toggle input[type='checkbox']:checked::after {
    transform: translateX(18px);
    background: var(--accent, #6c8cff);
    box-shadow: 0 0 8px color-mix(in srgb, var(--accent, #6c8cff) 45%, transparent);
  }

  .toggle input[type='checkbox']:focus-visible {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent, #6c8cff) 25%, transparent);
  }

  .toggle span {
    line-height: 1;
  }

  /* ── Responsive ── */
  @media (max-width: 768px) {
    .wall-settings-wrap {
      padding: 12px;
    }

    .wall-settings-head {
      flex-direction: column;
      align-items: stretch;
      padding: 16px;
      gap: 14px;
    }

    .actions {
      justify-content: flex-end;
    }

    .btn {
      padding: 8px 14px;
      font-size: 12px;
    }

    h1 {
      font-size: 18px;
    }

    .grid {
      grid-template-columns: 1fr;
      padding: 16px;
      gap: 12px;
    }
  }

  /* ── Animations ── */
  @keyframes ws-page-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes ws-header-in {
    from {
      opacity: 0;
      transform: translateY(-10px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes ws-grid-in {
    from {
      opacity: 0;
      transform: translateY(16px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
