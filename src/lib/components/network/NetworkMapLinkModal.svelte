<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';

  export let show = false;
  export let editingLinkId: string | null = null;
  export let savingLink = false;
  export let linkPickDrawMode: 'quick' | 'path' = 'quick';
  export let linkSnapToNodeEnabled = true;
  export let linkPickMode = false;
  export let linkPickStep: 'from' | 'to' = 'from';
  export let linkPathBendPoints: Array<[number, number]> = [];
  export let linkForm: {
    name: string;
    link_type: string;
    status: string;
    from_node_id: string;
    to_node_id: string;
    priority: string;
    capacity_mbps: string;
    utilization_pct: string;
    loss_db: string;
    latency_ms: string;
    geometryText: string;
  };
  export let nodeRows: Array<{ id: string; name: string }> = [];
  export let linkTypeOptions: Array<{ label: string; value: string }> = [];
  export let linkStatusOptions: Array<{ label: string; value: string }> = [];
  export let linkFieldConfig: {
    capacityLabel: string;
    utilizationLabel: string;
    latencyLabel: string;
    lossLabel: string;
    showLoss: boolean;
    helper: string;
  };
  export let showManualEndpointSection = true;
  export let hasExistingLinkBetweenNodes: (
    fromNodeId: string,
    toNodeId: string,
    excludeLinkId?: string | null,
  ) => boolean;
  export let onClose: () => void;
  export let onSubmit: () => void;
  export let onTogglePickMode: () => void;
  export let onSetDrawMode: (mode: 'quick' | 'path') => void;
  export let onUndoPathPoint: () => void;
  export let onClearPathPoints: () => void;
  export let onUseStraightLine: () => void;
  export let onToggleSnap: () => void;
</script>

<Modal
  {show}
  title={editingLinkId ? ($t('network.map.edit_link') || 'Edit Link') : ($t('network.map.add_link') || 'Add Link')}
  width="920px"
  onclose={() => !savingLink && onClose()}
>
  {#if !editingLinkId}
    <div class="link-workflow">
      <div class="workflow-main">
        <div class="segmented-control" aria-label={$t('network.map.link_drawing_mode')}>
          <button
            class:active={linkPickDrawMode === 'quick'}
            type="button"
            onclick={() => onSetDrawMode('quick')}
          >
            {$t('network.map.quick')}
          </button>
          <button
            class:active={linkPickDrawMode === 'path'}
            type="button"
            onclick={() => onSetDrawMode('path')}
          >
            {$t('network.map.draw_path')}
          </button>
        </div>

        <button
          class="workflow-action primary"
          class:active={linkPickMode}
          type="button"
          onclick={onTogglePickMode}
        >
          <Icon name="map-pin" size={15} />
          <span>
            {linkPickMode
              ? ($t('network.map.picking') || `Picking ${linkPickStep === 'from' ? 'source' : 'destination'}`)
              : linkPickDrawMode === 'quick'
                ? ($t('network.map.pick_endpoints') || 'Pick endpoints')
                : ($t('network.map.draw_on_map') || 'Draw on map')}
          </span>
        </button>
      </div>

      {#if linkPickDrawMode === 'path' || linkPickMode}
        <div class="workflow-tools">
          {#if linkPickDrawMode === 'path'}
            <button
              class="workflow-action"
              class:active={linkSnapToNodeEnabled}
              type="button"
              onclick={onToggleSnap}
              title={$t('network.map.snap_bend_points')}
            >
              <Icon name="radio" size={14} />
              {$t('network.map.snap')} {linkSnapToNodeEnabled ? ($t('common.on') || 'On') : ($t('common.off') || 'Off')}
            </button>
          {/if}
          {#if linkPickMode && linkPickDrawMode === 'path'}
            <button
              class="workflow-action"
              type="button"
              onclick={onUndoPathPoint}
              disabled={linkPathBendPoints.length === 0}
            >
              <Icon name="arrow-left" size={14} />
              {$t('common.undo')}
            </button>
            <button
              class="workflow-action"
              type="button"
              onclick={onClearPathPoints}
              disabled={linkPathBendPoints.length === 0}
            >
              <Icon name="x-circle" size={14} />
              {$t('common.clear')}
            </button>
          {/if}
        </div>
      {/if}

      <div class="workflow-hint" class:active={linkPickMode}>
        {#if linkPickMode && linkPickDrawMode === 'quick'}
          {$t('network.map.link_hint_quick')}
        {:else if linkPickMode && linkPickStep === 'from'}
          {$t('network.map.link_hint_path_from')}
        {:else if linkPickMode}
          {$t('network.map.link_hint_path')}{linkSnapToNodeEnabled ? ($t('network.map.with_snap_enabled') || ' with snap enabled') : ''}, {$t('network.map.then_click_destination')}
        {:else}
          {$t('network.map.link_hint_choose')}
        {/if}
      </div>
    </div>
  {/if}

  <div class="link-form">
    <section class="form-section">
      <div class="section-head">
        <h4>{$t('network.map.identity')}</h4>
      </div>
      <div class="form-grid identity-grid">
        <label class="field span-5">
          <span>{$t('common.name')}</span>
          <input class="input" bind:value={linkForm.name} placeholder={$t('network.map.name_placeholder')} />
        </label>
        <label class="field span-2">
          <span>{$t('common.type')}</span>
          <Select2
            bind:value={linkForm.link_type}
            options={linkTypeOptions}
            width="100%"
            placeholder={$t('network.map.select_link_type')}
            searchPlaceholder={$t('common.search')}
            noResultsText={$t('common.not_found')}
          />
        </label>
        <label class="field span-2">
          <span>{$t('common.status')}</span>
          <Select2
            bind:value={linkForm.status}
            options={linkStatusOptions}
            width="100%"
            placeholder={$t('network.map.select_status')}
            searchPlaceholder={$t('common.search')}
            noResultsText={$t('common.not_found')}
          />
        </label>
        <label class="field span-3">
          <span>{$t('network.map.priority')}</span>
          <input class="input" type="number" min="1" bind:value={linkForm.priority} />
        </label>
      </div>
    </section>

    {#if showManualEndpointSection}
      <section class="form-section">
        <div class="section-head endpoints-head">
          <div>
            <h4>{$t('network.map.endpoints')}</h4>
          </div>
          <button class="inline-link-action" type="button" onclick={onUseStraightLine}>
            <Icon name="link" size={15} />
            {$t('network.map.use_straight_line')}
          </button>
        </div>
        <div class="form-grid endpoints-grid">
          <label class="field span-6">
            <span>{$t('network.map.from_node')}</span>
            <select class="input select-input" bind:value={linkForm.from_node_id}>
              <option value="">{$t('network.map.select_node')}</option>
              {#each nodeRows as n}
                <option value={n.id}>{n.name}</option>
              {/each}
            </select>
          </label>
          <label class="field span-6">
            <span>{$t('network.map.to_node')}</span>
            <select class="input select-input" bind:value={linkForm.to_node_id}>
              <option value="">{$t('network.map.select_node')}</option>
              {#each nodeRows as n}
                <option
                  value={n.id}
                  disabled={n.id === linkForm.from_node_id ||
                    hasExistingLinkBetweenNodes(linkForm.from_node_id, n.id, editingLinkId)}
                >
                  {n.name}
                </option>
              {/each}
            </select>
          </label>
        </div>
      </section>
    {/if}

    <section class="form-section">
      <div class="section-head with-helper">
        <div>
          <h4>{$t('network.map.link_metrics')}</h4>
        </div>
        <div class="helper-pill">
          <Icon name="info" size={14} />
          <span>{linkFieldConfig.helper}</span>
        </div>
      </div>
      <div class="form-grid metrics-grid">
        <label class="field">
          <span>{linkFieldConfig.capacityLabel}</span>
          <input
            class="input"
            type="number"
            min="0"
            step="0.01"
            bind:value={linkForm.capacity_mbps}
          />
        </label>
        <label class="field">
          <span>{linkFieldConfig.utilizationLabel}</span>
          <input
            class="input"
            type="number"
            min="0"
            max="100"
            step="0.01"
            bind:value={linkForm.utilization_pct}
          />
        </label>
        {#if linkFieldConfig.showLoss}
          <label class="field">
            <span>{linkFieldConfig.lossLabel}</span>
            <input class="input" type="number" step="0.01" bind:value={linkForm.loss_db} />
          </label>
        {/if}
        <label class="field">
          <span>{linkFieldConfig.latencyLabel}</span>
          <input class="input" type="number" min="0" step="0.01" bind:value={linkForm.latency_ms} />
        </label>
      </div>
    </section>
  </div>
  {#snippet footer()}
    <button class="modal-btn secondary" type="button" onclick={onClose} disabled={savingLink}>
      {$t('common.cancel')}
    </button>
    <button class="modal-btn primary" type="button" onclick={onSubmit} disabled={savingLink}>
      {savingLink ? ($t('common.saving') || 'Saving...') : ($t('common.save') || 'Save')}
    </button>
  {/snippet}
</Modal>

<style>
  .link-workflow {
    display: grid;
    gap: 10px;
    padding: 12px;
    margin-bottom: 16px;
    border: 1px solid color-mix(in srgb, var(--border-color) 82%, transparent);
    border-radius: 12px;
    background: var(--bg-surface);
  }

  .workflow-main,
  .workflow-tools,
  .section-head,
  .endpoints-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .segmented-control {
    display: inline-grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    min-width: 196px;
    padding: 3px;
    border: 1px solid color-mix(in srgb, var(--border-color) 88%, transparent);
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg-surface) 76%, #020617);
  }

  .segmented-control button,
  .workflow-action,
  .inline-link-action,
  .modal-btn {
    border: 1px solid transparent;
    font: inherit;
    cursor: pointer;
    transition:
      background 0.16s ease,
      border-color 0.16s ease,
      color 0.16s ease,
      box-shadow 0.16s ease;
  }

  .segmented-control button {
    min-height: 32px;
    border-radius: 8px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.82rem;
    font-weight: 800;
  }

  .segmented-control button.active {
    background: var(--color-primary);
    color: #fff;
    box-shadow: 0 8px 22px color-mix(in srgb, var(--color-primary) 28%, transparent);
  }

  .workflow-action {
    min-height: 34px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 0 11px;
    border-radius: 9px;
    background: color-mix(in srgb, var(--bg-surface) 78%, transparent);
    border-color: var(--border-color);
    color: var(--text-primary);
    font-size: 0.82rem;
    font-weight: 800;
    white-space: nowrap;
  }

  .workflow-action.primary {
    background: color-mix(in srgb, var(--color-primary) 13%, var(--bg-surface));
    border-color: color-mix(in srgb, var(--color-primary) 36%, var(--border-color));
  }

  .workflow-action.active {
    color: #fff;
    background: color-mix(in srgb, var(--color-primary) 80%, #0f172a);
    border-color: color-mix(in srgb, var(--color-primary) 82%, #fff);
  }

  .workflow-action:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }

  .workflow-hint {
    min-height: 32px;
    display: flex;
    align-items: center;
    padding: 7px 10px;
    border-radius: 9px;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-app) 72%, transparent);
    border: 1px dashed color-mix(in srgb, var(--border-color) 75%, transparent);
    font-size: 0.8rem;
    line-height: 1.35;
  }

  .workflow-hint.active {
    color: var(--text-primary);
    border-style: solid;
    border-color: color-mix(in srgb, var(--color-primary) 42%, var(--border-color));
    background: color-mix(in srgb, var(--color-primary) 10%, var(--bg-app));
  }

  .link-form {
    display: grid;
    gap: 12px;
  }

  .form-section {
    display: grid;
    gap: 10px;
    padding: 12px;
    border: 1px solid color-mix(in srgb, var(--border-color) 86%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface) 84%, #020617);
  }

  .section-head {
    align-items: flex-start;
  }

  .section-head h4 {
    margin: 0;
    color: var(--text-primary);
    font-size: 0.94rem;
    line-height: 1.2;
  }

  .section-head.with-helper {
    align-items: center;
  }

  .helper-pill {
    max-width: 460px;
    display: inline-flex;
    align-items: flex-start;
    gap: 7px;
    padding: 7px 9px;
    border-radius: 9px;
    background: color-mix(in srgb, var(--color-primary) 8%, var(--bg-app));
    border: 1px solid color-mix(in srgb, var(--color-primary) 22%, var(--border-color));
    color: var(--text-secondary);
    font-size: 0.77rem;
    line-height: 1.35;
  }

  .helper-pill :global(svg) {
    flex: 0 0 auto;
    margin-top: 1px;
    color: var(--color-primary);
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 10px;
    align-items: start;
  }

  .metrics-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .field {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .field span {
    color: var(--text-secondary);
    font-size: 0.77rem;
    font-weight: 800;
    line-height: 1.25;
  }

  .span-2 {
    grid-column: span 2;
  }

  .span-3 {
    grid-column: span 3;
  }

  .span-5 {
    grid-column: span 5;
  }

  .span-6 {
    grid-column: span 6;
  }

  .input {
    width: 100%;
    min-height: 38px;
    border: 1px solid var(--border-color);
    border-radius: 9px;
    background: color-mix(in srgb, var(--bg-app) 78%, #020617);
    color: var(--text-primary);
    padding: 8px 10px;
    font-size: 0.88rem;
    outline: none;
    transition:
      border-color 0.16s ease,
      box-shadow 0.16s ease,
      background 0.16s ease;
  }

  .input:focus {
    border-color: color-mix(in srgb, var(--color-primary) 58%, var(--border-color));
    background: color-mix(in srgb, var(--bg-app) 88%, #020617);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 18%, transparent);
  }

  .select-input {
    appearance: auto;
    padding-right: 32px;
  }

  .inline-link-action {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 32px;
    padding: 0 10px;
    border-radius: 9px;
    background: transparent;
    border-color: var(--border-color);
    color: var(--text-primary);
    font-size: 0.8rem;
    font-weight: 800;
  }

  .inline-link-action:hover,
  .workflow-action:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--color-primary) 42%, var(--border-color));
    background: color-mix(in srgb, var(--color-primary) 10%, var(--bg-surface));
  }

  .modal-btn {
    min-width: 96px;
    min-height: 40px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 16px;
    border-radius: 10px;
    font-size: 0.9rem;
    font-weight: 850;
  }

  .modal-btn.secondary {
    background: transparent;
    border-color: var(--border-color);
    color: var(--text-primary);
  }

  .modal-btn.primary {
    background: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 82%, #fff);
    color: #fff;
    box-shadow: 0 10px 24px color-mix(in srgb, var(--color-primary) 24%, transparent);
  }

  .modal-btn:hover:not(:disabled) {
    filter: brightness(1.06);
  }

  .modal-btn:disabled {
    cursor: not-allowed;
    opacity: 0.52;
  }

  :global(.select-trigger) {
    min-height: 40px;
  }

  @media (max-width: 920px) {
    .span-6,
    .span-5,
    .span-3,
    .span-2 {
      grid-column: span 12;
    }

    .metrics-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .helper-pill {
      max-width: none;
      width: 100%;
    }
  }

  @media (max-width: 640px) {
    .link-workflow,
    .form-section {
      padding: 12px;
      border-radius: 12px;
    }

    .workflow-main,
    .workflow-tools,
    .endpoints-head {
      align-items: stretch;
      flex-direction: column;
    }

    .segmented-control,
    .workflow-action,
    .inline-link-action {
      width: 100%;
    }

    .workflow-action,
    .inline-link-action {
      justify-content: center;
    }

    .metrics-grid,
    .form-grid {
      grid-template-columns: 1fr;
    }

    .span-6,
    .span-5,
    .span-3,
    .span-2 {
      grid-column: auto;
    }
  }
</style>
