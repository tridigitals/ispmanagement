<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  type Unit = 'Kbps' | 'Mbps' | 'Gbps';

  let {
    iface = '',
    routerLabel = '',
    currentRxBps = null,
    currentTxBps = null,
    thWarnRxKbps = $bindable(''),
    thWarnTxKbps = $bindable(''),
    thWarnRxUnit = $bindable<Unit>('Kbps'),
    thWarnTxUnit = $bindable<Unit>('Kbps'),
    formatBps,
    onClose,
    onChangeInterface,
    onClear,
    onSave,
  }: {
    iface?: string;
    routerLabel?: string;
    currentRxBps?: number | null;
    currentTxBps?: number | null;
    thWarnRxKbps?: string;
    thWarnTxKbps?: string;
    thWarnRxUnit?: Unit;
    thWarnTxUnit?: Unit;
    formatBps: (bps?: number | null) => string;
    onClose: () => void;
    onChangeInterface: () => void;
    onClear: () => void;
    onSave: () => void;
  } = $props();
</script>

<div class="threshold-overlay" role="dialog" aria-modal="true">
  <button
    class="threshold-backdrop"
    type="button"
    onclick={onClose}
    aria-label={$t('common.close') || 'Close'}
  ></button>
  <div class="threshold">
    <div class="threshold-head">
      <div>
        <div class="full-kicker">{$t('admin.network.wallboard.thresholds.title') || 'Thresholds'}</div>
        <div class="full-title">
          <span class="mono">{iface || ''}</span>
          <span class="muted">·</span>
          <span>{routerLabel}</span>
        </div>
      </div>
      <div class="full-actions">
        <button class="btn-mini" type="button" onclick={onChangeInterface}>
          <Icon name="settings" size={16} />
          {$t('admin.network.wallboard.thresholds.change_interface') || 'Change interface'}
        </button>
        <button class="icon-x" type="button" onclick={onClose} title={$t('common.close') || 'Close'}>
          <Icon name="x" size={18} />
        </button>
      </div>
    </div>

    <div class="threshold-summary">
      <div class="threshold-chip">
        <span class="k">{$t('admin.network.wallboard.thresholds.current_rx') || 'Current RX threshold'}</span>
        <span class="v mono">{currentRxBps != null
          ? formatBps(currentRxBps)
          : $t('admin.network.wallboard.thresholds.not_set') || 'Not set'}</span>
      </div>
      <div class="threshold-chip">
        <span class="k">{$t('admin.network.wallboard.thresholds.current_tx') || 'Current TX threshold'}</span>
        <span class="v mono">{currentTxBps != null
          ? formatBps(currentTxBps)
          : $t('admin.network.wallboard.thresholds.not_set') || 'Not set'}</span>
      </div>
    </div>

    <div class="tile-settings">
      <div class="settings-grid">
        <label class="field">
          <span class="k">{$t('admin.network.wallboard.warn_below_rx') || 'Warn if RX below'}</span>
          <div class="row">
            <input
              inputmode="numeric"
              value={thWarnRxKbps}
              oninput={(e) => (thWarnRxKbps = (e.currentTarget as HTMLInputElement).value)}
              placeholder="0"
            />
            <select class="unit-select" value={thWarnRxUnit} onchange={(e) => (thWarnRxUnit = (e.currentTarget as HTMLSelectElement).value as Unit)}>
              <option value="Kbps">Kbps</option>
              <option value="Mbps">Mbps</option>
              <option value="Gbps">Gbps</option>
            </select>
          </div>
          <span class="hint">{$t('admin.network.wallboard.thresholds.hint') || 'Leave empty to disable warning.'}</span>
        </label>
        <label class="field">
          <span class="k">{$t('admin.network.wallboard.warn_below_tx') || 'Warn if TX below'}</span>
          <div class="row">
            <input
              inputmode="numeric"
              value={thWarnTxKbps}
              oninput={(e) => (thWarnTxKbps = (e.currentTarget as HTMLInputElement).value)}
              placeholder="0"
            />
            <select class="unit-select" value={thWarnTxUnit} onchange={(e) => (thWarnTxUnit = (e.currentTarget as HTMLSelectElement).value as Unit)}>
              <option value="Kbps">Kbps</option>
              <option value="Mbps">Mbps</option>
              <option value="Gbps">Gbps</option>
            </select>
          </div>
          <span class="hint">{$t('admin.network.wallboard.thresholds.hint') || 'Leave empty to disable warning.'}</span>
        </label>
      </div>

      <div class="settings-actions">
        <button class="btn-mini ghost" type="button" onclick={onClear}>
          {$t('common.clear') || 'Clear'}
        </button>
        <button class="btn-mini primary" type="button" onclick={onSave}>
          <Icon name="save" size={16} />
          {$t('common.save') || 'Save'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .threshold-overlay {
    position: fixed;
    inset: 0;
    z-index: 95;
    display: grid;
    place-items: center;
  }
  .threshold-backdrop {
    position: absolute;
    inset: 0;
    border: none;
    background: rgba(0, 0, 0, 0.7);
  }
  .threshold {
    position: relative;
    width: min(860px, calc(100vw - 24px));
    max-height: min(740px, calc(100vh - 24px));
    overflow: auto;
    border-radius: 18px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    box-shadow: var(--shadow-lg);
    padding: 14px;
  }
  .threshold-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .full-kicker {
    color: var(--text-muted);
    letter-spacing: 0.14em;
    font-weight: 900;
    font-size: 11px;
  }
  .full-title {
    margin-top: 6px;
    font-size: 22px;
    font-weight: 950;
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .full-actions {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .icon-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 55%, transparent);
    color: var(--text-primary);
    cursor: pointer;
  }
  .btn-mini {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 55%, transparent);
    color: var(--text-primary);
    cursor: pointer;
    font-weight: 850;
    font-size: 13px;
    white-space: nowrap;
  }
  .btn-mini.primary {
    border-color: color-mix(in srgb, var(--accent) 65%, var(--border-color));
    background: color-mix(in srgb, var(--accent) 22%, var(--bg-surface));
  }
  .btn-mini.ghost {
    background: transparent;
  }
  .threshold-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 10px;
  }
  .threshold-chip {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
    padding: 10px 12px;
    display: grid;
    gap: 4px;
  }
  .threshold-chip .k {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 900;
    color: var(--text-muted);
  }
  .threshold-chip .v {
    font-size: 14px;
    font-weight: 900;
  }
  .tile-settings {
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 12px;
    background:
      linear-gradient(
        to bottom,
        color-mix(in srgb, var(--bg-surface) 82%, transparent),
        color-mix(in srgb, var(--bg-surface) 68%, transparent)
      );
  }
  .settings-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .field {
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 10px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
  }
  .field .k {
    display: block;
    font-size: 11px;
    letter-spacing: 0.12em;
    font-weight: 900;
    color: var(--text-muted);
    text-transform: uppercase;
    margin-bottom: 8px;
  }
  .field .hint {
    display: block;
    margin-top: 8px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .field .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .field input {
    width: 100%;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    color: var(--text-primary);
    outline: none;
  }
  .unit-select {
    width: 92px;
    min-width: 92px;
    max-width: 92px;
    padding: 10px 8px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 65%, transparent);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 700;
    outline: none;
  }
  .settings-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid color-mix(in srgb, var(--border-color) 85%, transparent);
  }
  .muted {
    color: var(--text-muted);
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }

  @media (max-width: 920px) {
    .threshold-summary {
      grid-template-columns: 1fr;
    }
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
