<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';

  export type NetworkMapFloatingLabels = {
    title: string;
    subtitle: string;
    layers: string;
    view: string;
    tools: string;
    manage: string;
    standard: string;
    satellite: string;
    serviceMode: string;
    traceMode: string;
    clearMode: string;
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
    viewMode,
    nodesVisible,
    linksVisible,
    zonesVisible,
    routersVisible,
    customersVisible,
    canShowRouters,
    canManageTopology,
    activeInvestigationMode,
    onViewModeChange,
    onNodesVisibleChange,
    onLinksVisibleChange,
    onZonesVisibleChange,
    onRoutersVisibleChange,
    onCustomersVisibleChange,
    onEnterServiceMode,
    onEnterTraceMode,
    onClearMode,
    onOpenCreateNode,
    onOpenCreateLink,
    onOpenCreateZone,
  }: {
    labels: NetworkMapFloatingLabels;
    viewMode: 'standard' | 'satellite';
    nodesVisible: boolean;
    linksVisible: boolean;
    zonesVisible: boolean;
    routersVisible: boolean;
    customersVisible: boolean;
    canShowRouters: boolean;
    canManageTopology: boolean;
    activeInvestigationMode: 'service' | 'trace' | null;
    onViewModeChange: (mode: 'standard' | 'satellite') => void;
    onNodesVisibleChange: (checked: boolean) => void;
    onLinksVisibleChange: (checked: boolean) => void;
    onZonesVisibleChange: (checked: boolean) => void;
    onRoutersVisibleChange: (checked: boolean) => void;
    onCustomersVisibleChange: (checked: boolean) => void;
    onEnterServiceMode: () => void;
    onEnterTraceMode: () => void;
    onClearMode: () => void;
    onOpenCreateNode: () => void;
    onOpenCreateLink: () => void;
    onOpenCreateZone: () => void;
  } = $props();

  function buttonClass(active: boolean) {
    return active ? 'control-chip active' : 'control-chip';
  }
</script>

<aside class="floating-controls" aria-label="Map workspace controls">
  <div class="controls-head">
    <div class="controls-title">{labels.title}</div>
    <div class="controls-subtitle">{labels.subtitle}</div>
  </div>

  <section class="control-group">
    <div class="control-group-label">{labels.view}</div>
    <div class="control-row">
      <button
        type="button"
        class={buttonClass(viewMode === 'standard')}
        onclick={() => onViewModeChange('standard')}
      >
        <Icon name="map" size={14} />
        {labels.standard}
      </button>
      <button
        type="button"
        class={buttonClass(viewMode === 'satellite')}
        onclick={() => onViewModeChange('satellite')}
      >
        <Icon name="satellite" size={14} />
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
          onchange={(e) => onCustomersVisibleChange((e.currentTarget as HTMLInputElement).checked)}
        />
        <span>{labels.customers}</span>
      </label>
      {#if canShowRouters}
        <label class="toggle-chip">
          <input
            type="checkbox"
            checked={routersVisible}
            onchange={(e) => onRoutersVisibleChange((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>{labels.routers}</span>
        </label>
      {/if}
    </div>
  </section>

  <section class="control-group">
    <div class="control-group-label">{labels.tools}</div>
    <div class="control-row">
      <button
        type="button"
        class={buttonClass(activeInvestigationMode === 'service')}
        onclick={onEnterServiceMode}
      >
        <Icon name="users" size={14} />
        {labels.serviceMode}
      </button>
      <button
        type="button"
        class={buttonClass(activeInvestigationMode === 'trace')}
        onclick={onEnterTraceMode}
      >
        <Icon name="git-branch" size={14} />
        {labels.traceMode}
      </button>
      <button
        type="button"
        class={buttonClass(activeInvestigationMode === null)}
        onclick={onClearMode}
      >
        <Icon name="x-circle" size={14} />
        {labels.clearMode}
      </button>
    </div>
  </section>

  {#if canManageTopology}
    <section class="control-group">
      <div class="control-group-label">{labels.manage}</div>
      <div class="control-row">
        <button type="button" class="control-chip" onclick={onOpenCreateNode}>
          <Icon name="plus-circle" size={14} />
          {labels.addNode}
        </button>
        <button type="button" class="control-chip" onclick={onOpenCreateLink}>
          <Icon name="plus-circle" size={14} />
          {labels.addLink}
        </button>
        <button type="button" class="control-chip" onclick={onOpenCreateZone}>
          <Icon name="plus-circle" size={14} />
          {labels.addZone}
        </button>
      </div>
    </section>
  {/if}
</aside>

<style>
  .floating-controls {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 6;
    width: min(320px, calc(100vw - 32px));
    display: grid;
    gap: 12px;
    border-radius: 22px;
    border: 1px solid color-mix(in srgb, var(--border-color) 76%, transparent);
    padding: 14px;
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, rgba(5, 16, 27, 0.92) 70%, var(--bg-card)),
        rgba(5, 16, 27, 0.9)
      ),
      var(--bg-card);
    backdrop-filter: blur(14px);
    box-shadow: 0 24px 48px rgba(2, 6, 23, 0.28);
  }

  .controls-head {
    display: grid;
    gap: 4px;
  }

  .controls-title {
    font-size: 0.82rem;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-primary) 75%, white 25%);
  }

  .controls-subtitle {
    color: var(--text-secondary);
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .control-group {
    display: grid;
    gap: 8px;
  }

  .control-group-label {
    font-size: 0.72rem;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .control-row,
  .toggle-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .control-chip,
  .toggle-chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--border-color) 80%, transparent);
    padding: 9px 12px;
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
    color: var(--text-primary);
    font-size: 0.84rem;
    font-weight: 800;
    cursor: pointer;
  }

  .control-chip.active {
    border-color: color-mix(in srgb, var(--color-primary) 65%, var(--border-color));
    background: color-mix(in srgb, var(--color-primary) 18%, var(--bg-surface));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-primary) 25%, transparent);
  }

  .toggle-chip input {
    accent-color: var(--color-primary);
    margin: 0;
  }

  @media (max-width: 768px) {
    .floating-controls {
      left: 14px;
      right: 14px;
      width: auto;
    }
  }
</style>
