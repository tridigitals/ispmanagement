<script lang="ts">
  export type NetworkMapFloatingLabels = {
    title: string;
    layers: string;
    view: string;
    manage: string;
    standard: string;
    satellite: string;
    openNodes: string;
    openLinks: string;
    openZones: string;
    openBindings: string;
    addNode: string;
    addLink: string;
    addZone: string;
    nodes: string;
    links: string;
    zones: string;
    routers: string;
    customers: string;
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
    canShowRouters,
    canManageTopology,
    onViewModeChange,
    onNodesVisibleChange,
    onLinksVisibleChange,
    onZonesVisibleChange,
    onRoutersVisibleChange,
    onCustomersVisibleChange,
    onOpenManageNodes,
    onOpenManageLinks,
    onOpenManageZones,
    onOpenManageBindings,
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
    canShowRouters: boolean;
    canManageTopology: boolean;
    onViewModeChange: (mode: 'standard' | 'satellite') => void;
    onNodesVisibleChange: (checked: boolean) => void;
    onLinksVisibleChange: (checked: boolean) => void;
    onZonesVisibleChange: (checked: boolean) => void;
    onRoutersVisibleChange: (checked: boolean) => void;
    onCustomersVisibleChange: (checked: boolean) => void;
    onOpenManageNodes: () => void;
    onOpenManageLinks: () => void;
    onOpenManageZones: () => void;
    onOpenManageBindings: () => void;
    onToggleHidden: () => void;
  } = $props();

  function buttonClass(active: boolean) {
    return active ? 'control-chip active' : 'control-chip';
  }

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
    aria-label="Show map controls"
    data-network-map-controls-toggle
    onclick={onToggleHidden}
  >
    <span>Controls</span>
  </button>
{:else}
  <aside
    class="floating-controls"
    aria-label="Map workspace controls"
    data-network-map-controls-root
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

    {#if canManageTopology}
      <section class="control-group">
        <div class="control-group-label">{labels.manage}</div>
        <div class="manage-grid">
          <button type="button" class="control-chip" onclick={onOpenManageNodes}>
            {labels.openNodes}
          </button>
          <button type="button" class="control-chip" onclick={onOpenManageLinks}>
            {labels.openLinks}
          </button>
          <button type="button" class="control-chip" onclick={onOpenManageZones}>
            {labels.openZones}
          </button>
          <button type="button" class="control-chip" onclick={onOpenManageBindings}>
            {labels.openBindings}
          </button>
        </div>
      </section>
    {/if}
  </aside>
{/if}

<style>
  .floating-controls {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 6;
    width: min(320px, calc(100vw - 32px));
    display: grid;
    gap: 10px;
    border-radius: 22px;
    border: 1px solid rgba(15, 23, 42, 0.18);
    padding: 16px;
    background:
      linear-gradient(180deg, rgba(251, 248, 241, 0.98), rgba(245, 239, 228, 0.97)), #f7f0e4;
    backdrop-filter: blur(10px);
    box-shadow:
      0 24px 48px rgba(15, 23, 42, 0.14),
      inset 0 1px 0 rgba(255, 255, 255, 0.4);
    max-height: min(78vh, 720px);
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
    backdrop-filter: blur(14px);
    box-shadow: 0 14px 28px rgba(2, 6, 23, 0.24);
    cursor: pointer;
  }

  .controls-head {
    display: grid;
    gap: 2px;
  }

  .controls-title {
    font-size: 0.82rem;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-primary) 72%, #4338ca 28%);
  }

  .control-group {
    display: grid;
    gap: 7px;
    padding-top: 6px;
    border-top: 1px solid rgba(148, 163, 184, 0.18);
  }

  .control-group-label {
    font-size: 0.7rem;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #64748b;
  }

  .control-row,
  .toggle-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .manage-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .control-chip,
  .toggle-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border-radius: 999px;
    border: 1px solid rgba(15, 23, 42, 0.12);
    min-height: 38px;
    padding: 8px 12px;
    background: rgba(15, 23, 42, 0.92);
    color: #f8fafc;
    font-size: 0.8rem;
    font-weight: 800;
    cursor: pointer;
    box-shadow: 0 8px 20px rgba(15, 23, 42, 0.08);
  }

  .control-chip.active {
    border-color: color-mix(in srgb, var(--color-primary) 65%, var(--border-color));
    background: linear-gradient(180deg, rgba(67, 56, 202, 0.92), rgba(49, 46, 129, 0.96));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-primary) 25%, transparent);
  }

  .toggle-chip input {
    accent-color: var(--color-primary);
    margin: 0;
  }

  @media (max-width: 768px) {
    .floating-controls-toggle {
      top: auto;
      right: 14px;
      bottom: 14px;
    }

    .floating-controls {
      left: 14px;
      right: 14px;
      width: auto;
      top: auto;
      bottom: 14px;
      max-height: min(46vh, 420px);
      padding: 12px;
      border-radius: 18px;
    }

    .controls-title {
      font-size: 0.76rem;
    }
    .control-group {
      gap: 6px;
    }

    .control-row,
    .toggle-grid,
    .manage-grid {
      gap: 6px;
    }

    .control-chip,
    .toggle-chip {
      padding: 8px 10px;
      font-size: 0.78rem;
      min-height: 34px;
    }

    .toggle-chip {
      flex: 1 1 calc(50% - 6px);
      justify-content: center;
    }

    .control-row > button,
    .manage-grid > button {
      flex: 1 1 100%;
      justify-content: center;
    }

    .manage-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
