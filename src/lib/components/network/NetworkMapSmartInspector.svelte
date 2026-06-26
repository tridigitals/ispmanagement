<script lang="ts">
  import type {
    NetworkMapInvestigationState,
    NetworkMapWorkspaceCapabilities,
    NetworkMapWorkspaceSelectedObject,
  } from '$lib/components/network/networkMapWorkspaceState';
  import Icon from '$lib/components/ui/Icon.svelte';

  export type NetworkMapInspectorSection = {
    title: string;
    lines: string[];
  };

  export type NetworkMapInspectorModel = {
    title: string;
    subtitle: string;
    tone: 'ok' | 'warn' | 'muted';
    sections: NetworkMapInspectorSection[];
  };

  let {
    collapsed,
    capabilities,
    selectedObject,
    investigationState,
    defaultModel,
    selectedModel,
    investigationModel,
    labels,
    onToggleCollapse,
    onClearSelection,
    onEnterServiceInvestigation,
    onEnterTraceInvestigation,
  }: {
    collapsed: boolean;
    capabilities: NetworkMapWorkspaceCapabilities;
    selectedObject: NetworkMapWorkspaceSelectedObject | null;
    investigationState: NetworkMapInvestigationState | null;
    defaultModel: NetworkMapInspectorModel;
    selectedModel: NetworkMapInspectorModel | null;
    investigationModel: NetworkMapInspectorModel | null;
    labels: {
      title: string;
      defaultKicker: string;
      selectedKicker: string;
      investigationKicker: string;
      collapse: string;
      expand: string;
      clear: string;
      serviceMode: string;
      traceMode: string;
      noSelection: string;
    };
    onToggleCollapse: () => void;
    onClearSelection: () => void;
    onEnterServiceInvestigation: () => void;
    onEnterTraceInvestigation: () => void;
  } = $props();

  const activeModel = $derived(
    investigationState ? investigationModel : selectedObject ? selectedModel : defaultModel,
  );
  const activeKicker = $derived(
    investigationState
      ? labels.investigationKicker
      : selectedObject
        ? labels.selectedKicker
        : labels.defaultKicker,
  );

  function toneClass(tone: NetworkMapInspectorModel['tone']) {
    if (tone === 'ok') return 'tone-ok';
    if (tone === 'warn') return 'tone-warn';
    return 'tone-muted';
  }
</script>

<aside class={`smart-inspector ${collapsed ? 'collapsed' : ''}`} aria-label={$t('network.map.smart_inspector')}>
  <div class="inspector-head">
    <div class="inspector-heading">
      <div class="inspector-kicker">{activeKicker}</div>
      {#if !collapsed}
        <div class="inspector-title">{labels.title}</div>
      {/if}
    </div>
    <button type="button" class="icon-btn" onclick={onToggleCollapse}>
      <Icon name={collapsed ? 'panel-right-open' : 'panel-right-close'} size={16} />
      <span class="sr-only">{collapsed ? labels.expand : labels.collapse}</span>
    </button>
  </div>

  {#if collapsed && activeModel}
    <div class="collapsed-summary">
      <div class={`collapsed-indicator ${toneClass(activeModel.tone)}`}></div>
      <div class="collapsed-labels">
        <div class="collapsed-title">{activeModel.title}</div>
        <div class="collapsed-subtitle">{activeModel.subtitle}</div>
      </div>
    </div>
  {/if}

  {#if !collapsed}
    <div class={`inspector-body ${toneClass(activeModel?.tone || 'muted')}`}>
      {#if activeModel}
        <div class="hero-card">
          <div class="hero-title">{activeModel.title}</div>
          <div class="hero-subtitle">{activeModel.subtitle}</div>
        </div>

        <div class="action-row">
          <button type="button" class="action-btn primary" onclick={onEnterServiceInvestigation}>
            <Icon name="users" size={14} />
            {labels.serviceMode}
          </button>
          <button type="button" class="action-btn" onclick={onEnterTraceInvestigation}>
            <Icon name="git-branch" size={14} />
            {labels.traceMode}
          </button>
          {#if selectedObject || investigationState}
            <button type="button" class="action-btn" onclick={onClearSelection}>
              <Icon name="x-circle" size={14} />
              {labels.clear}
            </button>
          {/if}
        </div>

        <div class="section-list">
          {#each activeModel.sections as section}
            <section class="info-section">
              <div class="section-title">{section.title}</div>
              <div class="section-lines">
                {#each section.lines as line}
                  <div class="section-line">{line}</div>
                {/each}
              </div>
            </section>
          {/each}
        </div>

        {#if !selectedObject && !investigationState}
          <div class="empty-note">{labels.noSelection}</div>
        {/if}
      {/if}
    </div>
  {/if}
</aside>

<style>
  .smart-inspector {
    height: 100%;
    border-left: 1px solid color-mix(in srgb, var(--border-color) 78%, transparent);
    background: var(--bg-surface);
    display: grid;
    grid-template-rows: auto 1fr;
    min-width: 0;
  }

  .smart-inspector.collapsed {
    width: 88px;
  }

  .inspector-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 14px 10px;
    border-bottom: 1px solid color-mix(in srgb, var(--border-color) 76%, transparent);
  }

  .inspector-kicker {
    font-size: 0.68rem;
    font-weight: 900;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-primary) 78%, white 22%);
  }

  .inspector-title {
    margin-top: 4px;
    font-size: 1rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color) 80%, transparent);
    background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
    color: var(--text-primary);
    cursor: pointer;
  }

  .inspector-body {
    display: grid;
    align-content: start;
    gap: 12px;
    padding: 14px;
    overflow: auto;
  }

  .collapsed-summary {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 14px 10px;
  }

  .collapsed-indicator {
    width: 12px;
    height: 12px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--border-color) 75%, transparent);
  }

  .collapsed-labels {
    display: grid;
    gap: 6px;
    justify-items: center;
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    min-height: 180px;
  }

  .collapsed-title {
    font-size: 0.72rem;
    font-weight: 900;
    color: var(--text-primary);
    line-height: 1.1;
  }

  .collapsed-subtitle {
    font-size: 0.68rem;
    color: var(--text-secondary);
    line-height: 1.2;
  }

  .hero-card {
    border-radius: var(--radius-lg);
    padding: 14px;
    border: 1px solid color-mix(in srgb, var(--border-color) 76%, transparent);
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
  }

  .hero-title {
    font-size: 1rem;
    font-weight: 900;
    color: var(--text-primary);
  }

  .hero-subtitle {
    margin-top: 6px;
    color: var(--text-secondary);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .action-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 38px;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color) 80%, transparent);
    padding: 0 12px;
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
    color: var(--text-primary);
    font-size: 0.84rem;
    font-weight: 800;
    cursor: pointer;
  }

  .action-btn.primary {
    border-color: color-mix(in srgb, var(--color-primary) 64%, var(--border-color));
    background: color-mix(in srgb, var(--color-primary) 18%, var(--bg-surface));
  }

  .section-list {
    display: grid;
    gap: 10px;
  }

  .info-section {
    border-radius: var(--radius-lg);
    border: 1px solid color-mix(in srgb, var(--border-color) 76%, transparent);
    padding: 12px;
    background: color-mix(in srgb, var(--bg-surface) 68%, transparent);
  }

  .section-title {
    font-size: 0.72rem;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .section-lines {
    margin-top: 8px;
    display: grid;
    gap: 6px;
  }

  .section-line {
    color: var(--text-primary);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .empty-note {
    color: var(--text-secondary);
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .tone-ok .hero-card {
    border-color: color-mix(in srgb, #10b981 42%, var(--border-color));
  }

  .tone-ok.collapsed-indicator,
  .tone-ok .collapsed-indicator {
    background: #10b981;
  }

  .tone-warn .hero-card {
    border-color: color-mix(in srgb, #f59e0b 42%, var(--border-color));
  }

  .tone-warn.collapsed-indicator,
  .tone-warn .collapsed-indicator {
    background: #f59e0b;
  }

  .tone-muted.collapsed-indicator,
  .tone-muted .collapsed-indicator {
    background: color-mix(in srgb, var(--border-color) 80%, transparent);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (max-width: 1180px) {
    .smart-inspector {
      min-height: 0;
    }

    .inspector-head {
      padding: 12px;
    }

    .inspector-body {
      padding: 12px;
      gap: 10px;
    }

    .action-row {
      display: grid;
      grid-template-columns: 1fr;
    }

    .action-btn {
      justify-content: center;
    }
  }
</style>
