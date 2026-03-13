<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import {
    WALLBOARD_LAYOUT_PRESETS,
    WALLBOARD_POLL_MS_OPTIONS,
    isLayoutPreset,
    type LayoutPreset,
  } from '$lib/constants/wallboard';

  let {
    refreshing = false,
    paused = false,
    isFullscreen = false,
    criticalSoundEnabled = true,
    pollMs = $bindable(1000),
    layout = $bindable('3x3' as LayoutPreset),
    onRefresh,
    onTogglePaused,
    onToggleFullscreen,
    onToggleCriticalSound,
    onExit,
  }: {
    refreshing?: boolean;
    paused?: boolean;
    isFullscreen?: boolean;
    criticalSoundEnabled?: boolean;
    pollMs?: number;
    layout?: LayoutPreset;
    onRefresh?: () => void | Promise<void>;
    onTogglePaused?: () => void;
    onToggleFullscreen?: () => void | Promise<void>;
    onToggleCriticalSound?: () => void;
    onExit?: () => void;
  } = $props();
</script>

<div class="wbic-panel">
  <div class="wbic-kicker">{$t('admin.network.wallboard.controls.open') || 'Controls'}</div>
  <div class="wbic-actions">
    <button onclick={() => onRefresh?.()} disabled={refreshing}>
      <Icon name="refresh-cw" size={16} />
      {$t('common.refresh') || 'Refresh'}
    </button>
    <button
      onclick={() => onTogglePaused?.()}
      title={paused ? $t('admin.network.wallboard.resume') || 'Resume' : $t('admin.network.wallboard.pause') || 'Pause'}
    >
      <Icon name={paused ? 'play' : 'pause'} size={16} />
      {paused ? $t('admin.network.wallboard.resume') || 'Resume' : $t('admin.network.wallboard.pause') || 'Pause'}
    </button>
    <button onclick={() => onToggleFullscreen?.()}>
      <Icon name="monitor" size={16} />
      {isFullscreen
        ? $t('admin.network.wallboard.exit_fullscreen') || 'Exit Fullscreen'
        : $t('admin.network.wallboard.fullscreen') || 'Fullscreen'}
    </button>
    <button
      onclick={() => onToggleCriticalSound?.()}
      title={$t('admin.network.wallboard.sound_toggle') || 'Toggle critical alert sound'}
    >
      <Icon name="alert-triangle" size={16} />
      {criticalSoundEnabled
        ? $t('admin.network.wallboard.sound_on') || 'Sound On'
        : $t('admin.network.wallboard.sound_off') || 'Sound Off'}
    </button>
    <button onclick={() => onExit?.()} title={$t('admin.network.wallboard.exit') || $t('sidebar.exit') || 'Exit'}>
      <Icon name="arrow-left" size={16} />
      {$t('admin.network.wallboard.exit') || $t('sidebar.exit') || 'Exit'}
    </button>
  </div>

  <div class="wbic-selects">
    <label class="wbic-field">
      <span class="muted">{$t('admin.network.wallboard.poll') || 'Poll'}</span>
      <select
        value={String(pollMs)}
        onchange={(e) => {
          const v = Number((e.currentTarget as HTMLSelectElement).value);
          if ((WALLBOARD_POLL_MS_OPTIONS as readonly number[]).includes(v)) pollMs = v;
        }}
      >
        {#each WALLBOARD_POLL_MS_OPTIONS as pollOpt}
          <option value={String(pollOpt)}>{Math.floor(pollOpt / 1000)}s</option>
        {/each}
      </select>
    </label>

    <label class="wbic-field">
      <span class="muted">{$t('admin.network.wallboard.controls.layout') || 'Layout'}</span>
      <select
        value={layout}
        onchange={(e) => {
          const v = (e.currentTarget as HTMLSelectElement).value;
          if (isLayoutPreset(v)) layout = v;
        }}
      >
        {#each WALLBOARD_LAYOUT_PRESETS as preset}
          <option value={preset}>{$t(`admin.network.wallboard.layouts.${preset}`) || preset}</option>
        {/each}
      </select>
    </label>
  </div>

</div>

<style>
  .wbic-panel {
    display: grid;
    gap: 8px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 8px;
    background: color-mix(in srgb, var(--bg-surface) 78%, transparent);
  }
  .wbic-kicker {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 900;
    color: var(--text-muted);
  }
  .wbic-actions {
    display: inline-flex;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    width: 100%;
    flex-wrap: wrap;
  }
  .wbic-actions button {
    flex: 1 1 45%;
    justify-content: center;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    font-weight: 650;
    font-size: 13px;
  }
  .wbic-actions button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .wbic-selects {
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    padding: 6px 8px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
  }
  .wbic-field {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 11px;
    font-weight: 700;
    white-space: nowrap;
  }
  .wbic-field select {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
    color: var(--text-primary);
    border-radius: 9px;
    padding: 5px 7px;
    font-size: 11px;
    font-weight: 800;
    line-height: 1.2;
    height: 34px;
    outline: none;
  }
  .muted {
    color: var(--text-muted);
  }
</style>
