<script lang="ts">
  import { t } from 'svelte-i18n';
  import { getNetworkMapFloatingControlsLayout } from './networkMapFloatingControlsLayout';

  export type NetworkMapFloatingLabels = {
    title: string;
    layers: string;
    view: string;
    standard: string;
    satellite: string;
    nodes: string;
    links: string;
    zones: string;
    routers: string;
    customers: string;
    assets: string;
  };

  let {
    labels,
    hidden = false,
    viewMode,
    nodesVisible,
    linksVisible,
    zonesVisible,
    routersVisible,
    customersVisible,
    topologyAssetsVisible,
    canShowRouters,
    onViewModeChange,
    onNodesVisibleChange,
    onLinksVisibleChange,
    onZonesVisibleChange,
    onRoutersVisibleChange,
    onCustomersVisibleChange,
    onTopologyAssetsVisibleChange,
    onToggleHidden,
  }: {
    labels: NetworkMapFloatingLabels;
    hidden?: boolean;
    viewMode: 'standard' | 'satellite';
    nodesVisible: boolean;
    linksVisible: boolean;
    zonesVisible: boolean;
    routersVisible: boolean;
    customersVisible: boolean;
    topologyAssetsVisible: boolean;
    canShowRouters: boolean;
    onViewModeChange: (mode: 'standard' | 'satellite') => void;
    onNodesVisibleChange: (checked: boolean) => void;
    onLinksVisibleChange: (checked: boolean) => void;
    onZonesVisibleChange: (checked: boolean) => void;
    onRoutersVisibleChange: (checked: boolean) => void;
    onCustomersVisibleChange: (checked: boolean) => void;
    onTopologyAssetsVisibleChange: (checked: boolean) => void;
    onToggleHidden: () => void;
  } = $props();

  function buttonClass(active: boolean) {
    return active ? 'control-chip active' : 'control-chip';
  }

  const layout = getNetworkMapFloatingControlsLayout();

  function handleWindowPointerDown(event: PointerEvent) {
    if (hidden) return;
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (target.closest('[data-network-map-controls-root]')) return;
    if (target.closest('[data-network-map-controls-toggle]')) return;
    onToggleHidden();
  }
</script>

<svelte:window onpointerdown={handleWindowPointerDown} />

{#if hidden}
  <button
    type="button"
    class="floating-controls-toggle"
    aria-label={$t('network.map.show_controls')}
    data-network-map-controls-toggle
    onclick={onToggleHidden}
  >
    <span>{$t('network.map.controls')}</span>
  </button>
{:else}
  <aside
    class="floating-controls"
    aria-label={$t('network.map.workspace_controls')}
    data-network-map-controls-root
    style={`--nm-controls-width:${layout.desktopWidth};--nm-controls-padding:${layout.desktopPadding};--nm-controls-radius:${layout.desktopRadius};--nm-controls-gap:${layout.desktopGap};--nm-controls-chip-min-height:${layout.chipMinHeight};--nm-controls-chip-padding-x:${layout.chipPaddingX};--nm-controls-chip-padding-y:${layout.chipPaddingY};--nm-controls-mobile-padding:${layout.mobilePadding};--nm-controls-mobile-radius:${layout.mobileRadius};`}
  >
    <div class="controls-head">
      <div class="controls-title">{labels.title}</div>
    </div>

    <section class="control-group">
      <div class="control-group-label">{labels.view}</div>
      <div class="control-row">
        <button
          type="button"
          class={buttonClass(viewMode === 'standard')}
          onclick={() => onViewModeChange('standard')}
        >
          {labels.standard}
        </button>
        <button
          type="button"
          class={buttonClass(viewMode === 'satellite')}
          onclick={() => onViewModeChange('satellite')}
        >
          {labels.satellite}
        </button>
      </div>
    </section>

    <section class="control-group">
      <div class="control-group-label">{labels.layers}</div>
      <div class="toggle-grid">
        <label class="toggle-chip">
          <input
            type="checkbox"
            checked={nodesVisible}
            onchange={(e) => onNodesVisibleChange((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>{labels.nodes}</span>
        </label>
        <label class="toggle-chip">
          <input
            type="checkbox"
            checked={linksVisible}
            onchange={(e) => onLinksVisibleChange((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>{labels.links}</span>
        </label>
        <label class="toggle-chip">
          <input
            type="checkbox"
            checked={zonesVisible}
            onchange={(e) => onZonesVisibleChange((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>{labels.zones}</span>
        </label>
        <label class="toggle-chip">
          <input
            type="checkbox"
            checked={topologyAssetsVisible}
            onchange={(e) =>
              onTopologyAssetsVisibleChange((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>{labels.assets}</span>
        </label>
        <label class="toggle-chip">
          <input
            type="checkbox"
            checked={customersVisible}
            onchange={(e) =>
              onCustomersVisibleChange((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>{labels.customers}</span>
        </label>
        {#if canShowRouters}
          <label class="toggle-chip">
            <input
              type="checkbox"
              checked={routersVisible}
              onchange={(e) =>
                onRoutersVisibleChange((e.currentTarget as HTMLInputElement).checked)}
            />
            <span>{labels.routers}</span>
          </label>
        {/if}
      </div>
    </section>
  </aside>
{/if}

<style>
  .floating-controls {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 6;
    width: min(var(--nm-controls-width), calc(100vw - 24px));
    display: grid;
    gap: var(--nm-controls-gap);
    border-radius: var(--nm-controls-radius);
    border: 1px solid rgba(148, 163, 184, 0.24);
    padding: var(--nm-controls-padding);
    background: rgba(255, 255, 255, 0.94);
        box-shadow:
      0 10px 24px rgba(15, 23, 42, 0.08),
      inset 0 1px 0 rgba(255, 255, 255, 0.65);
    max-height: min(68vh, 560px);
    overflow: auto;
  }

  .floating-controls-toggle {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 7;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 40px;
    padding: 0 14px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-color) 80%, transparent);
    background: rgba(15, 23, 42, 0.92);
    color: #f8fafc;
    font-size: 0.82rem;
    font-weight: 800;
        box-shadow: 0 14px 28px rgba(2, 6, 23, 0.24);
    cursor: pointer;
  }

  .controls-head {
    display: grid;
    gap: 0;
  }

  .controls-title {
    font-size: 0.68rem;
    font-weight: 900;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-primary) 52%, #475569 48%);
  }

  .control-group {
    display: grid;
    gap: 5px;
    padding-top: 4px;
    border-top: 1px solid rgba(148, 163, 184, 0.1);
  }

  .control-group-label {
    font-size: 0.58rem;
    font-weight: 900;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #94a3b8;
  }

  .control-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 5px;
  }

  .toggle-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 5px;
  }

  .control-chip,
  .toggle-chip {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    gap: 5px;
    border-radius: 12px;
    border: 1px solid rgba(148, 163, 184, 0.34);
    min-height: var(--nm-controls-chip-min-height);
    padding: var(--nm-controls-chip-padding-y) var(--nm-controls-chip-padding-x);
    background: rgba(255, 255, 255, 0.88);
    color: #334155;
    font-size: 0.72rem;
    font-weight: 760;
    cursor: pointer;
    box-shadow: 0 1px 3px rgba(15, 23, 42, 0.04);
    line-height: 1.15;
  }

  .control-chip.active {
    border-color: color-mix(in srgb, var(--color-primary) 46%, rgba(148, 163, 184, 0.34));
    background: color-mix(in srgb, var(--color-primary) 14%, white 86%);
    color: color-mix(in srgb, var(--color-primary) 72%, #1e293b 28%);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--color-primary) 10%, transparent),
      0 1px 4px rgba(79, 70, 229, 0.08);
  }

  .toggle-chip input {
    accent-color: var(--color-primary);
    margin: 0;
    transform: scale(0.84);
  }

  .toggle-chip:has(input:checked) {
    border-color: color-mix(in srgb, var(--color-primary) 42%, rgba(148, 163, 184, 0.34));
    background: color-mix(in srgb, var(--color-primary) 12%, white 88%);
    color: color-mix(in srgb, var(--color-primary) 68%, #1e293b 32%);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--color-primary) 8%, transparent),
      0 1px 4px rgba(79, 70, 229, 0.06);
  }

  @media (max-width: 768px) {
    .floating-controls-toggle {
      top: max(14px, env(safe-area-inset-top, 0px));
      right: 14px;
      bottom: auto;
    }

    .floating-controls {
      left: 14px;
      right: 14px;
      width: auto;
      top: calc(max(14px, env(safe-area-inset-top, 0px)) + 44px);
      bottom: auto;
      max-height: min(46vh, 420px);
      padding: var(--nm-controls-mobile-padding);
      border-radius: var(--nm-controls-mobile-radius);
    }

    .controls-title {
      font-size: 0.66rem;
    }
    .control-group {
      gap: 4px;
    }

    .control-row,
    .toggle-grid {
      gap: 5px;
    }

    .control-chip,
    .toggle-chip {
      padding: 5px 8px;
      font-size: 0.7rem;
      min-height: 28px;
    }

    .toggle-chip {
      justify-content: center;
    }

    .control-row > button {
      justify-content: center;
    }
  }
</style>
