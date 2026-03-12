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
  title={editingLinkId ? 'Edit Link' : 'Add Link'}
  width="860px"
  onclose={() => !savingLink && onClose()}
>
  {#if !editingLinkId}
    <div class="link-pick-toolbar">
      <div class="link-pick-mode">
        <button
          class={`mode-btn ${linkPickDrawMode === 'quick' ? 'active' : ''}`}
          type="button"
          onclick={() => onSetDrawMode('quick')}
        >
          Quick
        </button>
        <button
          class={`mode-btn ${linkPickDrawMode === 'path' ? 'active' : ''}`}
          type="button"
          onclick={() => onSetDrawMode('path')}
        >
          Draw Path
        </button>
      </div>
      {#if linkPickDrawMode === 'path'}
        <button
          class={`btn ghost btn-xs ${linkSnapToNodeEnabled ? 'active' : ''}`}
          type="button"
          onclick={onToggleSnap}
          title="Snap bend points to nearest node"
        >
          Snap: {linkSnapToNodeEnabled ? 'On' : 'Off'}
        </button>
      {/if}
      <button
        class={`btn ghost btn-xs ${linkPickMode ? 'active' : ''}`}
        type="button"
        onclick={onTogglePickMode}
      >
        <Icon name="map-pin" size={14} />
        {linkPickMode
          ? `Picking ${linkPickStep === 'from' ? 'source' : 'destination'}...`
          : linkPickDrawMode === 'quick'
            ? 'Pick Endpoints on Map'
            : 'Draw Path on Map'}
      </button>
      {#if linkPickMode && linkPickDrawMode === 'path'}
        <button class="btn ghost btn-xs" type="button" onclick={onUndoPathPoint} disabled={linkPathBendPoints.length === 0}>
          <Icon name="arrow-left" size={14} />
          Undo
        </button>
        <button class="btn ghost btn-xs" type="button" onclick={onClearPathPoints} disabled={linkPathBendPoints.length === 0}>
          <Icon name="x-circle" size={14} />
          Clear
        </button>
      {/if}
    </div>
    {#if linkPickMode}
      <div class="link-pick-hint">
        {#if linkPickDrawMode === 'quick'}
          Quick: click source node then destination node.
        {:else if linkPickStep === 'from'}
          Draw Path: click source node.
        {:else}
          Draw Path: click map to add bend points{linkSnapToNodeEnabled ? ' (auto-snap near node)' : ''}, then click destination node.
        {/if}
      </div>
    {/if}
  {/if}
  <div class="form-grid two-col">
    <label class="field">
      <span>Name</span>
      <input class="input" bind:value={linkForm.name} />
    </label>
    <label class="field">
      <span>Type</span>
      <Select2
        bind:value={linkForm.link_type}
        options={linkTypeOptions}
        width="100%"
        placeholder="Select link type"
        searchPlaceholder="Search type..."
        noResultsText="No type found"
      />
    </label>
    <label class="field">
      <span>Status</span>
      <Select2
        bind:value={linkForm.status}
        options={linkStatusOptions}
        width="100%"
        placeholder="Select status"
        searchPlaceholder="Search status..."
        noResultsText="No status found"
      />
    </label>
    <label class="field">
      <span>From Node</span>
      <select class="input" bind:value={linkForm.from_node_id}>
        <option value="">Select node</option>
        {#each nodeRows as n}
          <option value={n.id}>{n.name}</option>
        {/each}
      </select>
    </label>
    <label class="field">
      <span>To Node</span>
      <select class="input" bind:value={linkForm.to_node_id}>
        <option value="">Select node</option>
        {#each nodeRows as n}
          <option
            value={n.id}
            disabled={n.id === linkForm.from_node_id || hasExistingLinkBetweenNodes(linkForm.from_node_id, n.id, editingLinkId)}
          >
            {n.name}
          </option>
        {/each}
      </select>
    </label>
    <label class="field">
      <span>Priority</span>
      <input class="input" type="number" min="1" bind:value={linkForm.priority} />
    </label>
    <div class="field field-full link-type-helper">
      <Icon name="info" size={14} />
      <span>{linkFieldConfig.helper}</span>
    </div>
    <label class="field">
      <span>{linkFieldConfig.capacityLabel}</span>
      <input class="input" type="number" min="0" step="0.01" bind:value={linkForm.capacity_mbps} />
    </label>
    <label class="field">
      <span>{linkFieldConfig.utilizationLabel}</span>
      <input class="input" type="number" min="0" max="100" step="0.01" bind:value={linkForm.utilization_pct} />
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
    <label class="field field-full">
      <span>Geometry (GeoJSON LineString)</span>
      <textarea class="input textarea" rows="7" bind:value={linkForm.geometryText}></textarea>
    </label>
  </div>
  <div class="inline-actions">
    <button class="btn ghost btn-xs" type="button" onclick={onUseStraightLine}>Use straight line from selected nodes</button>
  </div>
  {#snippet footer()}
    <button class="btn ghost" type="button" onclick={onClose} disabled={savingLink}>Cancel</button>
    <button class="btn" type="button" onclick={onSubmit} disabled={savingLink}>
      {savingLink ? 'Saving...' : 'Save'}
    </button>
  {/snippet}
</Modal>
